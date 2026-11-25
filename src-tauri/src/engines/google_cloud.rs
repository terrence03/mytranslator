use super::{
    api_error_message, http_client, send_request, EngineContext, EngineError, TranslateRequest,
    TranslateResponse, TranslationEngine,
};
use async_trait::async_trait;
use serde_json::{json, Value};

const ENDPOINT: &str = "https://translation.googleapis.com/language/translate/v2";

/// Google Cloud Translation API（Basic / v2），使用者自備 API key。
/// 官方付費 API，每月前 50 萬字元免費；需在綁定帳單的 GCP 專案啟用 Cloud Translation API。
pub struct GoogleCloudEngine;

#[async_trait]
impl TranslationEngine for GoogleCloudEngine {
    fn id(&self) -> &'static str {
        "google-cloud"
    }

    fn display_name(&self) -> &'static str {
        "Google Cloud 翻譯"
    }

    fn requires_key(&self) -> bool {
        true
    }

    async fn translate(
        &self,
        req: &TranslateRequest,
        ctx: &EngineContext,
    ) -> Result<TranslateResponse, EngineError> {
        let key = ctx
            .api_key
            .as_deref()
            .filter(|k| !k.is_empty())
            .ok_or(EngineError::MissingApiKey)?;

        // format=text：不做 HTML 跳脫，否則 ' & < 等字元會變成 &#39; 之類的實體
        let mut body = json!({
            "q": req.text,
            "target": req.target,
            "format": "text",
        });
        if req.source != "auto" {
            body["source"] = json!(req.source);
        }

        let make_req = || {
            http_client()
                .post(ENDPOINT)
                .header("x-goog-api-key", key)
                .json(&body)
        };
        let mut resp = send_request(make_req).await?;
        if resp.status().as_u16() >= 500 {
            resp = send_request(make_req).await?;
        }

        let status = resp.status().as_u16();
        match status {
            429 => return Err(EngineError::RateLimited),
            s if !(200..300).contains(&s) => {
                let detail = api_error_message(resp).await;
                return Err(map_error(s, &detail));
            }
            _ => {}
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| EngineError::BadResponse(e.to_string()))?;
        parse_response(&body)
    }
}

fn map_error(status: u16, detail: &str) -> EngineError {
    // key 打錯回 400 "API key not valid"；403 多半是專案沒啟用 API 或沒綁帳單，
    // 這時 Google 的訊息會說明要去哪裡開，比籠統的「key 無效」有用
    if status == 401 || (status == 400 && detail.contains("API key")) {
        return EngineError::InvalidApiKey(status);
    }
    if detail.is_empty() {
        if status == 403 {
            return EngineError::InvalidApiKey(status);
        }
        return EngineError::BadResponse(format!("HTTP {status}"));
    }
    EngineError::BadResponse(format!("HTTP {status}：{detail}"))
}

/// 回應：{"data":{"translations":[{"translatedText":"…","detectedSourceLanguage":"en"}]}}
fn parse_response(body: &Value) -> Result<TranslateResponse, EngineError> {
    let first = body
        .pointer("/data/translations/0")
        .ok_or_else(|| EngineError::BadResponse("unexpected response shape".into()))?;

    let text = first
        .get("translatedText")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if text.is_empty() {
        return Err(EngineError::BadResponse("empty translation".into()));
    }

    let detected_source = first
        .get("detectedSourceLanguage")
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(TranslateResponse {
        text,
        detected_source,
        engine: "google-cloud",
        tokens: None,
        usage_warning: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_translation_with_detected_source() {
        let body = json!({
            "data": { "translations": [
                { "translatedText": "你好，世界", "detectedSourceLanguage": "en" }
            ]}
        });
        let r = parse_response(&body).unwrap();
        assert_eq!(r.text, "你好，世界");
        assert_eq!(r.detected_source.as_deref(), Some("en"));
    }

    #[test]
    fn parses_translation_without_detected_source() {
        let body = json!({ "data": { "translations": [{ "translatedText": "Hello" }] } });
        let r = parse_response(&body).unwrap();
        assert_eq!(r.text, "Hello");
        assert!(r.detected_source.is_none());
    }

    #[test]
    fn rejects_unexpected_shape() {
        assert!(parse_response(&json!({ "data": { "translations": [] } })).is_err());
        assert!(parse_response(&json!({ "data": { "translations": [{ "translatedText": "" }] } })).is_err());
    }

    /// GOOGLE_CLOUD_TRANSLATE_KEY=… cargo test live_translate -- --ignored --nocapture
    #[test]
    #[ignore = "needs network and GOOGLE_CLOUD_TRANSLATE_KEY"]
    fn live_translate() {
        let key = std::env::var("GOOGLE_CLOUD_TRANSLATE_KEY").expect("set GOOGLE_CLOUD_TRANSLATE_KEY");
        let ctx = EngineContext { api_key: Some(key), model: None };
        let req = TranslateRequest {
            text: "It's a test & <demo>.".into(),
            source: "auto".into(),
            target: "zh-TW".into(),
        };
        let r = tauri::async_runtime::block_on(GoogleCloudEngine.translate(&req, &ctx)).unwrap();
        eprintln!("live result: {r:?}");
        assert_eq!(r.detected_source.as_deref(), Some("en"));
        assert!(!r.text.contains("&#39;") && !r.text.contains("&amp;"));

        let bad = EngineContext { api_key: Some("AIzaInvalidKey".into()), model: None };
        let err = tauri::async_runtime::block_on(GoogleCloudEngine.translate(&req, &bad)).unwrap_err();
        eprintln!("bad key error: {err}");
        assert!(matches!(err, EngineError::InvalidApiKey(_)), "got: {err}");
    }

    #[test]
    fn maps_errors() {
        assert!(matches!(
            map_error(400, "API key not valid. Please pass a valid API key."),
            EngineError::InvalidApiKey(400)
        ));
        let e = map_error(403, "Cloud Translation API has not been used in project 123 before or it is disabled.");
        assert!(e.to_string().contains("has not been used"));
        assert!(matches!(map_error(403, ""), EngineError::InvalidApiKey(403)));
        assert!(e.to_string().contains("403"));
    }
}
