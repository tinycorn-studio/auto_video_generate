use reqwest::blocking::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

use super::{checked, network_error, MODEL};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Provider {
    Google,
    Xpiki,
}

impl Provider {
    pub(super) fn detect(key: &str) -> Result<Self, String> {
        validate_key_format(key)?;
        if (key.starts_with("AIza") && key.len() > 4) || (key.starts_with("AQ.") && key.len() > 3) {
            Ok(Self::Google)
        } else if key.starts_with("sk-") && key.len() > 3 {
            Ok(Self::Xpiki)
        } else {
            Err(
                "Chưa nhận diện được key. Hãy sao chép API key từ Google AI Studio hoặc XPiki."
                    .into(),
            )
        }
    }
}

fn validate_key_format(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > 512 || !key.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err("API key Gemini không đúng định dạng.".into());
    }
    Ok(())
}

// One atomic credential-store value keeps the verified provider bound to its secret.
// Never derive Debug or return this structure through Tauri commands.
#[derive(Serialize, Deserialize)]
pub(super) struct Connection {
    version: u8,
    pub(super) provider: Provider,
    key: String,
}

impl Connection {
    pub(super) fn new(key: String) -> Result<Self, String> {
        let provider = Provider::detect(&key)?;
        Ok(Self {
            version: 1,
            provider,
            key,
        })
    }

    pub(super) fn decode(stored: &str) -> Result<Self, String> {
        if !stored.starts_with('{') {
            // Before dual-provider support this entry held a verified XPiki key only.
            validate_key_format(stored)?;
            return Ok(Self {
                version: 1,
                provider: Provider::Xpiki,
                key: stored.into(),
            });
        }
        let connection: Self = serde_json::from_str(stored).map_err(|_| {
            "Không đọc được kết nối Gemini đã lưu. Hãy lưu lại API key.".to_string()
        })?;
        if connection.version != 1 || Provider::detect(&connection.key)? != connection.provider {
            return Err("Kết nối Gemini đã lưu không hợp lệ. Hãy lưu lại API key.".into());
        }
        Ok(connection)
    }

    pub(super) fn encode(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|_| "Không chuẩn bị được kết nối Gemini.".into())
    }
}

pub(super) fn validation_request(client: &Client, connection: &Connection) -> RequestBuilder {
    match connection.provider {
        Provider::Google => client
            .get(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{MODEL}"
            ))
            .header("x-goog-api-key", &connection.key),
        Provider::Xpiki => client
            .get("https://api.xpiki.com/v1/models?type=all")
            .bearer_auth(&connection.key),
    }
    .timeout(Duration::from_secs(30))
}

pub(super) fn validate_model(provider: Provider, value: &Value) -> Result<(), String> {
    match provider {
        Provider::Google => {
            if value["name"].as_str() != Some(format!("models/{MODEL}").as_str())
                || !value["supportedGenerationMethods"]
                    .as_array()
                    .is_some_and(|methods| {
                        methods
                            .iter()
                            .any(|method| method.as_str() == Some("generateContent"))
                    })
            {
                return Err(
                    "Key Google Gemini chưa truy cập được model tạo nội dung đã cấu hình.".into(),
                );
            }
        }
        Provider::Xpiki => {
            let model = value["data"]
                .as_array()
                .and_then(|list| {
                    list.iter()
                        .find(|model| model["id"].as_str() == Some(MODEL))
                })
                .ok_or("Key XPiki không có quyền truy cập gemini-3.8-flash.")?;
            if !super::supports_publishing(model) {
                return Err("Model XPiki không hỗ trợ chat văn bản với đầu ra JSON.".into());
            }
        }
    }
    Ok(())
}

pub(super) fn verify(client: &Client, connection: &Connection) -> Result<(), String> {
    let metadata: Value = checked(
        validation_request(client, connection)
            .send()
            .map_err(network_error)?,
    )?
    .json()
    .map_err(|_| "Dịch vụ Gemini trả về thông tin model không hợp lệ.".to_string())?;
    validate_model(connection.provider, &metadata)
}

fn google_body(chat: &Value) -> Value {
    json!({
        "systemInstruction": {"parts": [{"text": chat["messages"][0]["content"]}]},
        "contents": [{"role": "user", "parts": [{"text": chat["messages"][1]["content"]}]}],
        "generationConfig": {
            "responseMimeType": "application/json",
            "maxOutputTokens": chat["max_tokens"],
            "thinkingConfig": {"thinkingLevel": "low"},
        }
    })
}

pub(super) fn generation_request(
    client: &Client,
    connection: &Connection,
    chat: &Value,
) -> RequestBuilder {
    match connection.provider {
        Provider::Google => client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent"
            ))
            .header("x-goog-api-key", &connection.key)
            .json(&google_body(chat)),
        Provider::Xpiki => client
            .post("https://api.xpiki.com/v1/chat/completions")
            .bearer_auth(&connection.key)
            .json(chat),
    }
}

