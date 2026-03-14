//! DecideEngine — shared autonomy + gate + anomaly detection state.
//!
//! Evaluates commands through the full 6-layer security pipeline:
//! anomaly detection, boundary enforcement, risk assessment, gate decision.

use hydra_autonomy::{ActionRisk, AutonomyLevel, GraduatedAutonomy, TrustDomain};
use hydra_core::types::{Action, ActionType};
use hydra_gate::boundary::{BoundaryEnforcer, BoundaryResult};
use hydra_gate::risk::{ActionContext, RiskAssessor};
use hydra_gate::{ExecutionGate, GateConfig, GateDecision};
use std::sync::Arc;

use super::decide_anomaly::{AnomalyDetector, CommandGateResult, DecideResult};

/// Shared autonomy + gate + anomaly detection state
pub struct DecideEngine {
    autonomy: Arc<GraduatedAutonomy>,
    gate: Arc<ExecutionGate>,
    boundary: BoundaryEnforcer,
    risk_assessor: RiskAssessor,
    anomaly: AnomalyDetector,
}

impl DecideEngine {
    pub fn new() -> Self {
        // Start at Partner ceiling — user can adjust
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Partner);
        let gate = ExecutionGate::new(GateConfig::default());
        Self {
            autonomy: Arc::new(autonomy),
            gate: Arc::new(gate),
            boundary: BoundaryEnforcer::new(),
            risk_assessor: RiskAssessor::new(),
            anomaly: AnomalyDetector::new(),
        }
    }

    /// Map text-based risk assessment to ActionRisk enum
    fn map_risk(risk_str: &str) -> ActionRisk {
        match risk_str {
            "critical" => ActionRisk::Critical,
            "high" => ActionRisk::High,
            "medium" => ActionRisk::Medium,
            "low" => ActionRisk::Low,
            _ => ActionRisk::None,
        }
    }

    /// SAFETY-CRITICAL: Hardcoded command classification for risk assessment.
    /// These MUST remain hardcoded — they're safety rails, not intelligence.
    /// rm, sudo, git, curl, etc. must always be classified regardless of LLM availability.
    /// Maps shell commands to ActionType variants for the risk scoring pipeline.
    pub(crate) fn classify_command(cmd: &str) -> ActionType {
        let lower = cmd.to_lowercase();
        if lower.starts_with("rm ") || lower.contains("unlink ") || lower.contains("trash ") {
            ActionType::FileDelete
        } else if lower.starts_with("sudo ") || lower.starts_with("systemctl ")
            || lower.starts_with("launchctl ") || lower.starts_with("chmod ")
            || lower.starts_with("chown ")
        {
            ActionType::System
        } else if lower.starts_with("git ") {
            ActionType::GitOperation
        } else if lower.contains(" > ") || lower.starts_with("cat >")
            || lower.starts_with("tee ")
        {
            ActionType::Write
        } else if lower.starts_with("cat ") || lower.starts_with("less ") || lower.starts_with("head ")
            || lower.starts_with("tail ") || lower.starts_with("ls ") || lower.starts_with("find ")
            || lower.starts_with("wc ") || lower.starts_with("file ") || lower.starts_with("stat ")
            || lower.starts_with("du ") || lower.starts_with("df ") || lower.starts_with("pwd")
            || lower.starts_with("which ") || lower.starts_with("whereis ")
            || lower.starts_with("whoami") || lower.starts_with("date") || lower.starts_with("uname")
            || lower.starts_with("echo '") || lower.starts_with("echo \"")
            || lower.starts_with("echo =") // echo headers like "=== Top Stories ==="
            || lower.starts_with("ps ") || lower.starts_with("top ") || lower.starts_with("uptime")
        {
            ActionType::Read
        } else if lower.starts_with("open ") || lower.starts_with("xdg-open ")
            || lower.starts_with("osascript ") || lower.starts_with("start ")
            || lower.starts_with("pbcopy") || lower.starts_with("pbpaste")
        {
            // macOS/Linux app launching and clipboard — low risk
            ActionType::Execute
        } else if lower.starts_with("curl ") || lower.starts_with("wget ") {
            // Network requests — distinguish read-only fetches from uploads
            if lower.contains("-x ") || lower.contains("--data") || lower.contains("-d ")
                || lower.contains("--upload") || lower.contains("post")
            {
                ActionType::Network // Mutating network call
            } else {
                ActionType::ApiCall // Read-only fetch (curl -s for data)
            }
        } else if lower.starts_with("mkdir ") || lower.starts_with("touch ") || lower.starts_with("cp ") {
            ActionType::FileCreate
        } else if lower.starts_with("npm ") || lower.starts_with("yarn ") || lower.starts_with("pnpm ")
            || lower.starts_with("cargo ") || lower.starts_with("pip ") || lower.starts_with("python3 -c")
            || lower.starts_with("python -c") || lower.starts_with("node -e")
            || lower.starts_with("brew ") || lower.starts_with("apt ")
        {
            ActionType::Execute
        } else {
            ActionType::ShellExecute
        }
    }

    /// Check whether an action should proceed (trust-based, for overall request)
    pub fn check(&self, risk_str: &str, domain_str: &str) -> DecideResult {
        let risk = Self::map_risk(risk_str);
        let domain = if domain_str.is_empty() {
            TrustDomain::global()
        } else {
            TrustDomain::new(domain_str)
        };

        let decision = self.autonomy.check_action(&domain, risk);

        DecideResult {
            allowed: decision.allowed,
            requires_approval: decision.requires_approval,
            autonomy_level: decision.autonomy_level,
            trust_score: decision.trust_score,
            risk_level: risk,
            reason: decision.reason,
        }
    }

    /// Evaluate a SPECIFIC COMMAND through the full security pipeline:
    /// Anomaly detection → Boundary enforcement → Risk assessment → Gate decision
    pub fn evaluate_command(&self, command: &str) -> CommandGateResult {
        // ── Layer 0: Anomaly detection (burst, exfiltration, destructive patterns) ──
        if let Some(anomaly_reason) = self.anomaly.check(command) {
            return CommandGateResult {
                allowed: false,
                risk_score: 1.0,
                risk_level: "critical".into(),
                reason: anomaly_reason,
                boundary_blocked: false,
                anomaly_detected: true,
            };
        }

        // ── Layer 1: Boundary enforcement (hard blocks on system paths) ──
        match self.boundary.check(command) {
            BoundaryResult::Blocked(violation) => {
                return CommandGateResult {
                    allowed: false,
                    risk_score: 1.0,
                    risk_level: "critical".into(),
                    reason: format!("Boundary violation: {}", violation),
                    boundary_blocked: true,
                    anomaly_detected: false,
                };
            }
            BoundaryResult::Allowed => {}
        }

        // ── Layer 2: Risk assessment (weighted scoring) ──
        let action_type = Self::classify_command(command);
        let action = Action::new(action_type, command);
        let context = ActionContext {
            target_path: Some(command.to_string()),
            is_hydra_internal: command.contains("hydra-gate") || command.contains("hydra-core")
                || command.contains("hydra-kernel"),
            in_sandbox: false, // shell commands run outside sandbox
            has_backup: false,
        };
        let assessment = self.risk_assessor.assess_risk_fast(&action, &context);
        let risk_score = RiskAssessor::risk_score(&assessment);

        // ── Layer 3: Decision thresholds — warn only, never block ──
        let (allowed, requires_approval) = if risk_score >= 0.9 {
            eprintln!("[hydra:risk] ⚠ CRITICAL risk {:.2} — proceeding with warning", risk_score);
            (true, false) // Warn but allow
        } else if risk_score >= 0.5 {
            eprintln!("[hydra:risk] ⚠ Elevated risk {:.2} — proceeding", risk_score);
            (true, false) // Warn but allow
        } else {
            (true, false) // Auto-approve
        };

        let risk_level = if risk_score >= 0.9 {
            "critical"
        } else if risk_score >= 0.7 {
            "high"
        } else if risk_score >= 0.5 {
            "medium"
        } else if risk_score >= 0.3 {
            "low"
        } else {
            "none"
        };

        let reason = format!("Risk: {:.2} ({}). {}", risk_score, risk_level,
            assessment.factors.iter().map(|f| f.description.clone()).collect::<Vec<_>>().join("; "));

        CommandGateResult {
            allowed: allowed || requires_approval, // requires_approval still proceeds after UI approval
            risk_score,
            risk_level: risk_level.into(),
            reason,
            boundary_blocked: false,
            anomaly_detected: false,
        }
    }

    /// Evaluate a command through the async ExecutionGate (full 6-layer pipeline)
    pub async fn evaluate_command_full(&self, command: &str) -> GateDecision {
        let action_type = Self::classify_command(command);
        let action = Action::new(action_type, command);
        let context = ActionContext {
            target_path: Some(command.to_string()),
            is_hydra_internal: command.contains("hydra-gate") || command.contains("hydra-core"),
            in_sandbox: false,
            has_backup: false,
        };
        self.gate.evaluate(&action, &context, None).await
    }

    /// Record a successful action (earns trust)
    pub fn record_success(&self, risk_str: &str, domain_str: &str) {
        let risk = Self::map_risk(risk_str);
        let domain = if domain_str.is_empty() {
            TrustDomain::global()
        } else {
            TrustDomain::new(domain_str)
        };
        self.autonomy.record_success(&domain, risk);
    }

    /// Record a failed action (loses trust)
    pub fn record_failure(&self, risk_str: &str, domain_str: &str) {
        let risk = Self::map_risk(risk_str);
        let domain = if domain_str.is_empty() {
            TrustDomain::global()
        } else {
            TrustDomain::new(domain_str)
        };
        self.autonomy.record_failure(&domain, risk);
    }

    /// Get current autonomy level for display
    pub fn current_level(&self) -> AutonomyLevel {
        self.autonomy.autonomy_level(&TrustDomain::global())
    }

    /// Get current trust score for display
    pub fn current_trust(&self) -> f64 {
        self.autonomy
            .trust_score(&TrustDomain::global())
            .map(|s| s.value)
            .unwrap_or(0.0)
    }

    /// Get a reference to the execution gate
    pub fn gate(&self) -> &ExecutionGate {
        &self.gate
    }

    /// Get anomaly detector stats (total_commands, distinct_paths)
    pub fn anomaly_stats(&self) -> (u64, usize) {
        self.anomaly.stats()
    }

    /// Activate the kill switch — immediately blocks ALL future actions
    pub fn kill_switch_engage(&self, reason: &str) {
        self.gate.kill_switch().instant_halt(reason, "hydra_anomaly_detector");
    }

    /// Check if the kill switch is engaged
    pub fn is_halted(&self) -> bool {
        self.gate.kill_switch().is_halted()
    }

    /// Phase 2, D3: Compound Risk Scoring
    /// Evaluates multiple commands together, accounting for cumulative risk.
    /// Multiple medium-risk commands together are higher risk than any single one.
    /// Returns (compound_risk_score, compound_risk_level, details).
    pub fn compound_risk_score(&self, commands: &[String]) -> (f64, String, String) {
        if commands.is_empty() {
            return (0.0, "none".to_string(), "No commands".to_string());
        }
        if commands.len() == 1 {
            let result = self.evaluate_command(&commands[0]);
            return (result.risk_score, result.risk_level, result.reason);
        }

        let mut total_risk = 0.0;
        let mut max_single_risk = 0.0_f64;
        let mut details = Vec::new();
        let mut has_network = false;
        let mut has_filesystem_write = false;
        let mut has_destructive = false;

        for cmd in commands {
            let result = self.evaluate_command(cmd);
            total_risk += result.risk_score;
            max_single_risk = max_single_risk.max(result.risk_score);
            details.push(format!("{}: {:.2}", safe_truncate_static(cmd, 30), result.risk_score));

            let lower = cmd.to_lowercase();
            if lower.contains("curl") || lower.contains("wget") || lower.contains("ssh") {
                has_network = true;
            }
            if lower.contains(" > ") || lower.starts_with("echo ") || lower.starts_with("cat >") {
                has_filesystem_write = true;
            }
            if lower.contains("rm ") || lower.contains("drop ") || lower.contains("delete") {
                has_destructive = true;
            }
        }

        // Compound risk: average + escalation for mixed dangerous operations
        let count = commands.len() as f64;
        let mut compound = (total_risk / count) * 0.6 + max_single_risk * 0.4;

        // Escalation: network + filesystem is suspicious
        if has_network && has_filesystem_write {
            compound += 0.15;
        }
        // Escalation: destructive + network is very suspicious
        if has_destructive && has_network {
            compound += 0.2;
        }
        // Escalation: many commands = higher risk
        if count > 5.0 {
            compound += 0.05 * (count - 5.0).min(5.0);
        }

        let compound = compound.min(1.0);
        let level = if compound >= 0.9 {
            "critical"
        } else if compound >= 0.7 {
            "high"
        } else if compound >= 0.5 {
            "medium"
        } else if compound >= 0.3 {
            "low"
        } else {
            "none"
        };

        (compound, level.to_string(), details.join("; "))
    }
}

/// Static-lifetime-free version of safe_truncate for decide.rs
fn safe_truncate_static(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

impl Default for DecideEngine {
    fn default() -> Self {
        Self::new()
    }
}
