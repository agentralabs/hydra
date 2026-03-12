//! Perceive phase — extracted from cognitive.rs for file size.
//! Contains perceive() and perceive_simple() methods.

use super::connection::extract_text;
use super::cognitive::Sisters;

impl Sisters {
    /// PERCEIVE: Gather context from ALL available sisters in parallel
    pub async fn perceive(&self, text: &str) -> serde_json::Value {
        // Debug: log sister connection status at perceive time
        let connected: Vec<&str> = self.all_sisters().iter()
            .filter_map(|(name, opt)| if opt.is_some() { Some(*name) } else { None })
            .collect();
        eprintln!("[hydra:perceive] {} sisters connected: {:?}", connected.len(), connected);

        let involves_code = Self::detects_code(text);
        let involves_vision = Self::detects_visual(text);

        // Facts/corrections/decisions first — high-signal stored preferences
        let facts_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_query", serde_json::json!({
                    "query": text,
                    "event_types": ["fact", "correction", "decision"],
                    "max_results": 5,
                    "sort_by": "highest_confidence"
                })).await.ok()
            } else { None }
        };
        let memory_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_query", serde_json::json!({"query": text, "max_results": 5})).await.ok()
            } else { None }
        };
        // V4 longevity search: deeper semantic search across 20-year hierarchy
        let longevity_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_longevity_search", serde_json::json!({
                    "query": text,
                    "limit": 3,
                    "include_layers": ["episode", "summary", "pattern"]
                })).await.ok()
            } else { None }
        };
        let identity_fut = async {
            if let Some(s) = &self.identity {
                s.call_tool("identity_whoami", serde_json::json!({})).await.ok()
            } else { None }
        };
        let time_fut = async {
            if let Some(s) = &self.time {
                s.call_tool("time_stats", serde_json::json!({})).await.ok()
            } else { None }
        };
        let cognition_fut = async {
            if let Some(s) = &self.cognition {
                s.call_tool("cognition_model_query", serde_json::json!({"context": "current_user"})).await.ok()
            } else { None }
        };
        let reality_fut = async {
            if let Some(s) = &self.reality {
                s.call_tool("reality_context", serde_json::json!({"input": text})).await.ok()
            } else { None }
        };
        let similar_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_similar", serde_json::json!({"content": text, "limit": 3})).await.ok()
            } else { None }
        };
        let ground_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_ground", serde_json::json!({"claim": text})).await.ok()
            } else { None }
        };
        let predict_fut = async {
            if let Some(s) = &self.cognition {
                s.call_tool("cognition_predict", serde_json::json!({"context": text})).await.ok()
            } else { None }
        };
        // ── Memory prediction: preload likely-needed memories (smarter than generic query) ──
        let mem_predict_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_predict", serde_json::json!({
                    "context": text, "max_results": 5, "include_confidence": true,
                })).await.ok()
            } else { None }
        };
        // ── Déjà vu: detect if user is revisiting a previous topic ──
        let dejavu_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_dejavu_check", serde_json::json!({"context": text})).await.ok()
            } else { None }
        };

        // ── Veritas intent verification (detect ambiguity in user query) ──
        let veritas_fut = async {
            if let Some(s) = &self.veritas {
                s.call_tool("verify_intent", serde_json::json!({"input": text})).await.ok()
            } else { None }
        };
        // ── Contract policy check (are there constraints on this query?) ──
        let contract_fut = async {
            if let Some(s) = &self.contract {
                s.call_tool("policy_query", serde_json::json!({"action": text})).await.ok()
            } else { None }
        };
        // ── Planning goal context (any active goals relevant to this query?) ──
        let planning_fut = async {
            if let Some(s) = &self.planning {
                s.call_tool("goal_query", serde_json::json!({"context": text})).await.ok()
            } else { None }
        };

        // ── Comm sister (check for pending messages/notifications) ──
        let comm_fut = async {
            if let Some(s) = &self.comm {
                s.call_tool("comm_inbox", serde_json::json!({"limit": 5})).await.ok()
            } else { None }
        };
        // ── Forge blueprint lookup (any existing blueprints for this topic?) ──
        let forge_fut = async {
            if let Some(s) = &self.forge {
                s.call_tool("blueprint_query", serde_json::json!({"query": text})).await.ok()
            } else { None }
        };
        // ── Temporal memory recall (what happened at similar times/contexts?) ──
        let temporal_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_temporal_recall", serde_json::json!({
                    "query": text,
                    "limit": 3
                })).await.ok()
            } else { None }
        };

        let (facts_r, memory_r, longevity_r, identity_r, time_r, cognition_r, reality_r,
             similar_r, ground_r, predict_r, mem_predict_r, dejavu_r, veritas_r, contract_r,
             planning_r, comm_r, forge_r, temporal_r) =
            tokio::join!(facts_fut, memory_fut, longevity_fut, identity_fut, time_fut, cognition_fut, reality_fut,
                         similar_fut, ground_fut, predict_fut, mem_predict_fut, dejavu_fut,
                         veritas_fut, contract_fut, planning_fut,
                         comm_fut, forge_fut, temporal_fut);

        // Conditional: Codebase tools (if code) — run in parallel
        let (codebase_r, concept_r, impact_r) = if involves_code {
            let code_fut = async {
                if let Some(s) = &self.codebase {
                    s.call_tool("search_semantic", serde_json::json!({"query": text})).await.ok()
                } else { None }
            };
            let concept_fut = async {
                if let Some(s) = &self.codebase {
                    s.call_tool("concept_find", serde_json::json!({"concept": text})).await.ok()
                } else { None }
            };
            let impact_fut = async {
                if let Some(s) = &self.codebase {
                    s.call_tool("impact_analyze", serde_json::json!({"query": text})).await.ok()
                } else { None }
            };
            tokio::join!(code_fut, concept_fut, impact_fut)
        } else {
            (None, None, None)
        };

        // Conditional: Vision (if visual)
        let vision_r = if involves_vision {
            if let Some(s) = &self.vision {
                s.call_tool("vision_capture", serde_json::json!({"context": text})).await.ok()
            } else {
                // Fallback: attempt direct screencapture on macOS when Vision sister is offline
                Self::screencapture_fallback().await
            }
        } else { None };

        let extract = |r: &Option<serde_json::Value>| -> Option<String> {
            r.as_ref().map(|v| extract_text(v)).filter(|t| !t.is_empty() && !t.contains("No memories found"))
        };

        // Merge facts (high-signal) + general memory + V4 longevity
        let facts_text = extract(&facts_r);
        let general_text = extract(&memory_r);
        let longevity_text = extract(&longevity_r);
        let merged_memory = match (&facts_text, &general_text, &longevity_text) {
            (Some(f), Some(m), Some(l)) => Some(format!("### Stored Facts:\n{}\n\n### Recent Memory:\n{}\n\n### Long-Term Memory:\n{}", f, m, l)),
            (Some(f), Some(m), None) => Some(format!("### Stored Facts:\n{}\n\n### Recent Memory:\n{}", f, m)),
            (Some(f), None, Some(l)) => Some(format!("### Stored Facts:\n{}\n\n### Long-Term Memory:\n{}", f, l)),
            (Some(f), None, None) => Some(f.clone()),
            (None, Some(m), Some(l)) => Some(format!("{}\n\n### Long-Term Memory:\n{}", m, l)),
            (None, Some(m), None) => Some(m.clone()),
            (None, None, Some(l)) => Some(format!("### Long-Term Memory:\n{}", l)),
            (None, None, None) => None,
        };

        serde_json::json!({
            "input": text,
            "involves_code": involves_code,
            "involves_vision": involves_vision,
            "memory_context": merged_memory,
            "identity_context": extract(&identity_r),
            "time_context": extract(&time_r),
            "cognition_context": extract(&cognition_r),
            "reality_context": extract(&reality_r),
            "codebase_context": extract(&codebase_r),
            "concept_context": extract(&concept_r),
            "impact_context": extract(&impact_r),
            "vision_context": extract(&vision_r),
            "similar_context": extract(&similar_r),
            "grounding_context": extract(&ground_r),
            "prediction_context": extract(&predict_r),
            "veritas_context": extract(&veritas_r),
            "contract_context": extract(&contract_r),
            "planning_context": extract(&planning_r),
            "comm_context": extract(&comm_r),
            "forge_context": extract(&forge_r),
            "temporal_context": extract(&temporal_r),
            "memory_prediction": extract(&mem_predict_r),
            "dejavu_context": extract(&dejavu_r),
            "sisters_online": self.connected_count(),
        })
    }

    /// Lightweight perceive for simple queries — only queries memory + cognition.
    pub async fn perceive_simple(&self, text: &str) -> serde_json::Value {
        let facts_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_query", serde_json::json!({
                    "query": text,
                    "event_types": ["fact", "correction", "decision"],
                    "max_results": 5,
                    "sort_by": "highest_confidence"
                })).await.ok()
            } else { None }
        };
        let general_memory_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_query", serde_json::json!({
                    "query": text,
                    "max_results": 3
                })).await.ok()
            } else { None }
        };
        let cognition_fut = async {
            if let Some(s) = &self.cognition {
                s.call_tool("cognition_model_query", serde_json::json!({"context": "current_user"})).await.ok()
            } else { None }
        };
        let beliefs_fut = async {
            if let Some(s) = &self.cognition {
                s.call_tool("cognition_belief_query", serde_json::json!({"query": text})).await.ok()
            } else { None }
        };

        let (facts_r, general_r, cognition_r, beliefs_r) = tokio::join!(
            facts_fut, general_memory_fut, cognition_fut, beliefs_fut
        );

        let extract = |r: &Option<serde_json::Value>| -> Option<String> {
            r.as_ref().map(|v| extract_text(v)).filter(|t| !t.is_empty() && !t.contains("No memories found"))
        };

        // Facts get priority — merge facts first, then general memory
        let memory_context = match (extract(&facts_r), extract(&general_r)) {
            (Some(f), Some(g)) => Some(format!("### Stored Facts:\n{}\n\n### Recent Context:\n{}", f, g)),
            (Some(f), None) => Some(f),
            (None, Some(g)) => Some(g),
            (None, None) => None,
        };

        // Merge cognition + beliefs
        let cog_context = match (extract(&cognition_r), extract(&beliefs_r)) {
            (Some(c), Some(b)) => Some(format!("{}\n\n{}", c, b)),
            (Some(c), None) => Some(c),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        serde_json::json!({
            "input": text,
            "involves_code": false,
            "involves_vision": false,
            "memory_context": memory_context,
            "cognition_context": cog_context,
            "sisters_online": self.connected_count(),
        })
    }
}
