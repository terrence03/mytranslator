use super::{
    http_client, lang_display, send_request, EngineContext, EngineError, TranslateRequest,
    TranslateResponse, TranslationEngine,
};
use async_trait::async_trait;
use serde_json::{json, Value};

pub const DEFAULT_MODEL: &str = "gemini-3.1-flash-lite";

const SYSTEM_PROMPT: &str = "You are a translation engine. Translate the text given by the user \
into the requested target language. Output ONLY the translation with the original formatting \
preserved. Do not add explanations, notes, or quotation marks.";

/// Gemini AI 翻譯，使用者自備 API key（generativelanguage.googleapis.com）
pub struct GeminiEngine;

#[async_trait]
impl TranslationEngine for GeminiEngine {
    fn id(&self) -> &'static str {
        "gemini"
    }

    fn display_name(&self) -> &'static str {
        "Gemini AI"
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
        let model = ctx.model.as_deref().unwrap_or(DEFAULT_MODEL);

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
        );
        let user_text = format!(
            "Translate into {}:\n\n{}",
            lang_display(&req.target),
            req.text
        );
        // Gemma 系列不支援 system_instruction，改併入 user 訊息
        let mut body = json!({
            "contents": [{
                "role": "user",
                "parts": [{ "text": if model.starts_with("gemma") {
                    format!("{SYSTEM_PROMPT}\n\n{user_text}")
                } else {
                    user_text
                }}]
            }],
            "generationConfig": { "temperature": 0.2 }
        });
        if !model.starts_with("gemma") {
            body["system_instruction"] = json!({ "parts": [{ "text": SYSTEM_PROMPT }] });
        }

        let make_req = || {
            http_client()
                .post(&url)
                .header("x-goog-api-key", key)
                .json(&body)
        };
        let mut resp = send_request(make_req).await?;
        // Gemini API 偶發 500 INTERNAL 暫時性錯誤，重試一次
        if resp.status().as_u16() >= 500 {
            resp = send_request(make_req).await?;
        }

        let status = resp.status().as_u16();
        match status {
            401 | 403 => return Err(EngineError::InvalidApiKey(status)),
            429 => return Err(EngineError::RateLimited),
            s if !(200..300).contains(&s) => {
                let detail = resp
                    .json::<Value>()
                    .await
                    .ok()
                    .and_then(|b| {
                        b.pointer("/error/message")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                // 400 多半是 key 無效，但也可能是請求參數問題，靠訊息內容區分
                if s == 400 && detail.contains("API key") {
                    return Err(EngineError::InvalidApiKey(s));
                }
                return Err(EngineError::BadResponse(if detail.is_empty() {
                    format!("HTTP {s}")
                } else {
                    format!("HTTP {s}：{detail}")
                }));
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

fn parse_response(body: &Value) -> Result<TranslateResponse, EngineError> {
    let parts = body
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            let detail = body
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("unexpected response shape");
            EngineError::BadResponse(detail.to_string())
        })?;

    // 推理模型（如 gemma-4）會附上 "thought": true 的思考過程，不屬於譯文
    let text: String = parts
        .iter()
        .filter(|p| p.get("thought").and_then(Value::as_bool) != Some(true))
        .filter_map(|p| p.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");

    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(EngineError::BadResponse("empty translation".into()));
    }

    Ok(TranslateResponse {
        text,
        detected_source: None,
        engine: "gemini",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_generate_content_response() {
        let body = json!({
            "candidates": [{
                "content": { "parts": [{ "text": "你好，世界\n" }], "role": "model" },
                "finishReason": "STOP"
            }]
        });
        let r = parse_response(&body).unwrap();
        assert_eq!(r.text, "你好，世界");
        assert!(r.detected_source.is_none());
    }

    #[test]
    fn skips_thought_parts() {
        let body = json!({
            "candidates": [{
                "content": {
                    "parts": [
                        { "text": "Let me think about this translation...", "thought": true },
                        { "text": "你好" }
                    ],
                    "role": "model"
                },
                "finishReason": "STOP"
            }]
        });
        let r = parse_response(&body).unwrap();
        assert_eq!(r.text, "你好");
    }

    #[test]
    fn surfaces_api_error_message() {
        let body = json!({ "error": { "code": 404, "message": "model not found" } });
        let err = parse_response(&body).unwrap_err();
        assert!(err.to_string().contains("model not found"));
    }

    #[test]
    #[ignore = "needs network"]
    fn live_request_reaches_google() {
        let engine = GeminiEngine;
        let ctx = EngineContext {
            api_key: Some("dummy-key-for-transport-test".into()),
            model: Some(DEFAULT_MODEL.into()),
        };
        let req = TranslateRequest {
            text: "hi".into(),
            source: "auto".into(),
            target: "zh-TW".into(),
        };
        let err = tauri::async_runtime::block_on(engine.translate(&req, &ctx)).unwrap_err();
        // dummy key 應該打到 Google 並收到 400/401/403，而不是網路層錯誤
        eprintln!("live error: {err}");
        assert!(matches!(err, EngineError::InvalidApiKey(_)), "got: {err}");
    }
}
