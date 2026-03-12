//! Learn phase — extracted from cognitive.rs for file size.
//! Contains the learn() method that dispatches to all learning sisters.

use hydra_native_state::utils::safe_truncate;
use super::cognitive::Sisters;

impl Sisters {
    /// LEARN: After response, dispatch to all learning sisters with V3 causal capture.
    ///
    /// Uses memory_capture_message (V3) for structured capture with causal chains,
    /// plus memory_capture_decision for corrections/preferences detected.
    pub async fn learn(&self, user_msg: &str, response: &str) {
        // Debug: log which sisters are connected
        eprintln!("[hydra:learn] memory={} identity={} cognition={} evolve={} time={}",
            if self.memory.is_some() { "CONNECTED" } else { "NONE" },
            if self.identity.is_some() { "CONNECTED" } else { "NONE" },
            if self.cognition.is_some() { "CONNECTED" } else { "NONE" },
            if self.evolve.is_some() { "CONNECTED" } else { "NONE" },
            if self.time.is_some() { "CONNECTED" } else { "NONE" },
        );
        eprintln!("[hydra:learn] user_msg='{}'", safe_truncate(&user_msg, 80));

        let lower = user_msg.to_lowercase();
        let is_correction = lower.starts_with("no,")
            || lower.starts_with("no ")
            || lower.starts_with("actually,")
            || lower.starts_with("actually ")
            || lower.contains("that's wrong")
            || lower.contains("that's not right")
            || lower.contains("i meant")
            || lower.starts_with("don't ")
            || lower.contains("always use")
            || lower.contains("never use")
            || lower.contains("i prefer");

        // Structured capture — uses memory_add for facts/preferences,
        // conversation_log for exchange history.
        // FIX 2: Skip storing questions and greetings — they pollute memory.
        use crate::cognitive::handlers::memory_intent::{is_question, is_greeting, classify_event_type};
        let should_store = is_correction
            || (!is_question(user_msg) && !is_greeting(user_msg));

        let v3_capture_fut = async {
            if !should_store {
                eprintln!("[hydra:learn] SKIPPED memory_add — question/greeting");
                return;
            }
            if let Some(mem) = &self.memory {
                eprintln!("[hydra:learn] Calling memory_add...");
                let content = format!("User: {}\nHydra: {}", user_msg, safe_truncate(&response, 200));
                let event_type = if is_correction { "correction" } else { classify_event_type(user_msg) };
                let result = mem.call_tool("memory_add", serde_json::json!({
                    "event_type": event_type,
                    "content": content,
                    "confidence": if is_correction { 0.95 } else { 0.8 },
                })).await;
                match &result {
                    Ok(v) => eprintln!("[hydra:learn] memory_add OK: {}", serde_json::to_string(v).unwrap_or_default()),
                    Err(e) => eprintln!("[hydra:learn] memory_add FAILED: {}", e),
                }

                // If correction detected, also store as high-importance fact
                if is_correction {
                    let _ = mem.call_tool("memory_add", serde_json::json!({
                        "event_type": "fact",
                        "content": format!("User preference: {}", user_msg),
                        "confidence": 0.95,
                    })).await;
                }
            } else {
                eprintln!("[hydra:learn] SKIPPED memory_add — memory sister is None");
            }
        };

        // V2 fallback: also log via conversation_log for backward compatibility
        let v2_log_fut = async {
            self.log_conversation(user_msg, response).await;
        };

        let cognition_fut = async {
            if let Some(s) = &self.cognition {
                let _ = s.call_tool("cognition_belief_revise", serde_json::json!({
                    "interaction": user_msg,
                    "response": safe_truncate(&response, 500),
                    "is_correction": is_correction,
                })).await;
            }
        };

        // Cognition user model update — longitudinal learning
        let cognition_model_fut = async {
            if let Some(s) = &self.cognition {
                let _ = s.call_tool("cognition_model_update", serde_json::json!({
                    "context": "current_user",
                    "observation": {
                        "message": safe_truncate(&user_msg, 300),
                        "response": safe_truncate(&response, 300),
                        "signals": {
                            "is_correction": is_correction,
                            "is_technical": Self::detects_code(user_msg),
                            "message_length": user_msg.len(),
                            "uses_slang": user_msg.contains("lol") || user_msg.contains("lmao") || user_msg.contains("bruh"),
                            "is_direct": user_msg.len() < 50,
                            "is_detailed": user_msg.len() > 200,
                        }
                    }
                })).await;
            }
        };

        let evolve_fut = async {
            if let Some(s) = &self.evolve {
                let _ = s.call_tool("evolve_crystallize", serde_json::json!({
                    "interaction": user_msg,
                    "response": safe_truncate(&response, 500),
                })).await;
            }
        };

        let identity_fut = async {
            if let Some(s) = &self.identity {
                let _ = s.call_tool("receipt_create", serde_json::json!({
                    "action": "conversation",
                    "input_summary": safe_truncate(&user_msg, 100),
                    "output_summary": safe_truncate(&response, 100),
                })).await;
            }
        };

        let time_fut = async {
            if let Some(s) = &self.time {
                let _ = s.call_tool("time_duration_track", serde_json::json!({
                    "action": user_msg,
                    "status": "completed",
                })).await;
            }
        };

        let quality_fut = async {
            if let Some(mem) = &self.memory {
                let _ = mem.call_tool("memory_quality", serde_json::json!({
                    "content": user_msg,
                    "action": "score"
                })).await;
            }
        };

        let reflect_fut = async {
            if let Some(s) = &self.cognition {
                let _ = s.call_tool("cognition_soul_reflect", serde_json::json!({
                    "interaction": user_msg,
                    "response": safe_truncate(&response, 500),
                })).await;
            }
        };

        let correct_fut = async {
            if is_correction {
                if let Some(mem) = &self.memory {
                    let _ = mem.call_tool("memory_correct", serde_json::json!({
                        "query": user_msg,
                        "correction": response,
                    })).await;
                }
            }
        };

        // Extract patterns from code-related interactions
        let pattern_fut = async {
            if Self::detects_code(user_msg) {
                if let Some(s) = &self.codebase {
                    let _ = s.call_tool("pattern_extract", serde_json::json!({
                        "context": safe_truncate(&user_msg, 200),
                    })).await;
                }
            }
        };

        let planning_learn_fut = async {
            if let Some(s) = &self.planning {
                let _ = s.call_tool("goal_progress", serde_json::json!({
                    "interaction": user_msg,
                    "outcome": safe_truncate(&response, 200),
                })).await;
            }
        };

        let comm_learn_fut = async {
            if let Some(s) = &self.comm {
                // Only share significant learnings (corrections, new patterns)
                if is_correction {
                    let _ = s.call_tool("broadcast_insight", serde_json::json!({
                        "insight": format!("User correction: {}", user_msg),
                        "source": "cognitive_loop",
                    })).await;
                }
            }
        };

        tokio::join!(v3_capture_fut, v2_log_fut, cognition_fut, cognition_model_fut, evolve_fut,
                     identity_fut, time_fut, quality_fut, reflect_fut, correct_fut, pattern_fut,
                     planning_learn_fut, comm_learn_fut);
    }
}
