use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{CompletionRequest, CompletionResponse, LlmError, Message};
use crate::llm_config::LlmConfig;

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Client for the Anthropic Messages API
pub struct AnthropicClient {
    client: Client,
    api_key: String,
    base_url: String,
}

// ── Anthropic API types ──

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Deserialize)]
struct AnthropicError {
    error: AnthropicErrorDetail,
}

#[derive(Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

impl AnthropicClient {
    pub fn new(config: &LlmConfig) -> Result<Self, LlmError> {
        let api_key = config.anthropic_api_key.clone().ok_or(LlmError::NoApiKey)?;
        Ok(Self {
            client: Client::new(),
            api_key,
            base_url: config.anthropic_base_url.clone(),
        })
    }

    /// Map model profile IDs to actual Anthropic model IDs
    fn resolve_model(model_id: &str) -> &str {
        match model_id {
            "claude-opus" => "claude-opus-4-6",
            "claude-sonnet" => "claude-sonnet-4-6",
            "claude-haiku" => "claude-haiku-4-5-20251001",
            other => other,
        }
    }

    pub async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, LlmError> {
        let api_model = Self::resolve_model(&request.model);

        let body = AnthropicRequest {
            model: api_model.into(),
            messages: request
                .messages
                .iter()
                .map(|m| AnthropicMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            system: request.system,
        };

        let url = format!("{}/v1/messages", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status().as_u16();

        if status == 429 {
            return Err(LlmError::RateLimited);
        }

        if !resp.status().is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            let message = serde_json::from_str::<AnthropicError>(&error_body)
                .map(|e| e.error.message)
                .unwrap_or(error_body);
            return Err(LlmError::ApiError { status, message });
        }

        let api_resp: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        let content = api_resp
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResponse {
            content,
            model: api_resp.model,
            input_tokens: api_resp.usage.input_tokens,
            output_tokens: api_resp.usage.output_tokens,
            stop_reason: api_resp.stop_reason,
        })
    }
}

impl From<&Message> for AnthropicMessage {
    fn from(m: &Message) -> Self {
        AnthropicMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        }
    }
}
