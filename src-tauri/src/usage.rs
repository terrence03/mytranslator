//! 本機用量記錄與上限檢查。
//!
//! Google 的實際用量只能透過 Cloud Monitoring / Billing API（需 OAuth）查詢，
//! 所以這裡在 app 端自行累計，作為估算：只涵蓋本機、本 app 送出的請求。
//! 週期與 Google 一致，以太平洋時間切日、切月。

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::settings::AppSettings;

const STORE_FILE: &str = "usage.json";
const STORE_KEY: &str = "usage";
/// 用量達上限的這個比例時開始提醒
const WARN_RATIO: f64 = 0.8;

/// 讀取—累加—寫回需互斥，避免連續快速翻譯時互相覆蓋
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EngineUsage {
    /// 目前計數所屬的月份（太平洋時間，"2026-11"）
    pub month: String,
    pub month_chars: u64,
    pub month_tokens: u64,
    /// 目前計數所屬的日期（太平洋時間，"2026-11-25"）
    pub day: String,
    pub day_requests: u64,
}

impl EngineUsage {
    /// 跨月／跨日時把對應的計數歸零
    fn roll(&mut self, p: &Period) {
        if self.month != p.month {
            self.month = p.month.clone();
            self.month_chars = 0;
            self.month_tokens = 0;
        }
        if self.day != p.day {
            self.day = p.day.clone();
            self.day_requests = 0;
        }
    }

