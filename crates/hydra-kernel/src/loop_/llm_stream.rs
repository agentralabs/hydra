//! Streaming LLM support — all 4 providers.
//!
//! Yields text chunks via a channel as the LLM generates them.
//! Supports: Anthropic SSE, OpenAI SSE, Gemini SSE, Ollama NDJSON.
//! Non-streaming path in llm.rs is preserved for harness/CLI usage.

use crate::loop_::llm::{LlmCaller, LlmError};
use crate::loop_::prompt::EnrichedPrompt;
use futures_util::StreamExt;
use tokio::sync::mpsc;

/// A single chunk of streamed text from the LLM.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// A piece of the response text.
    Text(String),
    /// Stream is complete. Contains total tokens used.
    Done { tokens_used: usize, duration_ms: u64 },
    /// An error occurred during streaming.
    Error(String),
}

impl LlmCaller {
    /// Call the LLM with streaming enabled. Returns a channel receiver
    /// that yields text chunks as they arrive. Works with all 4 providers.
    pub async fn call_streaming(
        &self,
        prompt: &EnrichedPrompt,
    ) -> Result<mpsc::Receiver<StreamChunk>, LlmError> {
        let (tx, rx) = mpsc::channel(64);
        let start = std::time::Instant::now();
        let max_tokens = (prompt.budget / 4).min(8096) as u32;

        let resp = match self.provider.as_str() {
            "openai" => self.stream_openai(prompt, max_tokens).await?,
            "gemini" => {
                // Gemini doesn't support SSE streaming well — fallback
                return self.fallback_stream(prompt, tx).await.map(|_| rx);
            }
            "ollama" => self.stream_ollama(prompt, max_tokens).await?,
            _ => self.stream_anthropic(prompt, max_tokens).await?,
        };

        let provider = self.provider.clone();
        let model = self.model.clone();

        tokio::spawn(async move {
            parse_sse_stream(resp, &provider, tx, start, &model).await;
        });

        Ok(rx)
    }

    async fn stream_anthropic(
        &self,
        prompt: &EnrichedPrompt,
        max_tokens: u32,
    ) -> Result<reqwest::Response, LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::MissingKey {
                provider: "anthropic".into(),
                key_env: "ANTHROPIC_API_KEY".into(),
            });
        }
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "stream": true,
            "system": prompt.system,
            "messages": [{ "role": "user", "content": prompt.user }]
        });
        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network { message: e.to_string() })?;
        check_status(resp, "anthropic").await
    }

    async fn stream_openai(
        &self,
        prompt: &EnrichedPrompt,
        max_tokens: u32,
    ) -> Result<reqwest::Response, LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::MissingKey {
                provider: "openai".into(),
                key_env: "OPENAI_API_KEY".into(),
            });
        }
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "stream": true,
            "messages": [
                { "role": "system", "content": prompt.system },
                { "role": "user", "content": prompt.user }
            ]
        });
        let resp = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network { message: e.to_string() })?;
        check_status(resp, "openai").await
    }

    async fn stream_ollama(
        &self,
        prompt: &EnrichedPrompt,
        max_tokens: u32,
    ) -> Result<reqwest::Response, LlmError> {
        let combined = format!("{}\n\n{}", prompt.system, prompt.user);
        let body = serde_json::json!({
            "model": self.model,
            "prompt": combined,
            "stream": true,
            "options": { "num_predict": max_tokens }
        });
        let url = format!("{}/api/generate", self.base_url);
        let resp = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network { message: e.to_string() })?;
        check_status(resp, "ollama").await
    }

    async fn fallback_stream(
        &self,
        prompt: &EnrichedPrompt,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<(), LlmError> {
        let response = self.call(prompt).await?;
        let _ = tx.send(StreamChunk::Text(response.content)).await;
        let _ = tx.send(StreamChunk::Done {
            tokens_used: response.tokens_used,
            duration_ms: response.duration_ms,
        }).await;
        Ok(())
    }
}

async fn check_status(resp: reqwest::Response, provider: &str) -> Result<reqwest::Response, LlmError> {
    let status = resp.status();
    if !status.is_success() {
        if status.as_u16() == 429 || status.as_u16() == 529 {
            return Err(LlmError::RateLimited { provider: provider.into() });
        }
        let body = resp.text().await.unwrap_or_default();
        return Err(LlmError::ProviderError {
            provider: provider.into(),
            message: body,
        });
    }
    Ok(resp)
}

/// Parse SSE stream from any provider and send chunks through channel.
async fn parse_sse_stream(
    resp: reqwest::Response,
    provider: &str,
    tx: mpsc::Sender<StreamChunk>,
    start: std::time::Instant,
    model: &str,
) {
    let mut bytes = resp.bytes_stream();
    let mut buffer = String::new();
    let mut tokens_used: usize = 0;

    while let Some(chunk_result) = bytes.next().await {
        let chunk = match chunk_result {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(StreamChunk::Error(e.to_string())).await;
                return;
            }
        };

        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete lines
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            let text = match provider {
                "anthropic" => parse_anthropic_sse(&line, &mut tokens_used),
                "openai" => parse_openai_sse(&line),
                "ollama" => parse_ollama_ndjson(&line, &mut tokens_used),
                _ => parse_anthropic_sse(&line, &mut tokens_used),
            };

            if let Some(t) = text {
                if tx.send(StreamChunk::Text(t)).await.is_err() {
                    return; // receiver dropped (interrupted)
                }
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let _ = tx.send(StreamChunk::Done { tokens_used, duration_ms }).await;
    eprintln!("[hydra] stream complete: {}tok {}ms model={}", tokens_used, duration_ms, model);
}

fn parse_anthropic_sse(line: &str, tokens: &mut usize) -> Option<String> {
    if !line.starts_with("data: ") { return None; }
    let data = &line[6..];
    if data == "[DONE]" { return None; }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    match json["type"].as_str()? {
        "content_block_delta" => json["delta"]["text"].as_str().map(String::from),
        "message_delta" => {
            if let Some(t) = json["usage"]["output_tokens"].as_u64() { *tokens = t as usize; }
            None
        }
        _ => None,
    }
}

fn parse_openai_sse(line: &str) -> Option<String> {
    if !line.starts_with("data: ") { return None; }
    let data = &line[6..];
    if data == "[DONE]" { return None; }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    json["choices"][0]["delta"]["content"].as_str().map(String::from)
}

fn parse_ollama_ndjson(line: &str, tokens: &mut usize) -> Option<String> {
    if line.is_empty() { return None; }
    let json: serde_json::Value = serde_json::from_str(line).ok()?;
    if let Some(t) = json["eval_count"].as_u64() { *tokens = t as usize; }
    json["response"].as_str().map(String::from)
}
