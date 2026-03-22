/// Configuration for an LLM provider connection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmConfig {
    pub provider: Provider,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Anthropic,
    OpenAI,
    Ollama,
}

impl LlmConfig {
    pub fn anthropic(api_key: impl Into<String>) -> Self {
        Self {
            provider: Provider::Anthropic,
            api_key: api_key.into(),
            model: "claude-haiku-4-5-20251001".into(),
            max_tokens: 256,
            temperature: 0.0,
            base_url: None,
        }
    }

    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            provider: Provider::OpenAI,
            api_key: api_key.into(),
            model: "gpt-4o-mini".into(),
            max_tokens: 256,
            temperature: 0.0,
            base_url: None,
        }
    }

    pub fn ollama() -> Self {
        Self {
            provider: Provider::Ollama,
            api_key: String::new(),
            model: "llama3.2".into(),
            max_tokens: 256,
            temperature: 0.0,
            base_url: Some("http://localhost:11434".into()),
        }
    }
}
