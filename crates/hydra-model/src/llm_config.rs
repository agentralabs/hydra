use std::env;

/// LLM provider configuration loaded from environment
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_base_url: String,
    pub openai_base_url: String,
}

impl LlmConfig {
    /// Load config from environment variables
    pub fn from_env() -> Self {
        Self {
            anthropic_api_key: sanitize_key(env::var("ANTHROPIC_API_KEY").ok()),
            openai_api_key: sanitize_key(env::var("OPENAI_API_KEY").ok()),
            anthropic_base_url: env::var("ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".into()),
            openai_base_url: env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com".into()),
        }
    }

    pub fn has_anthropic(&self) -> bool {
        self.anthropic_api_key.is_some()
    }

    pub fn has_openai(&self) -> bool {
        self.openai_api_key.is_some()
    }

    pub fn has_provider(&self, provider: &str) -> bool {
        match provider {
            "anthropic" => self.has_anthropic(),
            "openai" => self.has_openai(),
            "local" | "ollama" | "deepseek" => true,
            _ => false,
        }
    }

    /// Build config from env + overlay keys from CognitiveLoopConfig fields.
    /// Sanitizes all keys to prevent reqwest builder errors from non-ASCII chars.
    pub fn from_env_with_overlay(
        anthropic_key: &str,
        openai_key: &str,
        oauth_token: Option<&str>,
    ) -> Self {
        let mut cfg = Self::from_env();
        if let Some(oauth) = oauth_token {
            cfg.anthropic_api_key = sanitize_key(Some(oauth.to_string()));
        } else if !anthropic_key.is_empty() {
            cfg.anthropic_api_key = sanitize_key(Some(anthropic_key.to_string()));
        }
        if !openai_key.is_empty() {
            cfg.openai_api_key = sanitize_key(Some(openai_key.to_string()));
        }
        cfg
    }
}

/// Sanitize API keys: trim whitespace, strip non-ASCII-printable characters.
/// Keeps only bytes in 0x21..0x7E range (visible ASCII, no spaces/control/DEL).
pub fn sanitize_key(key: Option<String>) -> Option<String> {
    key.map(|k| {
        k.chars()
            .filter(|c| c.is_ascii_graphic()) // 0x21..=0x7E: visible ASCII only
            .collect::<String>()
    })
    .filter(|k| !k.is_empty())
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
