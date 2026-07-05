pub mod gemini;
mod google;

pub use gemini::GeminiEngine;
pub use google::GoogleFreeEngine;

use async_trait::async_trait;
use serde::Serialize;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct TranslateRequest {
    pub text: String,
    /// 語言代碼，"auto" 表示自動偵測來源語言
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResponse {
    pub text: String,
    pub detected_source: Option<String>,
    pub engine: &'static str,
}

/// 由呼叫端（command 層）解析好再傳入，引擎本身不碰設定存取
#[derive(Debug, Clone, Default)]
pub struct EngineContext {
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("網路請求失敗：{0}")]
    Network(String),
    #[error("尚未設定 API key，請到設定頁填入")]
    MissingApiKey,
    #[error("API key 無效或無權限（HTTP {0}）")]
    InvalidApiKey(u16),
    #[error("請求過於頻繁，請稍後再試或切換引擎")]
    RateLimited,
    #[error("翻譯服務回應異常：{0}")]
    BadResponse(String),
}

impl From<reqwest::Error> for EngineError {
    fn from(e: reqwest::Error) -> Self {
        // reqwest 0.12 的 Display 不含底層原因（DNS 失敗、連線被拒、TLS 錯誤等），
        // 手動走訪 source 鏈把真正原因串進訊息，否則只會看到籠統的
        // "error sending request for url"
        use std::error::Error as _;
        let mut msg = e.to_string();
        let mut source = e.source();
        while let Some(s) = source {
            let part = s.to_string();
            if !msg.contains(&part) {
                msg.push('：');
                msg.push_str(&part);
            }
            source = s.source();
        }
        EngineError::Network(msg)
    }
}

#[async_trait]
pub trait TranslationEngine: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn requires_key(&self) -> bool;
    async fn translate(
        &self,
        req: &TranslateRequest,
        ctx: &EngineContext,
    ) -> Result<TranslateResponse, EngineError>;
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub requires_key: bool,
}

pub struct EngineRegistry {
    engines: Vec<Box<dyn TranslationEngine>>,
}

impl EngineRegistry {
    pub fn new() -> Self {
        Self {
            engines: vec![Box::new(GoogleFreeEngine), Box::new(GeminiEngine)],
        }
    }

    pub fn get(&self, id: &str) -> Option<&dyn TranslationEngine> {
        self.engines
            .iter()
            .find(|e| e.id() == id)
            .map(|e| e.as_ref())
    }

    pub fn list(&self) -> Vec<EngineInfo> {
        self.engines
            .iter()
            .map(|e| EngineInfo {
                id: e.id(),
                name: e.display_name(),
                requires_key: e.requires_key(),
            })
            .collect()
    }
}

pub(crate) fn http_client() -> &'static reqwest::Client {
    static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .connect_timeout(std::time::Duration::from_secs(10))
            // 常駐 app 在睡眠喚醒或切換網路後，連線池裡的 keep-alive 連線
            // 常已失效（預設閒置 90 秒才丟棄）；縮短存留時間減少拿到死連線
            .pool_idle_timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("failed to build http client")
    });
    &CLIENT
}

/// 送出請求；傳輸層錯誤（非逾時）時重試一次。
/// 拿到池中已死的連線會立刻失敗，重試會改建新連線。
/// 翻譯請求皆為冪等，重送安全；逾時不重試以免使用者久等。
pub(crate) async fn send_request(
    build: impl Fn() -> reqwest::RequestBuilder,
) -> Result<reqwest::Response, reqwest::Error> {
    match build().send().await {
        Err(e) if !e.is_timeout() => build().send().await,
        r => r,
    }
}

/// 給 LLM 引擎的語言顯示名稱；查不到就原樣回傳代碼
pub(crate) fn lang_display(code: &str) -> &str {
    match code {
        "zh-TW" => "Traditional Chinese (繁體中文)",
        "zh-CN" => "Simplified Chinese (简体中文)",
        "en" => "English",
        "ja" => "Japanese",
        "ko" => "Korean",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        other => other,
    }
}