    fn add(&mut self, chars: u64, tokens: Option<u64>) {
        self.month_chars += chars;
        self.month_tokens += tokens.unwrap_or(0);
        self.day_requests += 1;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Period {
    pub day: String,
    pub month: String,
}

impl Period {
    pub fn at(now: DateTime<Utc>) -> Self {
        let local = now.naive_utc() + Duration::hours(pacific_offset_hours(now));
        Self {
            day: local.format("%Y-%m-%d").to_string(),
            month: local.format("%Y-%m").to_string(),
        }
    }

    fn now() -> Self {
        Self::at(Utc::now())
    }
}

/// 太平洋時間相對 UTC 的時差（PST −8 / PDT −7）。
/// 美國夏令時間：三月第二個週日 02:00 PST（10:00 UTC）起，
/// 十一月第一個週日 02:00 PDT（09:00 UTC）止
fn pacific_offset_hours(now: DateTime<Utc>) -> i64 {
    let year = now.year();
    let start = nth_sunday(year, 3, 2).and_hms_opt(10, 0, 0).unwrap().and_utc();
    let end = nth_sunday(year, 11, 1).and_hms_opt(9, 0, 0).unwrap().and_utc();
    if now >= start && now < end {
        -7
    } else {
        -8
    }
}

fn nth_sunday(year: i32, month: u32, n: u8) -> NaiveDate {
    NaiveDate::from_weekday_of_month_opt(year, month, Weekday::Sun, n)
        .expect("every month has a 1st and 2nd Sunday")
}

/// 各引擎的上限種類，對應 Google 的計費／限流單位
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Limit {
    /// Cloud Translation 按字元計費，免費額度以月計
    MonthlyChars(u64),
    /// Gemini 免費層以每日請求數（RPD）限流
    DailyRequests(u64),
}

pub fn limit_for(engine_id: &str, s: &AppSettings) -> Option<Limit> {
    let limit = match engine_id {
        "google-cloud" => Limit::MonthlyChars(s.google_cloud_monthly_char_limit),
        "gemini" => Limit::DailyRequests(s.gemini_daily_request_limit),
        _ => return None,
    };
    match limit {
        Limit::MonthlyChars(0) | Limit::DailyRequests(0) => None,
        l => Some(l),
    }
}

/// 送出前檢查：這次請求會讓用量超過上限就擋下
pub fn check(u: &EngineUsage, limit: Limit, chars: u64, engine_name: &str) -> Result<(), String> {
    match limit {
        Limit::MonthlyChars(max) if u.month_chars + chars > max => Err(format!(
            "{engine_name} 已達本月用量上限（已用 {} / {} 字元，本次需 {} 字元）。\
             可到設定頁調整上限，或切換其他引擎。",
            fmt_num(u.month_chars),
            fmt_num(max),
            fmt_num(chars)
        )),
        Limit::DailyRequests(max) if u.day_requests >= max => Err(format!(
            "{engine_name} 已達今日請求上限（{} / {} 次）。\
             可到設定頁調整上限，或切換其他引擎。",
            fmt_num(u.day_requests),
            fmt_num(max)
        )),
        _ => Ok(()),
    }
}

/// 用量達上限 80% 以上時回傳提醒文字
pub fn warning(u: &EngineUsage, limit: Limit, engine_name: &str) -> Option<String> {
    let (used, max, unit, period) = match limit {
        Limit::MonthlyChars(max) => (u.month_chars, max, "字元", "本月"),
        Limit::DailyRequests(max) => (u.day_requests, max, "次", "今日"),
    };
    let ratio = used as f64 / max as f64;
    (ratio >= WARN_RATIO).then(|| {
        format!(
            "{engine_name} {period}已用 {:.0}%（{} / {} {unit}）",
            (ratio * 100.0).min(100.0),
            fmt_num(used),
            fmt_num(max)
        )
    })
}

/// 千分位格式：1234567 → "1,234,567"
fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

// ---- 讀寫 store ----

fn load_all(app: &AppHandle) -> HashMap<String, EngineUsage> {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(STORE_KEY))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn save_all(app: &AppHandle, all: &HashMap<String, EngineUsage>) {
    // 用量是估算用途，寫入失敗不影響翻譯本身
    if let Ok(store) = app.store(STORE_FILE) {
        store.set(STORE_KEY, json!(all));
        let _ = store.save();
    }
}

/// 取得某引擎目前週期的用量（已處理跨日／跨月歸零）
pub fn get(app: &AppHandle, engine_id: &str) -> EngineUsage {
    let mut u = load_all(app).remove(engine_id).unwrap_or_default();
    u.roll(&Period::now());
    u
}

pub fn check_limit(
    app: &AppHandle,
    engine_id: &str,
    engine_name: &str,
    s: &AppSettings,
    chars: u64,
) -> Result<(), String> {
    match limit_for(engine_id, s) {
        Some(limit) => check(&get(app, engine_id), limit, chars, engine_name),
        None => Ok(()),
    }
}

/// 記錄一次成功的請求，回傳接近上限的提醒（若有）
pub fn record(
    app: &AppHandle,
    engine_id: &str,
    engine_name: &str,
    s: &AppSettings,
    chars: u64,
    tokens: Option<u64>,
) -> Option<String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = load_all(app);
    let u = all.entry(engine_id.to_string()).or_default();
    u.roll(&Period::now());
    u.add(chars, tokens);
    let warn = limit_for(engine_id, s).and_then(|l| warning(u, l, engine_name));
    save_all(app, &all);
    warn
}

pub fn reset(app: &AppHandle, engine_id: &str) {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = load_all(app);
    all.remove(engine_id);
    save_all(app, &all);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn pacific_period_uses_pst_in_winter() {
        // 2026-12-01 07:59 UTC = 2026-11-30 23:59 PST
        let p = Period::at(utc(2026, 12, 1, 7, 59));
        assert_eq!(p.day, "2026-11-30");
        assert_eq!(p.month, "2026-11");
        let p = Period::at(utc(2026, 12, 1, 8, 0));
        assert_eq!(p.day, "2026-12-01");
        assert_eq!(p.month, "2026-12");
    }

    #[test]
    fn pacific_period_uses_pdt_in_summer() {
        // 2026-07-01 06:59 UTC = 2026-06-30 23:59 PDT
        assert_eq!(Period::at(utc(2026, 7, 1, 6, 59)).day, "2026-06-30");
        assert_eq!(Period::at(utc(2026, 7, 1, 7, 0)).day, "2026-07-01");
    }

    #[test]
    fn dst_boundaries_2026() {
        // 2026 夏令時間：3/8 10:00 UTC 起，11/1 09:00 UTC 止
        assert_eq!(pacific_offset_hours(utc(2026, 3, 8, 9, 59)), -8);
        assert_eq!(pacific_offset_hours(utc(2026, 3, 8, 10, 0)), -7);
        assert_eq!(pacific_offset_hours(utc(2026, 11, 1, 8, 59)), -7);
        assert_eq!(pacific_offset_hours(utc(2026, 11, 1, 9, 0)), -8);
    }

    #[test]
    fn roll_resets_counters_on_new_period() {
        let mut u = EngineUsage {
            month: "2026-11".into(),
            month_chars: 100,
            month_tokens: 50,
            day: "2026-11-30".into(),
            day_requests: 7,
        };
        // 同月不同日：只歸零每日請求
        u.roll(&Period { day: "2026-11-29".into(), month: "2026-11".into() });
        assert_eq!((u.month_chars, u.month_tokens, u.day_requests), (100, 50, 0));
        u.add(10, Some(5));
        // 跨月：全部歸零
        u.roll(&Period { day: "2026-12-01".into(), month: "2026-12".into() });
        assert_eq!((u.month_chars, u.month_tokens, u.day_requests), (0, 0, 0));
        assert_eq!(u.month, "2026-12");
    }

    #[test]
    fn monthly_char_limit_blocks_request_that_would_exceed() {
        let u = EngineUsage { month_chars: 499_000, ..Default::default() };
        let limit = Limit::MonthlyChars(500_000);
        assert!(check(&u, limit, 1_000, "X").is_ok());
        let err = check(&u, limit, 1_001, "X").unwrap_err();
        assert!(err.contains("499,000 / 500,000"), "{err}");
    }

    #[test]
    fn daily_request_limit_blocks_at_limit() {
        let limit = Limit::DailyRequests(500);
        let u = EngineUsage { day_requests: 499, ..Default::default() };
        assert!(check(&u, limit, 10_000, "X").is_ok());
        let u = EngineUsage { day_requests: 500, ..Default::default() };
        assert!(check(&u, limit, 1, "X").is_err());
    }

    #[test]
    fn warns_from_80_percent() {
        let limit = Limit::MonthlyChars(1_000);
        let u = EngineUsage { month_chars: 799, ..Default::default() };
        assert!(warning(&u, limit, "X").is_none());
        let u = EngineUsage { month_chars: 800, ..Default::default() };
        assert_eq!(
            warning(&u, limit, "X").as_deref(),
            Some("X 本月已用 80%（800 / 1,000 字元）")
        );
    }

    #[test]
    fn zero_limit_means_unlimited() {
        let s = AppSettings {
            google_cloud_monthly_char_limit: 0,
            gemini_daily_request_limit: 0,
            ..Default::default()
        };
        assert!(limit_for("google-cloud", &s).is_none());
        assert!(limit_for("gemini", &s).is_none());
        assert!(limit_for("unknown", &AppSettings::default()).is_none());
    }

    #[test]
    fn formats_thousands() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(999), "999");
        assert_eq!(fmt_num(1_000), "1,000");
        assert_eq!(fmt_num(1_234_567), "1,234,567");
    }
}
