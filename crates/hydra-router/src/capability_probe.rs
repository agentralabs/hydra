//! LLM capability probing — detects what the connected model can do.
//!
//! Phase 4, Part D: Runs once at connection time, cached for the session.
//! Adapts Hydra's request format to the model's native capabilities.

use serde::{Deserialize, Serialize};

/// The tool-calling format the LLM natively supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFormat {
    /// Anthropic format: tools array with input_schema
    Anthropic,
    /// OpenAI format: functions array
    OpenAI,
    /// Ollama: depends on model, may or may not support tools
    Ollama,
    /// Text-only: LLM has no native tool support, simulate via prompting
    None,
}

/// Discovered capabilities of the connected LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMCapabilityProfile {
    pub model_id: String,
    pub provider: String,
    pub supports_tool_use: bool,
    pub supports_parallel_tools: bool,
    pub supports_structured_output: bool,
    pub supports_vision: bool,
    pub max_context_tokens: u32,
    pub supports_streaming: bool,
    pub native_tool_format: ToolFormat,
    /// When this profile was probed.
    pub probed_at: chrono::DateTime<chrono::Utc>,
}

impl LLMCapabilityProfile {
    /// Probe capabilities based on the model ID string.
    /// This uses known model characteristics rather than runtime probing
    /// (faster, no extra API call).
    pub fn from_model_id(model_id: &str, provider: &str) -> Self {
        let lower = model_id.to_lowercase();

        // Anthropic Claude models
        if provider == "anthropic" || lower.contains("claude") {
            return Self {
                model_id: model_id.to_string(),
                provider: "anthropic".to_string(),
                supports_tool_use: true,
                supports_parallel_tools: true,
                supports_structured_output: true,
                supports_vision: true,
                max_context_tokens: if lower.contains("haiku") {
                    200_000
                } else {
                    200_000
                },
                supports_streaming: true,
                native_tool_format: ToolFormat::Anthropic,
                probed_at: chrono::Utc::now(),
            };
        }

        // OpenAI GPT models
        if provider == "openai" || lower.contains("gpt") {
            return Self {
                model_id: model_id.to_string(),
                provider: "openai".to_string(),
                supports_tool_use: true,
                supports_parallel_tools: lower.contains("gpt-4") || lower.contains("gpt-4o"),
                supports_structured_output: true,
                supports_vision: lower.contains("vision") || lower.contains("4o") || lower.contains("4-turbo"),
                max_context_tokens: if lower.contains("128k") || lower.contains("4o") {
                    128_000
                } else {
                    16_000
                },
                supports_streaming: true,
                native_tool_format: ToolFormat::OpenAI,
                probed_at: chrono::Utc::now(),
            };
        }

        // Google Gemini models
        if provider == "google" || lower.contains("gemini") {
            return Self {
                model_id: model_id.to_string(),
                provider: "google".to_string(),
                supports_tool_use: true,
                supports_parallel_tools: true,
                supports_structured_output: true,
                supports_vision: true,
                max_context_tokens: 1_000_000,
                supports_streaming: true,
                native_tool_format: ToolFormat::OpenAI, // Google uses OpenAI-compatible format
                probed_at: chrono::Utc::now(),
            };
        }

        // Ollama local models
        if provider == "ollama" || lower.contains("llama") || lower.contains("mistral")
            || lower.contains("codellama") || lower.contains("deepseek")
        {
            // Most Ollama models have limited tool support
            let has_tools = lower.contains("mistral") || lower.contains("deepseek");
            return Self {
                model_id: model_id.to_string(),
                provider: "ollama".to_string(),
                supports_tool_use: has_tools,
                supports_parallel_tools: false,
                supports_structured_output: has_tools,
                supports_vision: lower.contains("llava") || lower.contains("bakllava"),
                max_context_tokens: if lower.contains("128k") { 128_000 } else { 8_000 },
                supports_streaming: true,
                native_tool_format: if has_tools { ToolFormat::Ollama } else { ToolFormat::None },
                probed_at: chrono::Utc::now(),
            };
        }

        // Unknown model — conservative defaults
        Self {
            model_id: model_id.to_string(),
            provider: provider.to_string(),
            supports_tool_use: false,
            supports_parallel_tools: false,
            supports_structured_output: false,
            supports_vision: false,
            max_context_tokens: 4_096,
            supports_streaming: true,
            native_tool_format: ToolFormat::None,
            probed_at: chrono::Utc::now(),
        }
    }

