use serde::Serialize;

use crate::engines::http_client;

const REPO: &str = "terrence03/mytranslator";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub has_update: bool,
    pub url: String,
}

/// 查詢 GitHub 最新 release，與目前版本比較。
/// 不做自動下載安裝，只回報是否有新版與下載頁連結。
pub async fn check(current: &str) -> Result<UpdateInfo, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = http_client()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "mytranslator")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let tag = body
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .ok_or("release 回應缺少 tag_name")?;
    let html_url = body
        .get("html_url")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&url)
        .to_string();
    let latest = tag.strip_prefix('v').unwrap_or(tag).to_string();

    Ok(UpdateInfo {
        has_update: is_newer(&latest, current),
        current: current.to_string(),
        latest,
        url: html_url,
    })
}

/// 逐段比較 "1.2.3" 形式的版本號，段數不同時缺的段視為 0
fn is_newer(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.').map(|p| p.parse().unwrap_or(0)).collect()
    };
    let (a, b) = (parse(a), parse(b));
    let len = a.len().max(b.len());
    for i in 0..len {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.2.0", "0.1.1"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
        assert!(is_newer("0.1.1.1", "0.1.1"));
    }
}
