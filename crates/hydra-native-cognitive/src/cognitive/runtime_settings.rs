//! Runtime behavior/policy settings passed from the UI to the cognitive loop.

/// Runtime behavior/policy settings from the desktop/TUI UI.
/// All fields have sensible defaults for backward compatibility.
#[derive(Debug, Clone)]
pub struct RuntimeSettings {
    pub intent_cache: bool,
    pub cache_ttl: String,
    pub learn_corrections: bool,
    pub belief_persist: String,
    pub compression: String,
    pub dispatch_mode: String,
    pub sister_timeout: String,
    pub retry_failures: bool,
    pub dream_state: bool,
    pub proactive: bool,
    pub risk_threshold: String,
    pub file_write: bool,
    pub network_access: bool,
    pub shell_exec: bool,
    pub max_file_edits: String,
    pub require_approval_critical: bool,
    pub sandbox_mode: bool,
    pub debug_mode: bool,
    pub log_level: String,
    pub federation_enabled: bool,
    pub memory_capture: String,
    /// Enable multi-turn agentic loop (tool results fed back to LLM)
    pub agentic_loop: bool,
    /// Max turns for agentic loop (default 8, max 15)
    pub agentic_max_turns: u8,
    /// Token budget for agentic loop (default 50000)
    pub agentic_token_budget: u64,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            intent_cache: true,
            cache_ttl: "1h".into(),
            learn_corrections: true,
            belief_persist: "7 days".into(),
            compression: "Balanced".into(),
            dispatch_mode: "Parallel".into(),
            sister_timeout: "10s".into(),
            retry_failures: true,
            dream_state: true,
            proactive: true,
            risk_threshold: "medium".into(),
            file_write: true,
            network_access: true,
            shell_exec: true,
            max_file_edits: "25".into(),
            require_approval_critical: true,
            sandbox_mode: false,
            debug_mode: false,
            log_level: "info".into(),
            federation_enabled: false,
            memory_capture: "all".into(),
            agentic_loop: true,
            agentic_max_turns: 8,
            agentic_token_budget: 50_000,
        }
    }
}

impl RuntimeSettings {
    /// Whether memory capture is set to "all" (full immortal + comm trail)
    pub fn should_capture_all(&self) -> bool {
        self.memory_capture == "all"
    }

    /// Whether memory capture allows at least facts (anything except "none")
    pub fn should_capture_facts(&self) -> bool {
        self.memory_capture != "none"
    }

    /// Parse sister timeout to milliseconds
    pub fn sister_timeout_ms(&self) -> u64 {
        match self.sister_timeout.as_str() {
            "5s" => 5000, "10s" => 10000, "30s" => 30000, "60s" => 60000,
            _ => 10000,
        }
    }

    /// Parse max file edits to a number
    pub fn max_file_edits_num(&self) -> usize {
        match self.max_file_edits.as_str() {
            "5" => 5, "10" => 10, "25" => 25, "50" => 50, "unlimited" => usize::MAX,
            _ => 25,
        }
    }

    /// Whether the risk threshold allows auto-approval at a given level
    pub fn auto_approve_risk(&self, level: &str) -> bool {
        match self.risk_threshold.as_str() {
            "none" => false,
            "low" => level == "low",
            "medium" => level == "low" || level == "medium",
            "high" => level != "critical",
            _ => level == "low",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let s = RuntimeSettings::default();
        assert!(s.intent_cache);
        assert!(s.file_write);
        assert_eq!(s.sister_timeout_ms(), 10000);
        assert_eq!(s.max_file_edits_num(), 25);
    }

    #[test]
    fn auto_approve_risk_levels() {
        let s = RuntimeSettings { risk_threshold: "medium".into(), ..Default::default() };
        assert!(s.auto_approve_risk("low"));
        assert!(s.auto_approve_risk("medium"));
        assert!(!s.auto_approve_risk("high"));
        assert!(!s.auto_approve_risk("critical"));
    }

    #[test]
    fn timeout_parsing() {
        let s = RuntimeSettings { sister_timeout: "30s".into(), ..Default::default() };
        assert_eq!(s.sister_timeout_ms(), 30000);
    }
}
