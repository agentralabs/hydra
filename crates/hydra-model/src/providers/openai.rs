use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{CompletionRequest, CompletionResponse, LlmError};
use crate::llm_config::LlmConfig;

/// Client for the OpenAI Chat Completions API
pub struct OpenAiClient {
    client: Client,
    api_key: String,
    base_url: String,
}

// ── OpenAI API types ──

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    model: String,
    usage: OpenAiUsage,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Deserialize)]
struct OpenAiError {
    error: OpenAiErrorDetail,
}

#[derive(Deserialize)]
struct OpenAiErrorDetail {
    message: String,
}

impl OpenAiClient {
    pub fn new(config: &LlmConfig) -> Result<Self, LlmError> {
        let api_key = config.openai_api_key.clone().ok_or(LlmError::NoApiKey)?;
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
            api_key,
            base_url: config.openai_base_url.clone(),
        })
    }

    /// Map model profile IDs to actual OpenAI model IDs
    fn resolve_model(model_id: &str) -> &str {
        match model_id {
            "gpt-4o" => "gpt-4o",
            "gpt-4o-mini" => "gpt-4o-mini",
            other => other,
        }
    }

    pub async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, LlmError> {
        let api_model = Self::resolve_model(&request.model);

        let mut messages: Vec<OpenAiMessage> = Vec::new();

        // Add system message if present
        if let Some(system) = &request.system {
            messages.push(OpenAiMessage {
                role: "system".into(),
                content: system.clone(),
            });
        }

        // Add user/assistant messages
        for m in &request.messages {
            messages.push(OpenAiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            });
        }

        // Clamp max_tokens to model's actual API limit to prevent 400 errors
        let clamped_max_tokens = match api_model {
            "gpt-4o" | "gpt-4o-mini" => std::cmp::min(request.max_tokens, 16_384),
            m if m.contains("gpt-4") => std::cmp::min(request.max_tokens, 8_192),
            _ => std::cmp::min(request.max_tokens, 16_384),
        };

        let body = OpenAiRequest {
            model: api_model.into(),
            messages,
            max_tokens: clamped_max_tokens,
            temperature: request.temperature,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("authorization", format!("Bearer {}", self.api_key))
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
            let message = serde_json::from_str::<OpenAiError>(&error_body)
                .map(|e| e.error.message)
                .unwrap_or(error_body);
            return Err(LlmError::ApiError { status, message });
        }

        let api_resp: OpenAiResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        let choice = api_resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::ParseError("No choices in response".into()))?;

        Ok(CompletionResponse {
            content: choice.message.content,
            model: api_resp.model,
            input_tokens: api_resp.usage.prompt_tokens,
            output_tokens: api_resp.usage.completion_tokens,
            stop_reason: choice.finish_reason,
        })
    }
}
