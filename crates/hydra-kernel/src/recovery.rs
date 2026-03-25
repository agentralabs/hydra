//! O30: Recovery Loop — adaptive replanning when actions fail.
//!
//! When prediction fails or a step errors, Hydra does not abort.
//! It classifies the failure, checks genome for known recoveries,
//! and re-compiles the remaining plan toward the original goal.
//! Max 3 recovery attempts per step to prevent infinite loops.

use crate::conductor::Step;

const MAX_RECOVERY_ATTEMPTS: u32 = 3;

/// Context for a recovery attempt.
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    pub failed_step_id: usize,
    pub failure_reason: String,
    pub original_goal: String,
    pub completed_steps: Vec<usize>,
    pub remaining_steps: Vec<Step>,
    pub screen_description: String,
    pub attempt: u32,
}

/// What the recovery engine decides to do.
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Dismiss an unexpected dialog and resume from a step.
    DismissAndResume { dismiss_method: String, resume_at: usize },
    /// Re-compile remaining steps with new context.
    Recompile { context: String, remaining_goal: String },
    /// Skip the failed step (non-critical).
    SkipAndContinue { reason: String },
    /// Search web for solution and retry.
    SearchAndRetry { query: String },
    /// Escalate to human — unrecoverable.
    Escalate { reason: String },
}

/// Classification of failure types.
#[derive(Debug, Clone, PartialEq)]
pub enum FailureType {
    UnexpectedDialog,
    ElementNotFound,
    WrongState,
    NetworkError,
    PermissionDenied,
    Timeout,
    Unknown,
}

/// The recovery engine — stateless, analyzes context and returns action.
pub struct RecoveryEngine;

impl RecoveryEngine {
    /// Analyze a failure and produce a recovery action.
    pub fn recover(
        ctx: &RecoveryContext,
        genome: &hydra_genome::GenomeStore,
    ) -> RecoveryAction {
        // Guard: max attempts
        if ctx.attempt >= MAX_RECOVERY_ATTEMPTS {
            return RecoveryAction::Escalate {
                reason: format!("Max recovery attempts ({MAX_RECOVERY_ATTEMPTS}) exhausted for step {}",
                    ctx.failed_step_id),
            };
        }

        let failure_type = Self::classify_failure(&ctx.failure_reason, &ctx.screen_description);
        eprintln!("hydra-recovery: step {} failed ({:?}): {}",
            ctx.failed_step_id, failure_type, ctx.failure_reason);

        // Check genome for known recovery pattern
        if let Some(action) = Self::known_recovery(&ctx.failure_reason, genome) {
            eprintln!("hydra-recovery: genome match found for '{}'", ctx.failure_reason);
            return action;
        }

        // Classify and decide
        match failure_type {
            FailureType::UnexpectedDialog => {
                RecoveryAction::DismissAndResume {
                    dismiss_method: "press Escape or click Cancel/OK".into(),
                    resume_at: ctx.failed_step_id,
                }
            }
            FailureType::ElementNotFound => {
                if ctx.attempt < 2 {
                    // Retry with scrolling — element might be off-screen
                    RecoveryAction::DismissAndResume {
                        dismiss_method: "scroll down and retry".into(),
                        resume_at: ctx.failed_step_id,
                    }
                } else {
                    RecoveryAction::Recompile {
                        context: format!("Element not found: {}", ctx.failure_reason),
                        remaining_goal: ctx.original_goal.clone(),
                    }
                }
            }
            FailureType::WrongState => {
                RecoveryAction::Recompile {
                    context: format!("App in unexpected state: {}", ctx.screen_description),
                    remaining_goal: ctx.original_goal.clone(),
                }
            }
            FailureType::NetworkError => {
                RecoveryAction::SearchAndRetry {
                    query: format!("fix network error: {}", ctx.failure_reason),
                }
            }
            FailureType::PermissionDenied => {
                RecoveryAction::Escalate {
                    reason: format!("Permission denied: {}", ctx.failure_reason),
                }
            }
            FailureType::Timeout => {
                if ctx.attempt < 2 {
                    RecoveryAction::DismissAndResume {
                        dismiss_method: "wait longer and retry".into(),
                        resume_at: ctx.failed_step_id,
                    }
                } else {
                    RecoveryAction::Escalate {
                        reason: format!("Persistent timeout on step {}", ctx.failed_step_id),
                    }
                }
            }
            FailureType::Unknown => {
                if !ctx.remaining_steps.is_empty() {
                    RecoveryAction::SkipAndContinue {
                        reason: format!("Unknown failure — skipping step {}", ctx.failed_step_id),
                    }
                } else {
                    RecoveryAction::Escalate {
                        reason: format!("Unknown failure on last step: {}", ctx.failure_reason),
                    }
                }
            }
        }
    }

