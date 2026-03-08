//! OllamaClient — HTTP client for the Ollama API (localhost:11434).

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::providers::{CompletionRequest, CompletionResponse, LlmError, Message};

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const OLLAMA_TIMEOUT: Duration = Duration::from_secs(120);

/// Client for the Ollama local LLM API
pub struct OllamaClient {
    client: Client,
    base_url: String,
}

// ── Ollama API types ──

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize, Deserialize, Clone)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatMessage,
    model: String,
    #[serde(default)]
    eval_count: u64,
    #[serde(default)]
    prompt_eval_count: u64,
    #[serde(default)]
    done: bool,
}

/// Response from GET /api/tags
#[derive(Debug, Deserialize)]
pub struct OllamaTagsResponse {
    pub models: Vec<OllamaModelInfo>,
}

/// Info about a model available in Ollama
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaModelInfo {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub digest: String,
}

impl OllamaClient {
    pub fn new() -> Self {
        Self::with_url(DEFAULT_OLLAMA_URL)
    }

    pub fn with_url(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(OLLAMA_TIMEOUT)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Check if Ollama is running and reachable
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.client
            .get(&url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .is_ok()
    }

    /// List all models available in Ollama
    pub async fn list_models(&self) -> Result<Vec<OllamaModelInfo>, LlmError> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(LlmError::ApiError {
                status: resp.status().as_u16(),
                message: "Failed to list Ollama models".into(),
            });
        }

        let tags: OllamaTagsResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        Ok(tags.models)
    }

    /// Send a chat completion request to Ollama
    pub async fn chat(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let url = format!("{}/api/chat", self.base_url);

        // Build messages, prepending system message if present
        let mut messages: Vec<OllamaChatMessage> = Vec::new();
        if let Some(system) = &request.system {
            messages.push(OllamaChatMessage {
                role: "system".into(),
                content: system.clone(),
            });
        }
        for msg in &request.messages {
            messages.push(OllamaChatMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        // Strip "local-" prefix from model name if present
        let model = request
            .model
            .strip_prefix("local-")
            .unwrap_or(&request.model)
            .to_string();

        let body = OllamaChatRequest {
            model,
            messages,
            stream: false,
            options: Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: Some(request.max_tokens),
            }),
        };

        let resp = self.client.post(&url).json(&body).send().await?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ApiError {
                status,
                message: error_body,
            });
        }

        let api_resp: OllamaChatResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        Ok(CompletionResponse {
            content: api_resp.message.content,
            model: api_resp.model,
            input_tokens: api_resp.prompt_eval_count,
            output_tokens: api_resp.eval_count,
            stop_reason: if api_resp.done {
                Some("stop".into())
            } else {
                None
            },
        })
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&Message> for OllamaChatMessage {
    fn from(m: &Message) -> Self {
        OllamaChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_client_creation() {
        let client = OllamaClient::new();
        assert_eq!(client.base_url, DEFAULT_OLLAMA_URL);
    }

    #[test]
    fn test_ollama_client_custom_url() {
        let client = OllamaClient::with_url("http://remote:11434/");
        assert_eq!(client.base_url, "http://remote:11434");
    }

    #[test]
    fn test_ollama_model_strip_prefix() {
        // Verify the strip logic used in chat()
        let model = "local-phi3";
        let stripped = model.strip_prefix("local-").unwrap_or(model);
        assert_eq!(stripped, "phi3");

        let model2 = "phi3";
        let stripped2 = model2.strip_prefix("local-").unwrap_or(model2);
        assert_eq!(stripped2, "phi3");
    }

    #[tokio::test]
    async fn test_graceful_when_ollama_unavailable() {
        // Connect to a port nothing is listening on
        let client = OllamaClient::with_url("http://localhost:19999");
        assert!(!client.is_available().await);
    }

    #[tokio::test]
    #[cfg(feature = "local-llm")]
    async fn test_live_ollama_list() {
        let client = OllamaClient::new();
        if !client.is_available().await {
            eprintln!("Ollama not running, skipping live test");
            return;
        }
        let models = client.list_models().await.unwrap();
        // Just verify we get a response — may be empty
        println!(
            "Ollama models: {:?}",
            models.iter().map(|m| &m.name).collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    #[cfg(feature = "local-llm")]
    async fn test_live_ollama_chat() {
        let client = OllamaClient::new();
        if !client.is_available().await {
            eprintln!("Ollama not running, skipping live test");
            return;
        }
        let models = client.list_models().await.unwrap();
        if models.is_empty() {
            eprintln!("No models installed, skipping live chat test");
            return;
        }
        let model_name = &models[0].name;
        let req = CompletionRequest {
            model: model_name.clone(),
            messages: vec![Message {
                role: "user".into(),
                content: "Say hello in 5 words.".into(),
            }],
            max_tokens: 50,
            temperature: Some(0.0),
            system: None,
        };
        let resp = client.chat(req).await.unwrap();
        assert!(!resp.content.is_empty());
        println!("Ollama response: {}", resp.content);
    }
}