    /// Whether this model can use tools natively (no prompt simulation needed).
    pub fn has_native_tools(&self) -> bool {
        self.supports_tool_use && self.native_tool_format != ToolFormat::None
    }

    /// Suggested max tokens for a request to this model.
    pub fn suggested_max_tokens(&self, is_complex: bool) -> u32 {
        if is_complex {
            // Use up to 10% of context for output on complex tasks
            (self.max_context_tokens / 10).min(8_192)
        } else {
            // Simple tasks: short output
            1_024
        }
    }

    /// Suggested temperature for this model.
    pub fn suggested_temperature(&self, is_complex: bool) -> f64 {
        if is_complex { 0.3 } else { 0.7 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_claude() {
        let profile = LLMCapabilityProfile::from_model_id("claude-sonnet-4-6", "anthropic");
        assert!(profile.supports_tool_use);
        assert!(profile.supports_parallel_tools);
        assert!(profile.supports_vision);
        assert_eq!(profile.native_tool_format, ToolFormat::Anthropic);
        assert_eq!(profile.max_context_tokens, 200_000);
    }

    #[test]
    fn test_probe_gpt4o() {
        let profile = LLMCapabilityProfile::from_model_id("gpt-4o", "openai");
        assert!(profile.supports_tool_use);
        assert!(profile.supports_parallel_tools);
        assert!(profile.supports_vision);
        assert_eq!(profile.native_tool_format, ToolFormat::OpenAI);
        assert_eq!(profile.max_context_tokens, 128_000);
    }

    #[test]
    fn test_probe_ollama_llama() {
        let profile = LLMCapabilityProfile::from_model_id("llama3.1", "ollama");
        assert!(!profile.supports_tool_use);
        assert!(!profile.supports_parallel_tools);
        assert_eq!(profile.native_tool_format, ToolFormat::None);
    }

    #[test]
    fn test_probe_ollama_mistral() {
        let profile = LLMCapabilityProfile::from_model_id("mistral-7b", "ollama");
        assert!(profile.supports_tool_use);
        assert_eq!(profile.native_tool_format, ToolFormat::Ollama);
    }

    #[test]
    fn test_probe_gemini() {
        let profile = LLMCapabilityProfile::from_model_id("gemini-2.0-flash", "google");
        assert!(profile.supports_tool_use);
        assert!(profile.supports_parallel_tools);
        assert_eq!(profile.max_context_tokens, 1_000_000);
    }

    #[test]
    fn test_probe_unknown() {
        let profile = LLMCapabilityProfile::from_model_id("my-custom-model", "custom");
        assert!(!profile.supports_tool_use);
        assert_eq!(profile.native_tool_format, ToolFormat::None);
        assert_eq!(profile.max_context_tokens, 4_096);
    }

    #[test]
    fn test_has_native_tools() {
        let claude = LLMCapabilityProfile::from_model_id("claude-opus-4-6", "anthropic");
        assert!(claude.has_native_tools());

        let llama = LLMCapabilityProfile::from_model_id("llama3", "ollama");
        assert!(!llama.has_native_tools());
    }

    #[test]
    fn test_suggested_max_tokens() {
        let claude = LLMCapabilityProfile::from_model_id("claude-sonnet-4-6", "anthropic");
        let complex = claude.suggested_max_tokens(true);
        let simple = claude.suggested_max_tokens(false);
        assert!(complex > simple);
        assert!(complex <= 8_192);
        assert_eq!(simple, 1_024);
    }

    #[test]
    fn test_suggested_temperature() {
        let profile = LLMCapabilityProfile::from_model_id("claude-sonnet-4-6", "anthropic");
        assert!((profile.suggested_temperature(true) - 0.3).abs() < f64::EPSILON);
        assert!((profile.suggested_temperature(false) - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_tool_format_serialization() {
        let json = serde_json::to_string(&ToolFormat::Anthropic).unwrap();
        assert_eq!(json, "\"anthropic\"");
        let back: ToolFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ToolFormat::Anthropic);
    }
}
