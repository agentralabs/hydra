use crate::config::LlmConfig;
use crate::{LlmAdapter, LlmResponse, Message, errors::LlmError};
use async_trait::async_trait;

pub struct OllamaAdapter {
    config: LlmConfig,
    client: reqwest::Client,
}

impl OllamaAdapter {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmAdapter for OllamaAdapter {
    async fn complete(&self, messages: &[Message]) -> Result<LlmResponse, LlmError> {
        let base = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434");

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
            "messages": msgs,
            "stream": false,
        });

        let resp = self
            .client
            .post(format!("{base}/api/chat"))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(LlmError::RequestFailed {
                provider: "ollama".into(),
                reason: format!("{status}: {text}"),
            });
        }

        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| LlmError::ParseError {
                provider: "ollama".into(),
                reason: e.to_string(),
            })?;

        let content = json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(LlmResponse {
            content,
            provider: "ollama".into(),
            model: self.config.model.clone(),
            input_tokens: json["prompt_eval_count"].as_u64().map(|v| v as u32),
            output_tokens: json["eval_count"].as_u64().map(|v| v as u32),
        })
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}
