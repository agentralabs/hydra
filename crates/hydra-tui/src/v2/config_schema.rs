//! Config schema — single source of truth for all settings.
//! Every setting has a name, type, default, description, and optional enum values.
//! Used by both /settings command and the inline config editor modal.

/// Type of a config value.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    String,
    Bool,
    Float,
    Int,
    Enum(&'static [&'static str]),
}

/// Schema for one config setting.
#[derive(Debug, Clone)]
pub struct ConfigSchema {
    pub key: &'static str,
    pub section: &'static str,
    pub description: &'static str,
    pub value_type: ValueType,
    pub default: &'static str,
}

impl ConfigSchema {
    /// Validate a value against this schema.
    pub fn validate(&self, value: &str) -> Result<(), String> {
        match &self.value_type {
            ValueType::Bool => {
                if !["true", "false", "0", "1"].contains(&value) {
                    return Err(format!("Expected true/false, got '{value}'"));
                }
            }
            ValueType::Float => {
                if value.parse::<f64>().is_err() {
                    return Err(format!("Expected number, got '{value}'"));
                }
            }
            ValueType::Int => {
                if value.parse::<i64>().is_err() {
                    return Err(format!("Expected integer, got '{value}'"));
                }
            }
            ValueType::Enum(options) => {
                if !options.contains(&value) {
                    return Err(format!(
                        "Expected one of [{}], got '{value}'",
                        options.join(", ")
                    ));
                }
            }
            ValueType::String => {} // anything goes
        }
        Ok(())
    }
}

/// All config schemas — the complete settings surface.
pub fn all_schemas() -> Vec<ConfigSchema> {
    vec![
        // TUI settings
        ConfigSchema {
            key: "theme",
            section: "tui",
            description: "Color theme",
            value_type: ValueType::Enum(&["dark", "light", "auto"]),
            default: "auto",
        },
        ConfigSchema {
            key: "markdown",
            section: "tui",
            description: "Render markdown in responses",
            value_type: ValueType::Bool,
            default: "true",
        },
        ConfigSchema {
            key: "streaming",
            section: "tui",
            description: "Stream responses token-by-token",
            value_type: ValueType::Bool,
            default: "true",
        },
        ConfigSchema {
            key: "pacer_speed",
            section: "tui",
            description: "Output speed multiplier (0.5 = slow, 2.0 = fast)",
            value_type: ValueType::Float,
            default: "1.0",
        },
        ConfigSchema {
            key: "max_history",
            section: "tui",
            description: "Maximum input history entries",
            value_type: ValueType::Int,
            default: "100",
        },
        ConfigSchema {
            key: "show_tool_dots",
            section: "tui",
            description: "Show tool use indicators in stream",
            value_type: ValueType::Bool,
            default: "true",
        },
        ConfigSchema {
            key: "show_enrichments",
            section: "tui",
            description: "Show genome/memory/calibration enrichments",
            value_type: ValueType::Bool,
            default: "true",
        },
        ConfigSchema {
            key: "placeholder",
            section: "tui",
            description: "Input placeholder text",
            value_type: ValueType::String,
            default: "What are we building today?",
        },
        // Voice settings
        ConfigSchema {
            key: "voice_mode",
            section: "voice",
            description: "Voice activation mode",
            value_type: ValueType::Enum(&["push-to-talk", "always-on", "disabled"]),
            default: "push-to-talk",
        },
        ConfigSchema {
            key: "voice_enabled",
            section: "voice",
            description: "Enable voice system",
            value_type: ValueType::Bool,
            default: "false",
        },
        // Companion settings
        ConfigSchema {
            key: "companion_enabled",
            section: "companion",
            description: "Enable companion signal system",
            value_type: ValueType::Bool,
            default: "false",
        },
        ConfigSchema {
            key: "companion_autonomy",
            section: "companion",
            description: "Default autonomy level for companion tasks",
            value_type: ValueType::Enum(&["report", "confirm", "summarize", "auto"]),
            default: "confirm",
        },
        // LLM settings
        ConfigSchema {
            key: "provider",
            section: "llm",
            description: "LLM provider",
            value_type: ValueType::Enum(&["anthropic", "openai", "ollama", "gemini"]),
            default: "anthropic",
        },
        ConfigSchema {
            key: "model",
            section: "llm",
            description: "LLM model name",
            value_type: ValueType::String,
            default: "claude-sonnet-4-20250514",
        },
        // Session settings
        ConfigSchema {
            key: "auto_save",
            section: "session",
            description: "Auto-save conversations",
            value_type: ValueType::Bool,
            default: "true",
        },
        ConfigSchema {
            key: "auto_backup_days",
            section: "backup",
            description: "Days between automatic backups (0 = disabled)",
            value_type: ValueType::Int,
            default: "7",
        },
    ]
}

/// Find a schema by key.
pub fn find_schema(_key: &str) -> Option<&'static ConfigSchema> {
    // Leak the vec to get 'static refs — only called once
    // In practice, use a lazy static or just search each time
    None // Placeholder — search all_schemas() at call site
}

/// Get schemas grouped by section.
pub fn schemas_by_section() -> Vec<(&'static str, Vec<ConfigSchema>)> {
    let schemas = all_schemas();
    let mut sections: std::collections::BTreeMap<&str, Vec<ConfigSchema>> =
        std::collections::BTreeMap::new();
    for schema in schemas {
        sections
            .entry(schema.section)
            .or_default()
            .push(schema);
    }
    sections.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_schemas_non_empty() {
        assert!(all_schemas().len() >= 10);
    }

    #[test]
    fn validate_bool() {
        let schema = ConfigSchema {
            key: "test",
            section: "test",
            description: "test",
            value_type: ValueType::Bool,
            default: "true",
        };
        assert!(schema.validate("true").is_ok());
        assert!(schema.validate("false").is_ok());
        assert!(schema.validate("maybe").is_err());
    }

    #[test]
    fn validate_enum() {
        let schema = ConfigSchema {
            key: "test",
            section: "test",
            description: "test",
            value_type: ValueType::Enum(&["dark", "light"]),
            default: "dark",
        };
        assert!(schema.validate("dark").is_ok());
        assert!(schema.validate("neon").is_err());
    }

    #[test]
    fn validate_float() {
        let schema = ConfigSchema {
            key: "test",
            section: "test",
            description: "test",
            value_type: ValueType::Float,
            default: "1.0",
        };
        assert!(schema.validate("2.5").is_ok());
        assert!(schema.validate("abc").is_err());
    }

    #[test]
    fn sections_grouped() {
        let grouped = schemas_by_section();
        assert!(!grouped.is_empty());
        // Should have at least tui, voice, companion sections
        let section_names: Vec<&str> = grouped.iter().map(|(k, _)| *k).collect();
        assert!(section_names.contains(&"tui"));
    }
}
