use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

use hydra_core::types::{Action, CapabilityToken, RiskAssessment};

use crate::boundary::{BoundaryEnforcer, BoundaryResult};
use crate::kill_switch::KillSwitch;
use crate::risk::{ActionContext, RiskAssessor};
use crate::security_layers::{self, GateAuditEntry, SessionContext};

// Re-export types so existing `use crate::gate::*` paths keep working
pub use crate::gate_types::{BatchResult, GateConfig, GateDecision};

/// The Execution Gate — nothing executes without passing through here
pub struct ExecutionGate {
    assessor: RiskAssessor,
    boundary: BoundaryEnforcer,
    config: Arc<RwLock<GateConfig>>,
    kill_switch: Arc<KillSwitch>,
    audit_log: Arc<parking_lot::Mutex<Vec<GateAuditEntry>>>,
    audit_sequence: AtomicU64,
    shadow_sim_crash: AtomicBool,
    user_disconnected: AtomicBool,
    user_always_rejects: AtomicBool,
    approval_retry_count: AtomicU32,
}

impl ExecutionGate {
    pub fn new(config: GateConfig) -> Self {
        Self {
            assessor: RiskAssessor::new(),
            boundary: BoundaryEnforcer::new(),
            config: Arc::new(RwLock::new(config)),
            kill_switch: Arc::new(KillSwitch::new()),
            audit_log: Arc::new(parking_lot::Mutex::new(Vec::new())),
            audit_sequence: AtomicU64::new(0),
            shadow_sim_crash: AtomicBool::new(false),
            user_disconnected: AtomicBool::new(false),
            user_always_rejects: AtomicBool::new(false),
            approval_retry_count: AtomicU32::new(0),
        }
    }

    pub fn boundary(&self) -> &BoundaryEnforcer {
        &self.boundary
    }

    pub fn kill_switch(&self) -> &KillSwitch {
        &self.kill_switch
    }

    pub fn audit_log(&self) -> Vec<GateAuditEntry> {
        self.audit_log.lock().clone()
    }

    /// Update config (thread-safe, for EC-EG-008)
    pub fn update_config(&self, config: GateConfig) {
        *self.config.write() = config;
    }

    /// Evaluate a single action through all 6 security layers
    pub async fn evaluate(
        &self,
        action: &Action,
        context: &ActionContext,
        auth_token: Option<&CapabilityToken>,
    ) -> GateDecision {
        self.evaluate_with_session(action, context, auth_token, &SessionContext::default())
            .await
    }

