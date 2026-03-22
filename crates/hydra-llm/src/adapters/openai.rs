use crate::config::LlmConfig;
use crate::{LlmAdapter, LlmResponse, Message, errors::LlmError};
use async_trait::async_trait;

pub struct OpenAIAdapter {
    config: LlmConfig,
    client: reqwest::Client,
}

impl OpenAIAdapter {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmAdapter for OpenAIAdapter {
    async fn complete(&self, messages: &[Message]) -> Result<LlmResponse, LlmError> {
        if self.config.api_key.is_empty() {
            return Err(LlmError::NoApiKey("openai".into()));
        }

        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "messages": msgs,
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(LlmError::RequestFailed {
                provider: "openai".into(),
                reason: format!("{status}: {text}"),
            });
        }

        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| LlmError::ParseError {
                provider: "openai".into(),
                reason: e.to_string(),
            })?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(LlmResponse {
            content,
            provider: "openai".into(),
            model: self.config.model.clone(),
            input_tokens: json["usage"]["prompt_tokens"].as_u64().map(|v| v as u32),
            output_tokens: json["usage"]["completion_tokens"]
                .as_u64()
                .map(|v| v as u32),
        })
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}
