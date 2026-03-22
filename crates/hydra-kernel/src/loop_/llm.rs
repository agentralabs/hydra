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
        let api_key = std::env::var(match provider.as_str() {
            "openai" => "OPENAI_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            _ => "ANTHROPIC_API_KEY",
        })
        .unwrap_or_default();
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

    /// Call the LLM with retry on transient failures.
    /// Rate limits (429/529) get longer backoff: 2s, 8s, 20s.
    /// Network errors get standard backoff: 1s, 2s, 4s.
    /// ProviderError and MissingKey fail immediately (not retryable).
    pub async fn call(&self, prompt: &EnrichedPrompt) -> Result<LlmResponse, LlmError> {
        let mut last_err = None;
        for attempt in 0..4u32 {
            if attempt > 0 {
                let delay = match &last_err {
                    Some(LlmError::RateLimited { .. }) => {
                        // Longer backoff for rate limits: 2s, 8s, 20s
                        match attempt {
                            1 => 2000,
                            2 => 8000,
                            _ => 20000,
                        }
                    }
                    _ => 1000 * 2u64.pow(attempt - 1), // 1s, 2s, 4s
                };
                eprintln!("[hydra] retrying in {}ms (attempt {}/4)...", delay, attempt + 1);
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
                return Err(LlmError::RateLimited {
                    provider: "anthropic".into(),
                });
            }
            return Err(LlmError::ProviderError {
                provider: "anthropic".into(),
                message: json["error"]["message"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
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
    fn load_dotenv() {
        let mut dir = std::env::current_dir().ok();
        while let Some(d) = dir {
            let env_path = d.join(".env");
            if env_path.is_file() {
                if let Ok(contents) = std::fs::read_to_string(&env_path) {
                    for line in contents.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        if let Some((key, val)) = line.split_once('=') {
                            let key = key.trim();
                            let val = val.trim();
                            if std::env::var(key).is_err() {
                                // SAFETY: called once at startup before threads
                                unsafe { std::env::set_var(key, val) };
                            }
                        }
                    }
                }
                return;
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
}
