use crate::config::LlmConfig;
use crate::{LlmAdapter, LlmResponse, Message, Role, errors::LlmError};
use async_trait::async_trait;

pub struct AnthropicAdapter {
    config: LlmConfig,
    client: reqwest::Client,
}

impl AnthropicAdapter {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmAdapter for AnthropicAdapter {
    async fn complete(&self, messages: &[Message]) -> Result<LlmResponse, LlmError> {
        if self.config.api_key.is_empty() {
            return Err(LlmError::NoApiKey("anthropic".into()));
        }

        let system = messages
            .iter()
            .filter(|m| matches!(m.role, Role::System))
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| !matches!(m.role, Role::System))
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": msgs,
        });

        if !system.is_empty() {
            body["system"] = serde_json::Value::String(system);
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(LlmError::RequestFailed {
                provider: "anthropic".into(),
                reason: format!("{status}: {text}"),
            });
        }

        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| LlmError::ParseError {
                provider: "anthropic".into(),
                reason: e.to_string(),
            })?;

        let content = json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(LlmResponse {
            content,
            provider: "anthropic".into(),
            model: self.config.model.clone(),
            input_tokens: json["usage"]["input_tokens"].as_u64().map(|v| v as u32),
            output_tokens: json["usage"]["output_tokens"].as_u64().map(|v| v as u32),
        })
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}
