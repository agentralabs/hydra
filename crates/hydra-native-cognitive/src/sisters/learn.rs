//! Learn phase — extracted from cognitive.rs for file size.
//! Contains the learn() method that dispatches to all learning sisters.

use hydra_native_state::utils::safe_truncate;
use super::cognitive::Sisters;

/// Log a sister call_tool result — never silently swallow errors.
macro_rules! log_call {
    ($label:expr, $result:expr) => {
        match $result {
            Ok(_) => {}
            Err(e) => eprintln!("[hydra:learn] {} FAILED: {}", $label, e),
        }
    };
}

impl Sisters {
    /// LEARN: After response, dispatch to all learning sisters.
    /// RULE: Store everything, classify nothing. Let sisters handle intelligence.
    pub async fn learn(&self, user_msg: &str, response: &str) {
        eprintln!("[hydra:learn] memory={} identity={} cognition={} evolve={} time={}",
            if self.memory.is_some() { "CONNECTED" } else { "NONE" },
            if self.identity.is_some() { "CONNECTED" } else { "NONE" },
            if self.cognition.is_some() { "CONNECTED" } else { "NONE" },
            if self.evolve.is_some() { "CONNECTED" } else { "NONE" },
            if self.time.is_some() { "CONNECTED" } else { "NONE" },
        );
        eprintln!("[hydra:learn] user_msg='{}'", safe_truncate(&user_msg, 80));

        // Store EVERYTHING as episode. No classification.
        let v3_capture_fut = async {
            if let Some(mem) = &self.memory {
                let content = format!("User: {}\nHydra: {}", user_msg, safe_truncate(&response, 200));
                match mem.call_tool("memory_add", serde_json::json!({
                    "event_type": "episode", "content": content, "confidence": 0.8,
                })).await {
                    Ok(v) => eprintln!("[hydra:learn] memory_add OK: {}", serde_json::to_string(&v).unwrap_or_default()),
                    Err(e) => eprintln!("[hydra:learn] memory_add FAILED: {}", e),
                }
            } else {
                eprintln!("[hydra:learn] SKIPPED memory_add — memory sister not connected");
            }
        };

        let v2_log_fut = async { self.log_conversation(user_msg, response).await; };

        let cognition_fut = async {
            if let Some(s) = &self.cognition {
                log_call!("cognition_belief_revise", s.call_tool("cognition_belief_revise", serde_json::json!({
                    "interaction": user_msg, "response": safe_truncate(&response, 500),
                })).await);
            }
        };

        let cognition_model_fut = async {
            if let Some(s) = &self.cognition {
                log_call!("cognition_model_heartbeat", s.call_tool("cognition_model_heartbeat", serde_json::json!({
                    "context": "current_user",
                    "observation": {
                        "message": safe_truncate(&user_msg, 300),
                        "response": safe_truncate(&response, 300),
                        "signals": {
                            "is_technical": Self::detects_code(user_msg),
                            "message_length": user_msg.len(),
                            "is_direct": user_msg.len() < 50,
                            "is_detailed": user_msg.len() > 200,
                        }
                    }
                })).await);
            }
        };

        let evolve_fut = async {
            if let Some(s) = &self.evolve {
                log_call!("evolve_crystallize", s.call_tool("evolve_crystallize", serde_json::json!({
                    "interaction": user_msg, "response": safe_truncate(&response, 500),
                })).await);
            }
        };

        let identity_fut = async {
            if let Some(s) = &self.identity {
                log_call!("receipt_create", s.call_tool("receipt_create", serde_json::json!({
                    "action": "conversation",
                    "input_summary": safe_truncate(&user_msg, 100),
                    "output_summary": safe_truncate(&response, 100),
                })).await);
            }
        };

        let time_fut = async {
            if let Some(s) = &self.time {
                log_call!("time_duration_track", s.call_tool("time_duration_track", serde_json::json!({
                    "action": user_msg, "status": "completed",
                })).await);
            }
        };

        let quality_fut = async {
            if let Some(mem) = &self.memory {
                log_call!("memory_quality", mem.call_tool("memory_quality", serde_json::json!({
                    "content": user_msg, "action": "score"
                })).await);
            }
        };

        let reflect_fut = async {
            if let Some(s) = &self.cognition {
                log_call!("cognition_soul_reflect", s.call_tool("cognition_soul_reflect", serde_json::json!({
                    "interaction": user_msg, "response": safe_truncate(&response, 500),
                })).await);
            }
        };

        let correct_fut = async {};

        let pattern_fut = async {
            if Self::detects_code(user_msg) {
                if let Some(s) = &self.codebase {
                    log_call!("pattern_extract", s.call_tool("pattern_extract", serde_json::json!({
                        "context": safe_truncate(&user_msg, 200),
                    })).await);
                    log_call!("hallucination_check", s.call_tool("hallucination_check", serde_json::json!({
                        "output": safe_truncate(&response, 500),
                    })).await);
                    log_call!("truth_register", s.call_tool("truth_register", serde_json::json!({
                        "claim": safe_truncate(&response, 300),
                    })).await);
                }
            }
        };

        let planning_learn_fut = async {
            if let Some(s) = &self.planning {
                log_call!("goal_progress", s.call_tool("goal_progress", serde_json::json!({
                    "interaction": user_msg, "outcome": safe_truncate(&response, 200),
                })).await);
            }
        };

        let comm_learn_fut = async {
            if let Some(s) = &self.comm {
                log_call!("broadcast_insight", s.call_tool("broadcast_insight", serde_json::json!({
                    "insight": safe_truncate(user_msg, 200), "source": "cognitive_loop",
                })).await);
            }
        };

        let immortal_capture_fut = async { self.memory_capture_exchange(user_msg, response).await; };
        let comm_session_log_fut = async { self.comm_session_log(user_msg, response).await; };
        let trust_reinforce_fut = async { self.identity_trust_reinforce("global", safe_truncate(user_msg, 100)).await; };
        let identity_actions_fut = async { self.identity_actions_log(safe_truncate(user_msg, 100), safe_truncate(response, 100)).await; };

        let contract_record_fut = async {
            if let Some(s) = &self.contract {
                log_call!("contract_record_decision", s.call_tool("contract_record_decision", serde_json::json!({
                    "action": safe_truncate(user_msg, 200),
                    "outcome": safe_truncate(response, 200),
                    "source": "cognitive_loop",
                })).await);
            }
        };

        let evolve_record_fut = async { self.evolve_record_pattern(safe_truncate(user_msg, 200), true).await; };

        tokio::join!(v3_capture_fut, v2_log_fut, cognition_fut, cognition_model_fut, evolve_fut,
                     identity_fut, time_fut, quality_fut, reflect_fut, correct_fut, pattern_fut,
                     planning_learn_fut, comm_learn_fut, immortal_capture_fut, comm_session_log_fut,
                     trust_reinforce_fut, identity_actions_fut, contract_record_fut, evolve_record_fut);
    }
}
