//! LLM stage — 4-provider adapter (Anthropic, OpenAI, Gemini, Ollama).
//! Reads provider config from environment. Retries transient failures.
//! Never panics. Never swallows errors silently.
//!
//! NOTE: The separate `hydra-llm` crate exists in the workspace with its own
//! adapter implementations. This kernel-inline LLM adapter was built during
//! the initial cognitive loop wiring and is the PRODUCTION implementation.
//! hydra-llm is NOT wired here — it is a parallel implementation, not a
//! replacement. DO NOT wire hydra-llm into this file or the kernel.
//! If hydra-llm needs to replace this in the future, it must be done as a
//! deliberate migration with full harness verification (47/47 + V2 behavioral),
//! not as a casual dependency swap.

use crate::loop_::prompt::EnrichedPrompt;
use crate::loop_::types::LlmResponse;
use crate::loop_::llm_key::resolve_api_key;

/// LLM call errors — all surfaced, none swallowed.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API key not set for provider {provider}. Set {key_env} environment variable.")]
    MissingKey { provider: String, key_env: String },
    #[error("Provider {provider}: {message}")]
    ProviderError { provider: String, message: String },
    #[error("Rate limited by {provider} (retry after backoff)")]
    RateLimited { provider: String },
    #[error("Network error: {message}")]
    Network { message: String },
}

pub struct LlmCaller {
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) api_key: String,
    pub(crate) base_url: String,
    pub(crate) client: reqwest::Client,
}

impl LlmCaller {
    pub fn from_env() -> Self {
        Self::load_dotenv();

        let provider = std::env::var("HYDRA_LLM_PROVIDER")
            .unwrap_or_else(|_| "anthropic".into())
            .to_lowercase();
        let model = std::env::var("HYDRA_LLM_MODEL").unwrap_or_else(|_| {
            match provider.as_str() {
                "openai" => "gpt-4o".into(),
                "gemini" => "gemini-2.0-flash".into(),
                "ollama" => "llama3.3".into(),
                _ => "claude-sonnet-4-20250514".into(),
            }
        });
        let key_env = match provider.as_str() {
            "openai" => "OPENAI_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            _ => "ANTHROPIC_API_KEY",
        };
        let api_key = resolve_api_key(&provider, key_env);
        let base_url =
            std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".into());

        Self {
            provider,
            model,
            api_key,
            base_url,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("http client"),
        }
    }

    /// Async micro-call. Use this from async contexts (intent classifier, middleware).
    /// Cheap model, 512 tokens, 15s timeout. Reads provider from env.
    pub async fn micro_call(prompt: &str) -> Option<String> {
        let provider = std::env::var("HYDRA_LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into()).to_lowercase();
        let key_env = match provider.as_str() {
            "openai" => "OPENAI_API_KEY", "gemini" => "GEMINI_API_KEY", _ => "ANTHROPIC_API_KEY",
        };
        let api_key = resolve_api_key(&provider, key_env);
        if api_key.is_empty() { return None; }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15)).build().ok()?;
        let (url, body) = match provider.as_str() {
            "openai" => ("https://api.openai.com/v1/chat/completions".to_string(), serde_json::json!({
                "model": "gpt-4o-mini", "max_tokens": 512,
                "messages": [{"role": "user", "content": prompt}]
            })),
            "ollama" => (format!("{}/api/chat", std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".into())),
                serde_json::json!({ "model": "llama3.2", "messages": [{"role": "user", "content": prompt}], "stream": false })),
            _ => ("https://api.anthropic.com/v1/messages".to_string(), serde_json::json!({
                "model": "claude-haiku-4-5-20251001", "max_tokens": 512,
                "messages": [{"role": "user", "content": prompt}]
            })),
        };
        let mut req = client.post(&url).header("content-type", "application/json").json(&body);
        match provider.as_str() {
            "openai" => { req = req.header("Authorization", format!("Bearer {api_key}")); }
            "ollama" => {}
            _ => { req = req.header("x-api-key", &api_key).header("anthropic-version", "2023-06-01"); }
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => { eprintln!("hydra-micro: send failed: {e}"); return None; }
        };
        let parsed: serde_json::Value = match resp.json().await {
            Ok(j) => j,
            Err(e) => { eprintln!("hydra-micro: parse failed: {e}"); return None; }
        };
        Self::extract_micro_response(&parsed)
    }