    /// Classify a failure from its reason text and screen state.
    fn classify_failure(reason: &str, screen: &str) -> FailureType {
        let lower = reason.to_lowercase();
        let screen_lower = screen.to_lowercase();
        if lower.contains("dialog") || lower.contains("popup") || lower.contains("alert")
            || screen_lower.contains("dialog") || lower.contains("license")
            || lower.contains("expired") || lower.contains("update available") {
            FailureType::UnexpectedDialog
        } else if lower.contains("not found") || lower.contains("no element")
            || lower.contains("selector") || lower.contains("missing") {
            FailureType::ElementNotFound
        } else if lower.contains("wrong state") || lower.contains("unexpected")
            || lower.contains("different screen") {
            FailureType::WrongState
        } else if lower.contains("network") || lower.contains("connection")
            || lower.contains("dns") || lower.contains("timeout") && lower.contains("http") {
            FailureType::NetworkError
        } else if lower.contains("permission") || lower.contains("denied")
            || lower.contains("access") || lower.contains("forbidden") {
            FailureType::PermissionDenied
        } else if lower.contains("timeout") || lower.contains("timed out") {
            FailureType::Timeout
        } else {
            FailureType::Unknown
        }
    }

    /// Check genome for a known recovery pattern.
    fn known_recovery(failure: &str, genome: &hydra_genome::GenomeStore) -> Option<RecoveryAction> {
        let query = format!("recovery:{failure}");
        let matches = genome.query(&query);
        let entry = matches.first()?;
        if entry.effective_confidence() < 0.5 { return None; }
        // Parse the recovery action from the stored approach
        let step = entry.approach.steps.first()?;
        if step.contains("dismiss") || step.contains("escape") || step.contains("cancel") {
            Some(RecoveryAction::DismissAndResume {
                dismiss_method: step.clone(), resume_at: 0,
            })
        } else if step.contains("skip") {
            Some(RecoveryAction::SkipAndContinue { reason: step.clone() })
        } else {
            Some(RecoveryAction::Recompile {
                context: step.clone(), remaining_goal: String::new(),
            })
        }
    }
}

/// Store a successful recovery in genome for future use.
pub fn record_recovery(
    failure: &str,
    action: &RecoveryAction,
    genome: &mut hydra_genome::GenomeStore,
) {
    let tag = format!("recovery:{failure}");
    let step_desc = match action {
        RecoveryAction::DismissAndResume { dismiss_method, .. } => dismiss_method.clone(),
        RecoveryAction::Recompile { context, .. } => format!("recompile:{context}"),
        RecoveryAction::SkipAndContinue { reason } => format!("skip:{reason}"),
        RecoveryAction::SearchAndRetry { query } => format!("search:{query}"),
        RecoveryAction::Escalate { reason } => format!("escalate:{reason}"),
    };
    let _ = genome.add_from_operation(
        &tag,
        hydra_genome::ApproachSignature {
            approach_type: "recovery".into(),
            steps: vec![step_desc],
            tools_used: vec!["recovery_engine".into()],
        },
        0.6, // start with moderate confidence
    );
}
