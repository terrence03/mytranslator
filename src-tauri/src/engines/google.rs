use super::{
    http_client, send_request, EngineContext, EngineError, TranslateRequest, TranslateResponse,
    TranslationEngine,
};
use async_trait::async_trait;
use serde_json::Value;

const ENDPOINT: &str = "https://translate.googleapis.com/translate_a/single";

/// Google 翻譯免費網頁端點（client=gtx），無需 API key。
/// 非官方端點，可能被限流（429），錯誤時提示使用者切換引擎。
pub struct GoogleFreeEngine;

#[async_trait]
impl TranslationEngine for GoogleFreeEngine {
    fn id(&self) -> &'static str {
        "google"
    }

    fn display_name(&self) -> &'static str {
        "Google 翻譯"
    }

    fn requires_key(&self) -> bool {
        false
    }

    async fn translate(
        &self,
        req: &TranslateRequest,
        _ctx: &EngineContext,
    ) -> Result<TranslateResponse, EngineError> {
        let resp = send_request(|| {
            http_client().get(ENDPOINT).query(&[
                ("client", "gtx"),
                ("sl", req.source.as_str()),
                ("tl", req.target.as_str()),
                ("dt", "t"),
                ("q", req.text.as_str()),
            ])
        })
        .await?;

        let status = resp.status();
        if status.as_u16() == 429 {
            return Err(EngineError::RateLimited);
        }
        if !status.is_success() {
            return Err(EngineError::BadResponse(format!("HTTP {}", status.as_u16())));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| EngineError::BadResponse(e.to_string()))?;
        parse_response(&body)
    }
}

/// 回應為巢狀陣列：[0] 是分段譯文列表（每段 [0] 為譯文），[2] 是偵測到的來源語言
fn parse_response(body: &Value) -> Result<TranslateResponse, EngineError> {
    let segments = body
        .get(0)
        .and_then(Value::as_array)
        .ok_or_else(|| EngineError::BadResponse("unexpected response shape".into()))?;

    let text: String = segments
        .iter()
        .filter_map(|seg| seg.get(0).and_then(Value::as_str))
        .collect();

    if text.is_empty() {
        return Err(EngineError::BadResponse("empty translation".into()));
    }

    let detected_source = body.get(2).and_then(Value::as_str).map(str::to_string);

    Ok(TranslateResponse {
        text,
        detected_source,
        engine: "google",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_multi_segment_response() {
        let body = json!([
            [
                ["你好。", "Hello.", null, null, 10],
                ["世界。", "World.", null, null, 10]
            ],
            null,
            "en"
        ]);
        let r = parse_response(&body).unwrap();
        assert_eq!(r.text, "你好。世界。");
        assert_eq!(r.detected_source.as_deref(), Some("en"));
    }

    #[test]
    fn rejects_unexpected_shape() {
        assert!(parse_response(&json!({"error": "nope"})).is_err());
        assert!(parse_response(&json!([[], null, "en"])).is_err());
    }
}