pub(super) fn normalize_response(provider: Provider, value: Value) -> Result<Value, String> {
    if provider == Provider::Xpiki {
        return Ok(value);
    }
    let candidate = value["candidates"]
        .as_array()
        .and_then(|candidates| candidates.first())
        .ok_or("Gemini không trả về nội dung. Kiểm tra nguồn văn bản rồi thử lại.")?;
    if candidate["finishReason"].as_str() != Some("STOP") {
        return Err("Gemini chưa tạo được nội dung hoàn chỉnh. Hãy thử lại.".into());
    }
    let parts = candidate["content"]["parts"]
        .as_array()
        .ok_or("Gemini trả về nội dung không hợp lệ.")?;
    let text = parts
        .iter()
        .filter(|part| part["thought"] != true)
        .filter_map(|part| part["text"].as_str())
        .collect::<String>();
    if text.trim().is_empty() {
        return Err("Gemini không trả về văn bản hợp lệ.".into());
    }
    Ok(json!({"choices":[{"finish_reason":"stop","message":{"content":text}}]}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_supported_key_families_and_rejects_ambiguous_input() {
        assert!(matches!(
            Provider::detect("AIza-fixture"),
            Ok(Provider::Google)
        ));
        assert!(matches!(
            Provider::detect("AQ.fixture"),
            Ok(Provider::Google)
        ));
        assert!(matches!(
            Provider::detect("sk-fixture"),
            Ok(Provider::Xpiki)
        ));
        for key in [
            "",
            "AIza",
            "AQ.",
            "sk-",
            "opaque-fixture",
            "sk-with space",
            "sk-\nfixture",
        ] {
            assert!(Provider::detect(key).is_err());
        }
        assert!(Provider::detect(&"x".repeat(513)).is_err());
    }

    #[test]
    fn restores_legacy_keys_and_roundtrips_verified_connections() {
        assert!(matches!(
            Connection::decode("legacy-xpiki-fixture").unwrap().provider,
            Provider::Xpiki
        ));
        for key in ["sk-fixture", "AIza-fixture", "AQ.fixture"] {
            let original = Connection::new(key.into()).unwrap();
            let restored = Connection::decode(&original.encode().unwrap()).unwrap();
            assert!(original.provider == restored.provider);
            assert!(original.key == restored.key);
        }
        assert!(
            Connection::decode(r#"{"version":1,"provider":"xpiki","key":"AIza-fixture"}"#).is_err()
        );
        assert!(
            Connection::decode(r#"{"version":2,"provider":"google","key":"AIza-fixture"}"#)
                .is_err()
        );
        assert!(Connection::decode("{broken").is_err());
    }

    #[test]
    fn keeps_credentials_on_the_selected_provider_and_out_of_urls() {
        let client = Client::new();
        let chat = json!({"messages":[{"content":"system fixture"},{"content":"source fixture"}],"max_tokens":8192});
        for key in ["AIza-fixture", "AQ.fixture", "sk-fixture"] {
            let connection = Connection::new(key.into()).unwrap();
            for builder in [
                validation_request(&client, &connection),
                generation_request(&client, &connection, &chat),
            ] {
                let request = builder.build().unwrap();
                assert!(!request.url().as_str().contains(key));
                if connection.provider == Provider::Google {
                    assert_eq!(
                        request.url().host_str(),
                        Some("generativelanguage.googleapis.com")
                    );
                    assert_eq!(request.headers()["x-goog-api-key"].to_str().unwrap(), key);
                    assert!(!request.headers().contains_key("authorization"));
                } else {
                    assert_eq!(request.url().host_str(), Some("api.xpiki.com"));
                    assert_eq!(
                        request.headers()["authorization"].to_str().unwrap(),
                        format!("Bearer {key}")
                    );
                    assert!(!request.headers().contains_key("x-goog-api-key"));
                }
            }
        }
    }

    #[test]
    fn checks_provider_specific_metadata_and_preserves_the_shared_prompt() {
        let model = json!({"name":format!("models/{MODEL}"),"supportedGenerationMethods":["generateContent"]});
        assert!(validate_model(Provider::Google, &model).is_ok());
        assert!(validate_model(
            Provider::Google,
            &json!({"name":"models/other","supportedGenerationMethods":["generateContent"]})
        )
        .is_err());
        assert!(
            validate_model(Provider::Google, &json!({"name":format!("models/{MODEL}")})).is_err()
        );
        let chat = json!({"messages":[{"content":"system fixture"},{"content":"source fixture"}],"max_tokens":8192});
        let native = google_body(&chat);
        assert_eq!(
            native["systemInstruction"]["parts"][0]["text"],
            "system fixture"
        );
        assert_eq!(native["contents"][0]["parts"][0]["text"], "source fixture");
        assert_eq!(
            native["generationConfig"]["responseMimeType"],
            "application/json"
        );
    }

    #[test]
    fn reads_google_final_text_without_thoughts_and_rejects_partial_or_blocked_output() {
        let response = json!({"candidates":[{"finishReason":"STOP","content":{"parts":[
            {"thought":true,"text":"private reasoning"}, {"text":"{\"posts\":"}, {"text":"[]}"}
        ]}}]});
        let normalized = normalize_response(Provider::Google, response.clone()).unwrap();
        assert_eq!(
            normalized["choices"][0]["message"]["content"],
            "{\"posts\":[]}"
        );
        for reason in ["MAX_TOKENS", "SAFETY", "RECITATION"] {
            let mut partial = response.clone();
            partial["candidates"][0]["finishReason"] = json!(reason);
            assert!(normalize_response(Provider::Google, partial).is_err());
        }
        assert!(normalize_response(
            Provider::Google,
            json!({"promptFeedback":{"blockReason":"SAFETY"}})
        )
        .is_err());
        assert!(normalize_response(Provider::Google, json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"thought":true,"text":"reasoning only"}]}}]})).is_err());
    }
}