    /// Extract text from micro-call response (handles Anthropic, OpenAI, Ollama formats).
    fn extract_micro_response(parsed: &serde_json::Value) -> Option<String> {
        parsed.get("content").and_then(|c| c.as_array()).and_then(|a| a.first())
            .and_then(|b| b.get("text")).and_then(|t| t.as_str()).map(|s| s.to_string())
            .or_else(|| parsed.get("choices").and_then(|c| c.as_array()).and_then(|a| a.first())
                .and_then(|ch| ch.get("message")).and_then(|m| m.get("content")).and_then(|t| t.as_str()).map(|s| s.to_string()))
            .or_else(|| parsed.get("message").and_then(|m| m.get("content")).and_then(|t| t.as_str()).map(|s| s.to_string()))
    }

    /// Blocking micro-call. Only use from non-async contexts (CLI tools, tests).
    /// WARNING: This will fail inside an async runtime — use micro_call() instead.
    pub fn micro_call_blocking(prompt: &str) -> Option<String> {
        let provider = std::env::var("HYDRA_LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into()).to_lowercase();
        let key_env = match provider.as_str() {
            "openai" => "OPENAI_API_KEY", "gemini" => "GEMINI_API_KEY", _ => "ANTHROPIC_API_KEY",
        };
        let api_key = resolve_api_key(&provider, key_env);
        if api_key.is_empty() { return None; }
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15)).build().ok()?;
        let (url, body) = match provider.as_str() {
            "openai" => ("https://api.openai.com/v1/chat/completions".to_string(), serde_json::json!({
                "model": "gpt-4o-mini", "max_tokens": 512,
                "messages": [{"role": "user", "content": prompt}]
            })),
            "ollama" => (format!("{}/api/chat", std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".into())),
                serde_json::json!({ "model": "llama3.2", "messages": [{"role": "user", "content": prompt}], "stream": false })),
            _ => ("https://api.anthropic.com/v1/messages".to_string(), serde_json::json!({
                "model": "claude-haiku-4-5-20251001", "max_tokens": 512,
                "messages": [{"role": "user", "content": prompt}]
            })),
        };
        let mut req = client.post(&url).header("content-type", "application/json").json(&body);
        match provider.as_str() {
            "openai" => { req = req.header("Authorization", format!("Bearer {api_key}")); }
            "ollama" => {} // No auth
            _ => { req = req.header("x-api-key", &api_key).header("anthropic-version", "2023-06-01"); }
        }
        let resp = match req.send() {
            Ok(r) => r,
            Err(e) => { eprintln!("hydra-micro-blocking: send failed: {e}"); return None; }
        };
        let parsed: serde_json::Value = match resp.json() {
            Ok(j) => j,
            Err(e) => { eprintln!("hydra-micro-blocking: parse failed: {e}"); return None; }
        };
        Self::extract_micro_response(&parsed)
    }

