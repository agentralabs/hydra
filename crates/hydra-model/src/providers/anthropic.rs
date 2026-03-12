use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{CompletionRequest, CompletionResponse, LlmError, Message};
use crate::llm_config::LlmConfig;

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Authentication mode for the Anthropic API.
#[derive(Debug, Clone)]
pub enum AnthropicAuth {
    /// Traditional API key (x-api-key header).
    ApiKey(String),
    /// OAuth bearer token (Authorization: Bearer header) — uses subscription credits.
    OAuthToken(String),
}

/// Client for the Anthropic Messages API
pub struct AnthropicClient {
    client: Client,
    auth: AnthropicAuth,
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
        let auth = if let Some(ref key) = config.anthropic_api_key {
            if key.starts_with("sk-ant-") {
                AnthropicAuth::ApiKey(key.clone())
            } else {
                // Treat non-sk-ant keys as OAuth tokens
                AnthropicAuth::OAuthToken(key.clone())
            }
        } else {
            return Err(LlmError::NoApiKey);
        };
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
            auth,
            base_url: config.anthropic_base_url.clone(),
        })
    }

    /// Create a client from an OAuth token directly.
    pub fn from_oauth_token(token: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
            auth: AnthropicAuth::OAuthToken(token.to_string()),
            base_url: "https://api.anthropic.com".into(),
        }
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

        // Clamp max_tokens to model's actual API limit to prevent 400 errors
        let clamped_max_tokens = match api_model {
            "claude-opus-4-6" => std::cmp::min(request.max_tokens, 32_768),
            "claude-sonnet-4-6" => std::cmp::min(request.max_tokens, 16_384),
            "claude-haiku-4-5-20251001" => std::cmp::min(request.max_tokens, 8_192),
            _ => std::cmp::min(request.max_tokens, 16_384), // safe default
        };

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
            max_tokens: clamped_max_tokens,
            temperature: request.temperature,
            system: request.system,
        };

        let url = format!("{}/v1/messages", self.base_url);

        let mut req = self
            .client
            .post(&url)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json");

        // Apply authentication based on mode
        req = match &self.auth {
            AnthropicAuth::ApiKey(key) => req.header("x-api-key", key),
            AnthropicAuth::OAuthToken(token) => req.bearer_auth(token),
        };

        let resp = req.json(&body).send().await?;

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

    /// Streaming completion — sends chunks via callback as they arrive.
    /// Returns the full response with token counts after the stream completes.
    pub async fn complete_streaming(
        &self,
        request: CompletionRequest,
        on_chunk: impl Fn(&str),
    ) -> Result<CompletionResponse, LlmError> {
        let api_model = Self::resolve_model(&request.model);
        let clamped_max_tokens = match api_model {
            "claude-opus-4-6" => std::cmp::min(request.max_tokens, 32_768),
            "claude-sonnet-4-6" => std::cmp::min(request.max_tokens, 16_384),
            "claude-haiku-4-5-20251001" => std::cmp::min(request.max_tokens, 8_192),
            _ => std::cmp::min(request.max_tokens, 16_384),
        };

        let body = serde_json::json!({
            "model": api_model,
            "messages": request.messages.iter().map(|m| serde_json::json!({
                "role": m.role, "content": m.content
            })).collect::<Vec<_>>(),
            "max_tokens": clamped_max_tokens,
            "temperature": request.temperature,
            "system": request.system,
            "stream": true,
        });

        let url = format!("{}/v1/messages", self.base_url);
        let mut req = self.client.post(&url)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json");
        req = match &self.auth {
            AnthropicAuth::ApiKey(key) => req.header("x-api-key", key),
            AnthropicAuth::OAuthToken(token) => req.bearer_auth(token),
        };

        let resp = req.json(&body).send().await?;
        let status = resp.status();
        if status.as_u16() == 429 { return Err(LlmError::RateLimited); }
        if !status.is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            let message = serde_json::from_str::<AnthropicError>(&error_body)
                .map(|e| e.error.message).unwrap_or(error_body);
            return Err(LlmError::ApiError { status: status.as_u16(), message });
        }

        // Parse SSE stream
        let full_text = resp.text().await.map_err(|e| LlmError::ParseError(e.to_string()))?;
        let mut content = String::new();
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut model_name = api_model.to_string();
        let mut stop_reason = None;

        for line in full_text.lines() {
            let line = line.trim();
            if !line.starts_with("data: ") { continue; }
            let data = &line[6..];
            if data == "[DONE]" { break; }
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                match event["type"].as_str() {
                    Some("content_block_delta") => {
                        if let Some(text) = event["delta"]["text"].as_str() {
                            content.push_str(text);
                            on_chunk(text);
                        }
                    }
                    Some("message_start") => {
                        if let Some(m) = event["message"]["model"].as_str() { model_name = m.to_string(); }
                        input_tokens = event["message"]["usage"]["input_tokens"].as_u64().unwrap_or(0);
                    }
                    Some("message_delta") => {
                        output_tokens = event["usage"]["output_tokens"].as_u64().unwrap_or(0);
                        stop_reason = event["delta"]["stop_reason"].as_str().map(|s| s.to_string());
                    }
                    _ => {}
                }
            }
        }

        Ok(CompletionResponse { content, model: model_name, input_tokens, output_tokens, stop_reason })
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
