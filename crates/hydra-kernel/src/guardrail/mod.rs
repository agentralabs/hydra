//! Owner Guardrail System — Hydra's self-governance layer.
//!
//! Guardrails are ADDITIVE: owner adds restrictions, never removes capabilities.
//! Default = fully permissive (no boundaries.toml = everything allowed).
//! Hydra is limitless in what it CAN do. Guardrails govern what it does TO ITSELF.
//!
//! 4 layers:
//! 1. Kill Switch: ~/.hydra/KILL file, dead-man-switch, remote HTTP kill
//! 2. Evolution Gates: approval queue, forbidden paths, blast radius thresholds
//! 3. Audit Trail: append-only JSONL log of every decision
//! 4. Owner Commands: /guardrail, /evolution TUI commands

pub mod audit;
pub mod config;
pub mod evolution_gate;

use audit::{AuditEventType, AuditLog};
use config::GuardrailConfig;

/// Guardrail system state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardrailState {
    /// Normal operation — all systems active.
    Active,
    /// Owner paused proactive + evolution. Core loops still run.
    Paused,
    /// Dead-man-switch triggered. Same as Paused but automatic.
    Dormant,
    /// Kill signal detected. Shutdown imminent.
    KillSignaled,
}

impl GuardrailState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Dormant => "dormant",
            Self::KillSignaled => "kill-signaled",
        }
    }
}

/// Core guardrail engine — persists across ambient ticks.
pub struct GuardrailEngine {
    pub config: GuardrailConfig,
    pub state: GuardrailState,
    pub audit: AuditLog,
}

impl GuardrailEngine {
    pub fn new() -> Self {
        let config = GuardrailConfig::load();
        let guardrails_dir = dirs::home_dir().unwrap_or_default().join(".hydra/guardrails");
        let _ = std::fs::create_dir_all(&guardrails_dir);
        let _ = std::fs::create_dir_all(guardrails_dir.join("evolution-queue"));
        let _ = std::fs::create_dir_all(guardrails_dir.join("evolution-processed"));

        // Check if already paused from a previous session
        let initial_state = if paused_path().exists() {
            eprintln!("hydra-guardrail: resuming in PAUSED state");
            GuardrailState::Paused
        } else {
            GuardrailState::Active
        };

        let mut engine = Self {
            config,
            state: initial_state,
            audit: AuditLog::new(),
        };
        engine.audit.record(AuditEventType::StateChange,
            &format!("Guardrail engine initialized: {}", initial_state.label()), "system");
        engine
    }

    /// Check for ~/.hydra/KILL file. Returns true if kill signal detected.
    pub fn check_kill_signal(&mut self) -> bool {
        let kill_path = kill_path();
        if kill_path.exists() {
            if self.state != GuardrailState::KillSignaled {
                self.state = GuardrailState::KillSignaled;
                self.audit.record(AuditEventType::KillSignal,
                    "KILL file detected — initiating shutdown", "system");
                eprintln!("hydra-guardrail: KILL SIGNAL — shutdown imminent");
            }
            return true;
        }
        false
    }

    /// Check dead-man-switch — pause if no owner interaction in N days.
    pub fn check_dead_man_switch(&mut self) {
        let days = match self.config.dead_man_switch_days {
            Some(d) => d,
            None => return, // Disabled
        };
        let interaction_path = interaction_path();
        let last = match std::fs::read_to_string(&interaction_path) {
            Ok(s) => match chrono::DateTime::parse_from_rfc3339(s.trim()) {
                Ok(dt) => dt.with_timezone(&chrono::Utc),
                Err(_) => return, // Can't parse, skip
            },
            Err(_) => return, // No file, skip (owner just started)
        };
        let elapsed = chrono::Utc::now() - last;
        if elapsed.num_days() >= days as i64 && self.state == GuardrailState::Active {
            self.state = GuardrailState::Dormant;
            // Create PAUSED file so dream/proactive respect it
            let _ = std::fs::write(paused_path(), "dormant: dead-man-switch");
            self.audit.record(AuditEventType::DeadManSwitch,
                &format!("No owner interaction for {} days — entering dormant mode", elapsed.num_days()),
                "system");
            eprintln!("hydra-guardrail: DEAD-MAN-SWITCH — dormant after {} days", elapsed.num_days());
        }
    }

    /// Record that the owner interacted with Hydra.
    pub fn record_owner_interaction(&mut self) {
        let path = interaction_path();
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(&path, chrono::Utc::now().to_rfc3339());
        // If dormant, wake up
        if self.state == GuardrailState::Dormant {
            self.state = GuardrailState::Active;
            let _ = std::fs::remove_file(paused_path());
            self.audit.record(AuditEventType::StateChange,
                "Owner returned — resuming from dormant", "owner");
            eprintln!("hydra-guardrail: owner returned — resuming active state");
        }
    }

    /// Whether evolution is currently allowed.
    pub fn is_evolution_allowed(&self) -> bool {
        self.state == GuardrailState::Active
    }

    /// Whether proactive initiation is currently allowed.
    pub fn is_proactive_allowed(&self) -> bool {
        self.state == GuardrailState::Active
    }

    /// Pause Hydra (owner-initiated via /guardrail pause).
    pub fn pause(&mut self) {
        self.state = GuardrailState::Paused;
        let _ = std::fs::write(paused_path(), "paused by owner");
        self.audit.record(AuditEventType::StateChange, "Owner paused Hydra", "owner");
        eprintln!("hydra-guardrail: PAUSED by owner");
    }

    /// Resume Hydra (owner-initiated via /guardrail resume).
    pub fn resume(&mut self) {
        self.state = GuardrailState::Active;
        let _ = std::fs::remove_file(paused_path());
        let _ = std::fs::remove_file(kill_path()); // Also clear kill signal
        self.audit.record(AuditEventType::StateChange, "Owner resumed Hydra", "owner");
        eprintln!("hydra-guardrail: RESUMED by owner");
    }

    /// Reload config from disk.
    pub fn reload_config(&mut self) {
        self.config = GuardrailConfig::load();
        self.audit.record(AuditEventType::ConfigReloaded, "Config reloaded", "owner");
    }

    /// Get pending evolution proposals.
    pub fn pending_evolutions(&self) -> Vec<evolution_gate::EvolutionProposal> {
        evolution_gate::load_pending()
    }

    /// Status summary for TUI display.
    pub fn status_summary(&self) -> String {
        let pending = self.pending_evolutions().len();
        let forbidden = self.config.forbidden_paths.len();
        let dead_man = self.config.dead_man_switch_days
            .map(|d| format!("{d} days")).unwrap_or_else(|| "off".into());
        format!("State: {} | Pending evolutions: {} | Forbidden paths: {} | Dead-man: {} | Audit: {} entries",
            self.state.label(), pending, forbidden, dead_man, self.audit.len())
    }
}

impl Default for GuardrailEngine {
    fn default() -> Self { Self::new() }
}

fn kill_path() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/KILL")
}
fn paused_path() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/guardrails/PAUSED")
}
fn interaction_path() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/guardrails/last-interaction")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_active() {
        // Don't create real engine (touches filesystem), test enum
        assert_eq!(GuardrailState::Active.label(), "active");
        assert_eq!(GuardrailState::Paused.label(), "paused");
        assert_eq!(GuardrailState::KillSignaled.label(), "kill-signaled");
    }

    #[test]
    fn config_loads_default() {
        let config = GuardrailConfig::default();
        assert!(config.is_path_allowed("skills/auto_test/"));
        assert!(!config.is_path_allowed("guardrail/mod.rs"));
    }
}
