use super::{
    http_client, lang_display, EngineContext, EngineError, TranslateRequest, TranslateResponse,
    TranslationEngine,
};
use async_trait::async_trait;
use serde_json::{json, Value};

pub const DEFAULT_MODEL: &str = "gemma-4-31b-it";

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

        let resp = http_client()
            .post(&url)
            .header("x-goog-api-key", key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status().as_u16();
        match status {
            400 | 401 | 403 => return Err(EngineError::InvalidApiKey(status)),
            429 => return Err(EngineError::RateLimited),
            s if !(200..300).contains(&s) => {
                return Err(EngineError::BadResponse(format!("HTTP {s}")))
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

    let text: String = parts
        .iter()
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
    fn surfaces_api_error_message() {
        let body = json!({ "error": { "code": 404, "message": "model not found" } });
        let err = parse_response(&body).unwrap_err();
        assert!(err.to_string().contains("model not found"));
    }
}
