pub mod anthropic;
pub mod ollama;
pub mod openai;

use crate::LlmAdapter;
use crate::config::{LlmConfig, Provider};

/// Build the appropriate adapter from config.
pub fn from_config(config: LlmConfig) -> Box<dyn LlmAdapter> {
    match config.provider {
        Provider::Anthropic => Box::new(anthropic::AnthropicAdapter::new(config)),
        Provider::OpenAI => Box::new(openai::OpenAIAdapter::new(config)),
        Provider::Ollama => Box::new(ollama::OllamaAdapter::new(config)),
    }
}
