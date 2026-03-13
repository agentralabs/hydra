//! DECIDE phase — extracted from loop_runner.rs for compilation performance.
//!
//! Graduated autonomy + risk gating. Returns `None` if the loop should abort
//! (e.g. approval denied, clarification needed).

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::cognitive::decide::DecideEngine;
use crate::cognitive::inventions::InventionEngine;
use crate::sisters::SistersHandle;
use hydra_native_state::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use hydra_db::HydraDb;
use hydra_runtime::approval::{ApprovalDecision, ApprovalManager};

use super::super::loop_runner::CognitiveUpdate;
use super::actions::detect_direct_action_command;

/// Output of the DECIDE phase, consumed by ACT.
pub(crate) struct DecideResult {
    pub gate_decision: &'static str,
    pub decide_ms: u64,
    pub adjusted_confidence: f32,
}

/// Run the DECIDE phase. Returns `None` if the cognitive loop should abort
/// (approval denied, timeout, or clarification requested).
pub(crate) async fn run_decide(
    text: &str,
    risk_level: &str,
    is_simple: bool,
    is_action_request: bool,
    intent: &super::super::intent_router::ClassifiedIntent,
    config: &super::super::loop_runner::CognitiveLoopConfig,
    decide_engine: &Arc<DecideEngine>,
    inventions: &Option<Arc<InventionEngine>>,
    sisters_handle: &Option<SistersHandle>,
    approval_manager: &Option<Arc<ApprovalManager>>,
    db: &Option<Arc<HydraDb>>,
    llm_config: &hydra_model::LlmConfig,
    active_model: &str,
    perceive_ms: u64,
    think_ms: u64,
    input_tokens: u64,
    output_tokens: u64,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Option<DecideResult> {
    let _ = tx.send(CognitiveUpdate::Phase("Decide".into()));
    let _ = tx.send(CognitiveUpdate::IconState("needs-attention".into()));
    let decide_start = Instant::now();

    if let Some(ref sh) = sisters_handle {
        if let Some(p) = sh.contract_precognition(text).await {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Contract Precognition".into(),
                content: format!("Risk: {} | Allowed: {} | {}", p.risk_level, p.allowed, p.reason),
            });
        }
        // Phase G: Trust impact prediction — how will this action affect trust?
        if risk_level == "medium" || risk_level == "high" || risk_level == "critical" {
            if let Some(prophecy) = sh.identity_trust_prophecy(text).await {
                let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                    title: "Trust Impact".into(), content: prophecy,
                });
            }
        }
    }
    let decide_result = decide_engine.check(risk_level, "");
    let contract_verdict = if let Some(ref sh) = sisters_handle {
        sh.decide_contract(text, risk_level).await
    } else { None };

    let veritas_uncertainty = if let Some(ref sh) = sisters_handle {
        sh.decide_veritas(text).await
    } else { None };

    // If contract says blocked, override gate decision
    let mut gate_decision = if let Some(ref verdict) = contract_verdict {
        if verdict.to_lowercase().contains("blocked") || verdict.to_lowercase().contains("denied") {
            "requires_approval"
        } else if decide_result.requires_approval && !decide_result.allowed {
            "requires_approval"
        } else if risk_level == "medium" {
            "shadow_first"
        } else {
            "approved"
        }
    } else if decide_result.requires_approval && !decide_result.allowed {
        "requires_approval"
    } else if risk_level == "medium" {
        "shadow_first"
    } else {
        "approved"
    };

    // Apply runtime risk threshold — auto-approve if settings allow
    if gate_decision == "requires_approval" && config.runtime.auto_approve_risk(risk_level) {
        eprintln!("[hydra:decide] Runtime risk_threshold={} auto-approves {} risk", config.runtime.risk_threshold, risk_level);
        gate_decision = "approved";
    }
    // Force approval for critical actions if setting requires it
    if config.runtime.require_approval_critical && risk_level == "critical" {
        gate_decision = "requires_approval";
    }

    // Report trust-based decision context to the UI
    let _ = tx.send(CognitiveUpdate::Phase(format!(
        "Decide (trust: {:.0}%, {:?})",
        decide_result.trust_score * 100.0,
        decide_result.autonomy_level,
    )));

    // ── Phase 1: History-Aware Future Echo + Metacognition Adjustment ──
    // Enhanced prediction that uses historical outcome data and metacognition feedback.
    // Phase 2: adjusted_confidence declared outside block so X1 and gate checks can access it
    let mut adjusted_confidence: f32 = 0.5;
    if let Some(ref inv) = inventions {
        let risk_float: f32 = match risk_level {
            "high" | "critical" => 0.8,
            "medium" => 0.5,
            "low" => 0.2,
            _ => 0.1,
        };

        // Phase 1: Check metacognition for overconfidence bias
        let (is_overconfident, confidence_adjustment) = inv.check_overconfidence();
        if is_overconfident {
            eprintln!("[hydra:metacog] Overconfidence detected — applying {:.0}% confidence adjustment",
                confidence_adjustment * 100.0);
        }

        // Phase 1: Get historical confidence for similar actions
        let historical_factor = inv.historical_confidence_for(text);
        if historical_factor < 0.8 {
            eprintln!("[hydra:history] Low historical success rate ({:.0}%) for similar actions",
                historical_factor * 100.0);
        }

        let (raw_confidence, recommendation, prediction_desc) =
            inv.future_echo(text, risk_float);

        // Phase 2, P2: Perception Confidence Scoring
        // Blend intent classifier confidence with prediction confidence.
        // If the classifier is very uncertain (low confidence), reduce overall confidence.
        let perception_factor: f32 = if intent.confidence < 0.3 {
            0.7  // Significant penalty for uncertain intent classification
        } else if intent.confidence < 0.5 {
            0.85 // Moderate penalty
        } else {
            1.0  // No penalty for confident classification
        };

        // Veritas uncertainty factor — if Veritas flagged high uncertainty, reduce confidence
        let veritas_factor: f32 = match veritas_uncertainty.as_deref() {
            Some(v) if v.contains("high") || v.contains("uncertain") => 0.7,
            Some(v) if v.contains("medium") => 0.85,
            _ => 1.0,
        };
        // Apply all adjustments to confidence
        let momentum_penalty = inv.momentum_confidence_penalty() as f32;
        adjusted_confidence = (raw_confidence * confidence_adjustment * historical_factor * perception_factor * veritas_factor) - momentum_penalty;
        adjusted_confidence = adjusted_confidence.max(0.0); // Floor at 0

        let _ = tx.send(CognitiveUpdate::Phase(format!(
            "Prediction: {} (confidence: {:.0}%, risk: {}{})",
            prediction_desc,
            adjusted_confidence * 100.0,
            recommendation,
            if is_overconfident { " ⚠ overconfidence adjusted" } else { "" }
        )));
        let _ = tx.send(CognitiveUpdate::PredictionResult {
            action: text.to_string(),
            confidence: adjusted_confidence as f64,
            recommendation: recommendation.clone(),
        });

        // Phase 1: Shadow Self as Quality Gate — run for actions that involve execution.
        // Expanded from medium+ risk to include any action with direct commands or file ops.
        let has_commands = detect_direct_action_command(text).is_some()
            || is_action_request
            || risk_level == "medium" || risk_level == "high" || risk_level == "critical";

        // Track shadow result for D2 gate decision below
        let mut shadow_was_safe = true;
        if has_commands {
            let expected = std::collections::HashMap::new();
            let (safe, shadow_rec) = inv.shadow_validate(text, &expected);
            shadow_was_safe = safe;
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Shadow Validation".to_string(),
                content: format!("Safe: {} | {}", safe, shadow_rec),
            });
            let _ = tx.send(CognitiveUpdate::ShadowValidation {
                safe,
                recommendation: shadow_rec.clone(),
            });

            // Shadow flags unsafe on low-risk → escalate to shadow_first
            if !safe && risk_level == "low" && gate_decision == "approved" {
                eprintln!("[hydra:shadow] Shadow flagged unsafe on low-risk — escalating to shadow_first");
                gate_decision = "shadow_first";
            }
        }

        // Phase 1: Metacognition — if adjusted confidence is very low, warn
        if adjusted_confidence < 0.3 {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Low Confidence Warning".to_string(),
                content: format!(
                    "Confidence is only {:.0}%. Historical success: {:.0}%, Metacognition adjustment: {:.0}%",
                    adjusted_confidence * 100.0,
                    historical_factor * 100.0,
                    confidence_adjustment * 100.0,
                ),
            });
        }

        // Low confidence + shadow unsafe → warn only, proceed with execution
        if adjusted_confidence < 0.2 {
            eprintln!("[hydra:predict] ⚠ Low confidence {:.0}% — proceeding anyway", adjusted_confidence * 100.0);
        }
        if !shadow_was_safe {
            eprintln!("[hydra:shadow] ⚠ Shadow flagged unsafe on {} risk — proceeding", risk_level);
        }
    }

    // Low confidence + unknown intent — proceed anyway, Hydra tries its best
    if adjusted_confidence < 0.25
        && matches!(intent.category, super::super::intent_router::IntentCategory::Unknown)
    {
        eprintln!("[hydra:decide] ⚠ Low confidence {:.0}% + unknown intent — proceeding anyway",
            adjusted_confidence * 100.0);
    }

    // Contract sister: request approval for auditable receipt chain
    if gate_decision == "requires_approval" {
        if let Some(ref sh) = sisters_handle {
            let _ = sh.contract_request_approval(text, risk_level, "Cognitive loop action").await;
        }
    }

    // Contract: policy compliance check for all actions
    if let Some(ref sh) = sisters_handle {
        if let Some(pr) = sh.contract_policy_check(text).await {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory { title: "Policy Check".into(), content: pr });
        }
    }
    // Step 3.7: Gate integration — if action requires approval, notify UI
    if gate_decision == "requires_approval" {
        // Phase 3, C1: Challenge phrase gate — use ChallengePhraseGate for
        // irreversible HIGH+ risk actions, not just critical
        let challenge = if crate::cognitive::decide::ChallengePhraseGate::should_challenge(risk_level, text) {
            let gate = crate::cognitive::decide::ChallengePhraseGate::new(text);
            Some(gate.phrase)
        } else {
            None
        };
        let _ = tx.send(CognitiveUpdate::IconState("needs-attention".into()));

        // REAL APPROVAL BLOCKING: Use ApprovalManager to wait for user decision
        if let Some(ref mgr) = approval_manager {
            let (req, rx) = mgr.request_approval(
                &config.task_id,
                text,
                None,
                decide_result.trust_score,
                &format!("{} risk action", risk_level),
            );
            // Send the approval ID to UI so buttons can submit decision
            let _ = tx.send(CognitiveUpdate::AwaitApproval {
                approval_id: Some(req.id.clone()),
                risk_level: risk_level.to_string(),
                action: text.to_string(),
                description: format!(
                    "This action is classified as {} risk. Trust: {:.0}%, level: {:?}",
                    risk_level,
                    decide_result.trust_score * 100.0,
                    decide_result.autonomy_level,
                ),
                challenge_phrase: challenge,
            });
            tracing::info!("[hydra] Approval requested: {} ({})", req.id, risk_level);

            match mgr.wait_for_approval(&req.id, rx).await {
                Ok(ApprovalDecision::Approved) => {
                    tracing::info!("[hydra] Approval GRANTED: {}", req.id);
                    let _ = tx.send(CognitiveUpdate::Phase("Approved — proceeding".into()));
                }
                Ok(ApprovalDecision::Denied { reason }) => {
                    tracing::warn!("[hydra] Approval DENIED: {} — {}", req.id, reason);
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: format!("Action denied: {}", reason),
                        css_class: "message hydra error".into(),
                    });
                    let _ = tx.send(CognitiveUpdate::ResetIdle);
                    return None; // ABORT — denied
                }
                Ok(ApprovalDecision::Modified { new_action }) => {
                    tracing::info!("[hydra] Approval MODIFIED: {} → {}", req.id, new_action);
                    let _ = tx.send(CognitiveUpdate::Phase(format!("Modified: {}", new_action)));
                    // Continue with the modified action
                }
                Err(e) => {
                    tracing::warn!("[hydra] Approval timeout/cancelled: {} — {}", req.id, e);
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: format!("Approval timed out or was cancelled. Action not executed for safety."),
                        css_class: "message hydra error".into(),
                    });
                    let _ = tx.send(CognitiveUpdate::ResetIdle);
                    return None; // ABORT — timeout = deny by default
                }
            }
        } else {
            // No approval manager — send approval without ID and pause briefly (dev mode)
            let _ = tx.send(CognitiveUpdate::AwaitApproval {
                approval_id: None,
                risk_level: risk_level.to_string(),
                action: text.to_string(),
                description: format!(
                    "This action is classified as {} risk. Trust: {:.0}%, level: {:?}",
                    risk_level, decide_result.trust_score * 100.0, decide_result.autonomy_level,
                ),
                challenge_phrase: challenge,
            });
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    } else if gate_decision == "shadow_first" {
        // Shadow simulation: run action in sandbox first via Aegis sister
        if let Some(ref sh) = sisters_handle {
            if let Some(aegis) = &sh.aegis {
                let _ = aegis.call_tool("shadow_simulate", serde_json::json!({
                    "action": text,
                    "risk_level": risk_level,
                })).await;
            }
        }
    }

    let decide_ms = decide_start.elapsed().as_millis() as u64;

    if !is_simple {
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: 2, duration_ms: Some(decide_ms) });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(3));
    }
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Completed, tokens_used: Some(input_tokens + output_tokens), duration_ms: Some(think_ms) },
        PhaseStatus { phase: CognitivePhase::Decide, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(decide_ms) },
        PhaseStatus { phase: CognitivePhase::Act, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));

    Some(DecideResult {
        gate_decision,
        decide_ms,
        adjusted_confidence,
    })
}