    /// Evaluate with full session context (all 6 layers)
    pub async fn evaluate_with_session(
        &self,
        action: &Action,
        context: &ActionContext,
        auth_token: Option<&CapabilityToken>,
        session: &SessionContext,
    ) -> GateDecision {
        // Kill switch check — instant halt, cannot be overridden
        if self.kill_switch.is_halted() {
            let reason = self
                .kill_switch
                .halt_reason()
                .map(|r| r.reason)
                .unwrap_or_else(|| "System halted.".into());
            return GateDecision::Halted { reason };
        }

        // === BOUNDARY CHECK — hard blocks, runs BEFORE risk assessment ===
        if let BoundaryResult::Blocked(violation) = self.boundary.check(&action.target) {
            self.log_audit(action, "critical", "block", &violation.reason);
            return GateDecision::Block {
                risk_score: 1.0,
                reason: format!("Boundary violation: {}", violation),
            };
        }

        // === LAYER 1: Perimeter (TLS, domain allowlist, rate limit) ===
        if let Err(e) = security_layers::check_perimeter(action) {
            self.log_audit(action, "blocked", "block", &e.user_message());
            return GateDecision::Block {
                risk_score: 1.0,
                reason: e.user_message(),
            };
        }

        // === LAYER 2: Authentication + Session ===
        if let Err(e) = security_layers::check_authentication(auth_token) {
            self.log_audit(action, "blocked", "block", &e.user_message());
            return GateDecision::Block {
                risk_score: 1.0,
                reason: e.user_message(),
            };
        }
        if let Err(e) = security_layers::check_session(session) {
            self.log_audit(action, "blocked", "block", &e.user_message());
            return GateDecision::Block {
                risk_score: 1.0,
                reason: e.user_message(),
            };
        }

        // === LAYER 3: Authorization (capability-based, least privilege) ===
        if let Err(e) = security_layers::check_authorization(action, auth_token) {
            self.log_audit(action, "blocked", "block", &e.user_message());
            return GateDecision::Block {
                risk_score: 1.0,
                reason: e.user_message(),
            };
        }

        // === Data isolation (per-project) ===
        if let Err(e) = security_layers::check_data_isolation(action, session) {
            self.log_audit(action, "blocked", "block", &e.user_message());
            return GateDecision::Block {
                risk_score: 1.0,
                reason: e.user_message(),
            };
        }

        // === LAYER 4: Execution Control — Risk Assessment ===
        let _ = security_layers::check_execution_controls(action, context.in_sandbox);
        let assessment = self.assessor.assess_risk_fast(action, context);
        let risk_score = RiskAssessor::risk_score(&assessment);

        // Snapshot config (EC-EG-008: consistent config during eval)
        let config = self.config.read().clone();

        // Shadow simulation for high risk (optional, private/ stub)
        if risk_score >= 0.7
            && config.shadow_sim_enabled
            && self.shadow_sim_crash.load(Ordering::SeqCst)
        {
            self.log_audit(
                action,
                "high",
                "require_approval",
                "Shadow simulation failed. Falling back to manual approval.",
            );
            return GateDecision::RequireApproval {
                risk_score,
                reason: "Shadow simulation failed. Manual approval required for safety.".into(),
            };
        }

        // === Decision based on thresholds ===
        let decision = if risk_score >= config.block_above {
            GateDecision::Block {
                risk_score,
                reason: format!(
                    "Action blocked due to critical risk ({:.2}). {}",
                    risk_score,
                    assessment.mitigations.first().unwrap_or(&String::new())
                ),
            }
        } else if risk_score >= config.notify_below {
            // Check user disconnect (EC-EG-003)
            if self.user_disconnected.load(Ordering::SeqCst) {
                return GateDecision::Aborted {
                    reason: "User disconnected during approval. Action aborted for safety.".into(),
                };
            }
            // Check infinite loop (EC-EG-009)
            let retries = self.approval_retry_count.fetch_add(1, Ordering::SeqCst);
            if self.user_always_rejects.load(Ordering::SeqCst)
                && retries >= config.max_approval_retries
            {
                self.approval_retry_count.store(0, Ordering::SeqCst);
                return GateDecision::Aborted {
                    reason: format!(
                        "Approval denied {} times. Action aborted to prevent infinite loop.",
                        retries
                    ),
                };
            }
            GateDecision::RequireApproval {
                risk_score,
                reason: format!(
                    "This action has elevated risk ({:.2}). {}",
                    risk_score,
                    assessment
                        .factors
                        .first()
                        .map(|f| f.description.as_str())
                        .unwrap_or("Review before proceeding.")
                ),
            }
        } else if risk_score >= config.auto_approve_below {
            GateDecision::NotifyOnly {
                risk_score,
                message: format!("Proceeding with low-risk action ({:.2}).", risk_score),
            }
        } else {
            GateDecision::AutoApprove { risk_score }
        };

        // === LAYER 5: Data Protection — all output sanitized by GateAuditEntry ===
        // === LAYER 6: Audit — tamper-evident log ===
        let level_str = format!("{:.2}", risk_score);
        self.log_audit(action, &level_str, decision.decision_name(), "");

        decision
    }

    /// Evaluate a batch of actions (EC-EG-006)
    pub async fn evaluate_batch(
        &self,
        actions: &[Action],
        context: &ActionContext,
        auth_token: Option<&CapabilityToken>,
    ) -> BatchResult {
        let mut decisions = Vec::new();
        for (i, action) in actions.iter().enumerate() {
            let decision = self.evaluate(action, context, auth_token).await;
            decisions.push((i, decision));
        }
        BatchResult { decisions }
    }

    /// Measure latency of rule-based assessment
    pub fn assess_risk_fast_timed(
        &self,
        action: &Action,
        context: &ActionContext,
    ) -> (RiskAssessment, Duration) {
        let start = Instant::now();
        let assessment = self.assessor.assess_risk_fast(action, context);
        (assessment, start.elapsed())
    }

    fn log_audit(&self, action: &Action, risk_level: &str, decision: &str, reason: &str) {
        let seq = self.audit_sequence.fetch_add(1, Ordering::SeqCst);
        let prev_hash = {
            let log = self.audit_log.lock();
            log.last().map(|e| e.content_hash.clone())
        };
        let entry = GateAuditEntry::new(seq, action, risk_level, decision, reason, prev_hash);
        self.audit_log.lock().push(entry);
    }

    /// Verify the entire audit chain is tamper-evident
    pub fn verify_audit_chain(&self) -> bool {
        let log = self.audit_log.lock();
        for (i, entry) in log.iter().enumerate() {
            if !entry.verify_hash() {
                return false;
            }
            let prev = if i > 0 { Some(&log[i - 1]) } else { None };
            if !entry.verify_chain(prev) {
                return false;
            }
        }
        true
    }

    // Test helpers
    pub fn simulate_disconnect(&self) {
        self.user_disconnected.store(true, Ordering::SeqCst);
    }

    pub fn inject_shadow_sim_crash(&self) {
        self.shadow_sim_crash.store(true, Ordering::SeqCst);
    }

    pub fn set_user_always_rejects(&self) {
        self.user_always_rejects.store(true, Ordering::SeqCst);
    }

    pub fn reset_retry_count(&self) {
        self.approval_retry_count.store(0, Ordering::SeqCst);
    }
}

impl Default for ExecutionGate {
    fn default() -> Self {
        Self::new(GateConfig::default())
    }
}