    /// Call LLM with retry. Rate limits: 2/8/20s backoff. Network: 1/2/4s. Provider errors fail immediately.
    pub async fn call(&self, prompt: &EnrichedPrompt) -> Result<LlmResponse, LlmError> {
        let mut last_err = None;
        for attempt in 0..4u32 {
            if attempt > 0 {
                let delay = match &last_err {
                    Some(LlmError::RateLimited { .. }) => match attempt { 1 => 2000, 2 => 8000, _ => 20000 },
                    _ => 1000 * 2u64.pow(attempt - 1),
                };
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
            match self.call_once(prompt).await {
                Ok(r) => return Ok(r),
                Err(e @ LlmError::ProviderError { .. })
                | Err(e @ LlmError::MissingKey { .. }) => {
                    return Err(e);
                }
                Err(e) => {
                    eprintln!("[hydra] LLM attempt {} failed: {}", attempt + 1, e);
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or(LlmError::Network {
            message: "all retries exhausted".into(),
        }))
    }

    async fn call_once(&self, prompt: &EnrichedPrompt) -> Result<LlmResponse, LlmError> {
        let start = std::time::Instant::now();
        let max_tokens = (prompt.budget / 4).min(8096) as u32;
        match self.provider.as_str() {
            "openai" => self.openai(prompt, max_tokens, start).await,
            "gemini" => self.gemini(prompt, max_tokens, start).await,
            "ollama" => self.ollama(prompt, max_tokens, start).await,
            _ => self.anthropic(prompt, max_tokens, start).await,
        }
    }

    async fn anthropic(
        &self,
        prompt: &EnrichedPrompt,
        max_tokens: u32,
        start: std::time::Instant,
    ) -> Result<LlmResponse, LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::MissingKey {
                provider: "anthropic".into(),
                key_env: "ANTHROPIC_API_KEY".into(),
            });
        }
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "system": prompt.system,
            "messages": [{ "role": "user", "content": prompt.user }]
        });
        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                message: e.to_string(),
            })?;

        let status = resp.status();
        let json: serde_json::Value =
            resp.json().await.map_err(|e| LlmError::Network {
                message: e.to_string(),
            })?;

        if !status.is_success() {
            // 429 = rate limited, 529 = overloaded — both retryable
            if status.as_u16() == 429 || status.as_u16() == 529 {
                return Err(LlmError::RateLimited { provider: "anthropic".into() });
            }
            return Err(LlmError::ProviderError {
                provider: "anthropic".into(),
                message: json["error"]["message"].as_str().unwrap_or("unknown").to_string(),
            });
        }

        Ok(LlmResponse {
            content: json["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            tokens_used: json["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize,
            provider: "anthropic".into(),
            model: self.model.clone(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn openai(
        &self,
        prompt: &EnrichedPrompt,
        max_tokens: u32,
        start: std::time::Instant,
    ) -> Result<LlmResponse, LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::MissingKey {
                provider: "openai".into(),
                key_env: "OPENAI_API_KEY".into(),
            });
        }
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "messages": [
                { "role": "system", "content": prompt.system },
                { "role": "user",   "content": prompt.user   }
            ]
        });
        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                message: e.to_string(),
            })?;

        let status = resp.status();
        let json: serde_json::Value =
            resp.json().await.map_err(|e| LlmError::Network {
                message: e.to_string(),
            })?;

        if !status.is_success() {
            if status.as_u16() == 429 {
                return Err(LlmError::RateLimited {
                    provider: "openai".into(),
                });
            }
            return Err(LlmError::ProviderError {
                provider: "openai".into(),
                message: json["error"]["message"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
            });
        }

        Ok(LlmResponse {
            content: json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            tokens_used: json["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as usize,
            provider: "openai".into(),
            model: self.model.clone(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn gemini(
        &self,
        prompt: &EnrichedPrompt,
        max_tokens: u32,
        start: std::time::Instant,
    ) -> Result<LlmResponse, LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::MissingKey {
                provider: "gemini".into(),
                key_env: "GEMINI_API_KEY".into(),
            });
        }
        let combined = format!("{}\n\n{}", prompt.system, prompt.user);
        let body = serde_json::json!({
            "contents": [{ "parts": [{ "text": combined }] }],
            "generationConfig": { "maxOutputTokens": max_tokens }
        });
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/\
             {}:generateContent?key={}",
            self.model, self.api_key
        );
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                message: e.to_string(),
            })?;

        let json: serde_json::Value =
            resp.json().await.map_err(|e| LlmError::Network {
                message: e.to_string(),
            })?;

        Ok(LlmResponse {
            content: json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            tokens_used: 0,
            provider: "gemini".into(),
            model: self.model.clone(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn ollama(
        &self,
        prompt: &EnrichedPrompt,
        max_tokens: u32,
        start: std::time::Instant,
    ) -> Result<LlmResponse, LlmError> {
        let combined = format!("{}\n\n{}", prompt.system, prompt.user);
        let body = serde_json::json!({
            "model": self.model,
            "prompt": combined,
            "stream": false,
            "options": { "num_predict": max_tokens }
        });
        let url = format!("{}/api/generate", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                message: e.to_string(),
            })?;

        if !resp.status().is_success() {
            if resp.status().as_u16() == 429 {
                return Err(LlmError::RateLimited {
                    provider: "ollama".into(),
                });
            }
            return Err(LlmError::ProviderError {
                provider: "ollama".into(),
                message: format!("HTTP {}", resp.status()),
            });
        }

        let json: serde_json::Value =
            resp.json().await.map_err(|e| LlmError::Network {
                message: e.to_string(),
            })?;

        Ok(LlmResponse {
            content: json["response"].as_str().unwrap_or("").to_string(),
            tokens_used: json["eval_count"].as_u64().unwrap_or(0) as usize,
            provider: "ollama".into(),
            model: self.model.clone(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Load .env file into process environment. Walks up from cwd.
    pub fn load_dotenv() { crate::loop_::llm_key::load_dotenv(); }
}
