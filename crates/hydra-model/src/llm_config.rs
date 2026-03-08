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
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty()),
            openai_api_key: env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()),
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
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
