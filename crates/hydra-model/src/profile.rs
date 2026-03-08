use serde::{Deserialize, Serialize};

/// Privacy level — determines where data can be sent
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyLevel {
    /// No network, fully isolated
    AirGapped,
    /// Runs on user's machine
    Local,
    /// Data sent to cloud provider
    Cloud,
}

/// Task type for capability matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Code,
    Reasoning,
    Creative,
    Math,
    Conversation,
    Vision,
    General,
}

/// Model capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub reasoning: u8,
    pub code: u8,
    pub creative: u8,
    pub math: u8,
    pub instruction_following: u8,
    pub vision: bool,
    pub function_calling: bool,
    pub context_window: u32,
    pub max_output_tokens: u32,
}

impl ModelCapabilities {
    /// Score for a specific task type (0–100)
    pub fn score_for_task(&self, task: TaskType) -> u8 {
        match task {
            TaskType::Code => self.code,
            TaskType::Reasoning => self.reasoning,
            TaskType::Creative => self.creative,
            TaskType::Math => self.math,
            TaskType::Conversation => self.instruction_following,
            TaskType::Vision => {
                if self.vision {
                    90
                } else {
                    0
                }
            }
            TaskType::General => {
                ((self.reasoning as u16 + self.code as u16 + self.instruction_following as u16) / 3)
                    as u8
            }
        }
    }
}

/// Model profile with all metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub capabilities: ModelCapabilities,
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
    pub latency_ms: u32,
    pub privacy: PrivacyLevel,
    pub available: bool,
    pub rate_limited: bool,
}

impl ModelProfile {
    /// Combined cost per 1K tokens (average of input + output)
    pub fn cost_per_1k(&self) -> f64 {
        (self.cost_per_1k_input + self.cost_per_1k_output) / 2.0
    }

    /// Is this model usable right now?
    pub fn is_usable(&self) -> bool {
        self.available && !self.rate_limited
    }
}

// ═══════════════════════════════════════════════════════════
// BUILT-IN MODEL PROFILES
// ═══════════════════════════════════════════════════════════

pub fn builtin_profiles() -> Vec<ModelProfile> {
    vec![
        ModelProfile {
            id: "claude-opus".into(),
            name: "Claude Opus 4.6".into(),
            provider: "anthropic".into(),
            capabilities: ModelCapabilities {
                reasoning: 98,
                code: 95,
                creative: 92,
                math: 95,
                instruction_following: 97,
                vision: true,
                function_calling: true,
                context_window: 200_000,
                max_output_tokens: 32_000,
            },
            cost_per_1k_input: 0.015,
            cost_per_1k_output: 0.075,
            latency_ms: 2000,
            privacy: PrivacyLevel::Cloud,
            available: true,
            rate_limited: false,
        },
        ModelProfile {
            id: "claude-sonnet".into(),
            name: "Claude Sonnet 4.6".into(),
            provider: "anthropic".into(),
            capabilities: ModelCapabilities {
                reasoning: 92,
                code: 93,
                creative: 88,
                math: 90,
                instruction_following: 95,
                vision: true,
                function_calling: true,
                context_window: 200_000,
                max_output_tokens: 16_000,
            },
            cost_per_1k_input: 0.003,
            cost_per_1k_output: 0.015,
            latency_ms: 1000,
            privacy: PrivacyLevel::Cloud,
            available: true,
            rate_limited: false,
        },
        ModelProfile {
            id: "claude-haiku".into(),
            name: "Claude Haiku 4.5".into(),
            provider: "anthropic".into(),
            capabilities: ModelCapabilities {
                reasoning: 80,
                code: 82,
                creative: 78,
                math: 78,
                instruction_following: 88,
                vision: true,
                function_calling: true,
                context_window: 200_000,
                max_output_tokens: 8_000,
            },
            cost_per_1k_input: 0.001,
            cost_per_1k_output: 0.005,
            latency_ms: 500,
            privacy: PrivacyLevel::Cloud,
            available: true,
            rate_limited: false,
        },
        ModelProfile {
            id: "gpt-4o".into(),
            name: "GPT-4o".into(),
            provider: "openai".into(),
            capabilities: ModelCapabilities {
                reasoning: 90,
                code: 88,
                creative: 85,
                math: 88,
                instruction_following: 92,
                vision: true,
                function_calling: true,
                context_window: 128_000,
                max_output_tokens: 16_000,
            },
            cost_per_1k_input: 0.005,
            cost_per_1k_output: 0.015,
            latency_ms: 1200,
            privacy: PrivacyLevel::Cloud,
            available: true,
            rate_limited: false,
        },
        ModelProfile {
            id: "gpt-4o-mini".into(),
            name: "GPT-4o Mini".into(),
            provider: "openai".into(),
            capabilities: ModelCapabilities {
                reasoning: 78,
                code: 75,
                creative: 72,
                math: 75,
                instruction_following: 85,
                vision: true,
                function_calling: true,
                context_window: 128_000,
                max_output_tokens: 16_000,
            },
            cost_per_1k_input: 0.00015,
            cost_per_1k_output: 0.0006,
            latency_ms: 400,
            privacy: PrivacyLevel::Cloud,
            available: true,
            rate_limited: false,
        },
        ModelProfile {
            id: "llama-3-70b".into(),
            name: "Llama 3 70B".into(),
            provider: "local".into(),
            capabilities: ModelCapabilities {
                reasoning: 82,
                code: 80,
                creative: 75,
                math: 80,
                instruction_following: 85,
                vision: false,
                function_calling: false,
                context_window: 8_000,
                max_output_tokens: 4_000,
            },
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            latency_ms: 3000,
            privacy: PrivacyLevel::Local,
            available: true,
            rate_limited: false,
        },
        ModelProfile {
            id: "deepseek-coder-v2".into(),
            name: "DeepSeek Coder V2".into(),
            provider: "deepseek".into(),
            capabilities: ModelCapabilities {
                reasoning: 75,
                code: 90,
                creative: 60,
                math: 82,
                instruction_following: 80,
                vision: false,
                function_calling: true,
                context_window: 128_000,
                max_output_tokens: 8_000,
            },
            cost_per_1k_input: 0.00014,
            cost_per_1k_output: 0.00028,
            latency_ms: 800,
            privacy: PrivacyLevel::Cloud,
            available: true,
            rate_limited: false,
        },
    ]
}
