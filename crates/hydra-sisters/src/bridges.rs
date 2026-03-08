use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;

use crate::bridge::*;

/// Generic MCP-based sister bridge.
/// Each sister bridge wraps MCP tool calls to its sister's MCP server.
pub struct McpSisterBridge {
    id: SisterId,
    bridge_name: &'static str,
    bridge_version: &'static str,
    caps: Vec<String>,
    available: AtomicBool,
    timeout: Duration,
}

impl McpSisterBridge {
    fn new(id: SisterId, name: &'static str, version: &'static str, capabilities: &[&str]) -> Self {
        Self {
            id,
            bridge_name: name,
            bridge_version: version,
            caps: capabilities.iter().map(|s| s.to_string()).collect(),
            available: AtomicBool::new(true),
            timeout: Duration::from_secs(5),
        }
    }

    pub fn set_available(&self, available: bool) {
        self.available.store(available, Ordering::SeqCst);
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
}

#[async_trait]
impl SisterBridge for McpSisterBridge {
    fn sister_id(&self) -> SisterId {
        self.id
    }

    fn name(&self) -> &str {
        self.bridge_name
    }

    fn version(&self) -> &str {
        self.bridge_version
    }

    async fn health_check(&self) -> HealthStatus {
        if self.available.load(Ordering::SeqCst) {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unavailable
        }
    }

    async fn call(&self, action: SisterAction) -> Result<SisterResult, SisterError> {
        if !self.available.load(Ordering::SeqCst) {
            return Err(SisterError {
                sister_id: self.id,
                message: format!(
                    "{} is not available. The service may be offline or not configured.",
                    self.bridge_name
                ),
                retryable: true,
            });
        }

        // Simulate MCP tool call (real implementation uses LiveMcpBridge)
        let result = tokio::time::timeout(self.timeout, async {
            // Simulated response — in production this calls the sister's MCP server
            SisterResult {
                data: serde_json::json!({
                    "sister": self.bridge_name,
                    "tool": action.tool,
                    "status": "ok",
                }),
                tokens_used: 0,
            }
        })
        .await
        .map_err(|_| SisterError {
            sister_id: self.id,
            message: format!(
                "{} timed out after {:?}. The sister may be overloaded.",
                self.bridge_name, self.timeout
            ),
            retryable: true,
        })?;

        Ok(result)
    }

    async fn batch_call(
        &self,
        actions: Vec<SisterAction>,
    ) -> Vec<Result<SisterResult, SisterError>> {
        // Batch execution — single context, multiple operations
        let mut results = Vec::with_capacity(actions.len());
        for action in actions {
            results.push(self.call(action).await);
        }
        results
    }

    fn capabilities(&self) -> Vec<String> {
        self.caps.clone()
    }
}

// ═══════════════════════════════════════════════════════════
// ALL 14 BRIDGE CONSTRUCTORS — REAL MCP TOOL NAMES
// ═══════════════════════════════════════════════════════════

// Foundation Sisters (7)

pub fn memory_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Memory,
        "agentic-memory",
        "0.5.0",
        &[
            // V2 core operations
            "memory_add",
            "memory_query",
            "memory_similar",
            "memory_temporal",
            "memory_context",
            "memory_traverse",
            "memory_correct",
            "memory_resolve",
            "memory_causal",
            "memory_quality",
            "memory_stats",
            // Grounding
            "memory_ground",
            "memory_evidence",
            "memory_suggest",
            // Session
            "session_start",
            "session_end",
            "memory_session_resume",
            // Conversation
            "conversation_log",
            // V3 capture tools (Universal Fix — active capture)
            "memory_capture_message",
            "memory_capture_tool",
            "memory_capture_file",
            "memory_capture_decision",
            "memory_capture_boundary",
            // V3 retrieval tools
            "memory_retrieve",
            "memory_resurrect",
            "memory_v3_session_resume",
            "memory_search_temporal",
            "memory_search_semantic",
            "memory_search_entity",
            "memory_v3_stats",
            // V4 longevity tools (20-year memory hierarchy)
            "memory_longevity_stats",
            "memory_longevity_search",
            "memory_longevity_consolidate",
            "memory_longevity_health",
            "memory_hierarchy_query",
            "memory_hierarchy_navigate",
            "memory_hierarchy_significance",
            "memory_embedding_status",
            // Capability reporting (Universal Fix — honest reporting)
            "memory_capabilities",
            // Workspace
            "memory_workspace_create",
            "memory_workspace_switch",
            "memory_workspace_list",
            "memory_workspace_delete",
            "memory_workspace_export",
            "memory_workspace_import",
        ],
    )
}

pub fn vision_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Vision,
        "agentic-vision",
        "0.3.0",
        &[
            // Core operations
            "vision_capture",
            "vision_compare",
            "vision_query",
            "vision_ocr",
            "vision_similar",
            "vision_track",
            "vision_diff",
            "vision_health",
            "vision_link",
            // Grounding
            "vision_ground",
            "vision_evidence",
            "vision_suggest",
            // Observation
            "observation_log",
            // Workspace
            "vision_workspace_create",
            "vision_workspace_switch",
            "vision_workspace_list",
            "vision_workspace_delete",
            "vision_workspace_export",
            "vision_workspace_import",
        ],
    )
}

pub fn codebase_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Codebase,
        "agentic-codebase",
        "0.3.0",
        &[
            // Core operations
            "codebase_core",
            "codebase_session",
            "concept_find",
            "concept_map",
            "impact_analysis",
            "pattern_extract",
            "genetics_dna",
            "omniscience_search",
            "prophecy_if",
            "graph_stats",
            // Analysis
            "analysis_log",
            // Workspace
            "codebase_workspace_create",
            "codebase_workspace_switch",
            "codebase_workspace_list",
            "codebase_workspace_delete",
            "codebase_workspace_export",
            "codebase_workspace_import",
        ],
    )
}

pub fn identity_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Identity,
        "agentic-identity",
        "0.3.0",
        &[
            // Identity
            "identity_create",
            "identity_show",
            // Actions
            "action_sign",
            "action_context",
            // Trust
            "trust_grant",
            "trust_verify",
            "trust_revoke",
            // Receipts
            "receipt_create",
            "receipt_verify",
            // Continuity
            "continuity_record",
            "continuity_verify",
            // Spawning
            "spawn_create",
            "spawn_verify",
            // Competence
            "competence_record",
            "competence_query",
            // Negative knowledge
            "negative_prove",
            "negative_verify",
        ],
    )
}

pub fn time_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Time,
        "agentic-time",
        "0.2.0",
        &[
            // Deadlines
            "time_deadline_add",
            "time_deadline_check",
            "time_deadline_remove",
            // Schedules
            "time_schedule_create",
            "time_schedule_query",
            "time_schedule_update",
            // Sequences
            "time_sequence_create",
            "time_sequence_query",
            // Decay
            "time_decay_create",
            "time_decay_apply",
            // Duration
            "time_duration_estimate",
            "time_duration_track",
            // Stats & grounding
            "time_stats",
            "time_ground",
            // Workspace
            "time_workspace_create",
            "time_workspace_switch",
            "time_workspace_list",
            "time_workspace_delete",
            "time_workspace_export",
            "time_workspace_import",
        ],
    )
}

pub fn contract_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Contract,
        "agentic-contract",
        "0.2.0",
        &[
            // Contracts
            "contract_create",
            "contract_sign",
            "contract_verify",
            "contract_list",
            // Policies
            "policy_add",
            "policy_check",
            "policy_remove",
            // Risk
            "risk_limit_set",
            "risk_limit_check",
            // Approvals
            "approval_request",
            "approval_grant",
            "approval_deny",
            // Conditions
            "condition_add",
            "condition_check",
            // Obligations
            "obligation_add",
            "obligation_fulfill",
            // Violations
            "violation_list",
            "violation_report",
            // Workspace
            "contract_workspace_create",
            "contract_workspace_switch",
            "contract_workspace_list",
            "contract_workspace_delete",
            "contract_workspace_export",
            "contract_workspace_import",
        ],
    )
}

pub fn comm_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Comm,
        "agentic-comm",
        "0.2.0",
        &[
            // Consolidated domain tools (operation-based)
            "comm_channel",     // ops: create, join, leave, list, etc.
            "comm_message",     // ops: send, receive, edit, delete, etc.
            "comm_consent",     // ops: request, grant, revoke, check
            "comm_rate_limit",  // ops: check, update, reset, status
            "comm_audit",       // ops: log, query, export, retention
            "comm_federation",  // ops: connect, disconnect, list, status
            "comm_preferences", // ops: get, set, reset, export, import
            // Additional
            "comm_health",
            "comm_stats",
        ],
    )
}

// Cognitive Sisters (3)

pub fn planning_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Planning,
        "agentic-planning",
        "0.2.0",
        &[
            // Goals
            "planning_goal",
            "planning_decision",
            "planning_commitment",
            "planning_progress",
            // Advanced
            "planning_singularity",
            "planning_dream",
            "planning_sacrifice",
            "planning_entropy",
            // Workspace
            "planning_workspace_create",
            "planning_workspace_switch",
            "planning_workspace_list",
            "planning_workspace_delete",
            "planning_workspace_export",
            "planning_workspace_import",
        ],
    )
}

pub fn cognition_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Cognition,
        "agentic-cognition",
        "0.2.0",
        &[
            // User modeling
            "cognition_model_create",
            "cognition_model_update",
            "cognition_model_query",
            // Beliefs
            "cognition_belief_add",
            "cognition_belief_revise",
            "cognition_belief_query",
            // Soul reflection
            "cognition_soul_reflect",
            "cognition_soul_query",
            // Drift tracking
            "cognition_drift_track",
            "cognition_drift_query",
            // Prediction
            "cognition_predict",
            "cognition_predict_verify",
            // Bias
            "cognition_bias_detect",
            "cognition_bias_mitigate",
        ],
    )
}

pub fn reality_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Reality,
        "agentic-reality",
        "0.2.0",
        &[
            // Deployment awareness
            "reality_deployment",
            "reality_environment",
            "reality_resource",
            // Memory grounding
            "reality_memory",
            "reality_anchor",
            // Hallucination detection
            "reality_hallucination",
            "reality_verify",
            // Context
            "reality_context",
            "reality_ground",
            // Boundaries
            "reality_boundary",
            "reality_constraint",
        ],
    )
}

// Astral Sisters (4)

pub fn forge_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Forge,
        "agentic-forge",
        "0.1.0",
        &[
            // Blueprints
            "forge_blueprint_create",
            "forge_blueprint_query",
            "forge_blueprint_update",
            // Entities
            "forge_entity_add",
            "forge_entity_remove",
            "forge_entity_query",
            // Dependencies
            "forge_dependency_resolve",
            "forge_dependency_check",
            // Structure generation
            "forge_structure_generate",
            "forge_skeleton_create",
            // Integration
            "forge_integration_spec",
            "forge_test_architecture",
            // Validation
            "forge_validate",
            "forge_refine",
        ],
    )
}

pub fn aegis_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Aegis,
        "agentic-aegis",
        "0.1.0",
        &[
            // Streaming validation
            "aegis_validate_streaming",
            "aegis_validate_complete",
            // Shadow execution
            "aegis_shadow_execute",
            "aegis_shadow_compare",
            // Input/output protection
            "aegis_check_input",
            "aegis_check_output",
            // Security scanning
            "aegis_scan_security",
            "aegis_scan_vulnerability",
            // Reporting
            "aegis_report",
            "aegis_alert",
            // Policy
            "aegis_policy_check",
            "aegis_policy_enforce",
        ],
    )
}

pub fn veritas_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Veritas,
        "agentic-veritas",
        "0.1.0",
        &[
            // Intent compilation
            "veritas_compile_intent",
            "veritas_parse_intent",
            // Ambiguity detection
            "veritas_detect_ambiguity",
            "veritas_resolve_ambiguity",
            // Claim verification
            "veritas_verify_claim",
            "veritas_check_consistency",
            // Causal reasoning
            "veritas_reason_causally",
            "veritas_trace_cause",
            // Uncertainty
            "veritas_uncertainty_detect",
            "veritas_confidence_score",
        ],
    )
}

pub fn evolve_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Evolve,
        "agentic-evolve",
        "0.1.0",
        &[
            // Pattern storage
            "evolve_pattern_store",
            "evolve_pattern_query",
            "evolve_pattern_delete",
            // Signature matching
            "evolve_match_signature",
            "evolve_find_similar",
            // Crystallization
            "evolve_crystallize",
            "evolve_crystallize_status",
            // Composition
            "evolve_compose",
            "evolve_decompose",
            // Coverage
            "evolve_coverage",
            "evolve_gap_analysis",
            // Collective
            "evolve_collective_sync",
            "evolve_collective_query",
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Bridge constructor tests ───────────────────────────

    #[test]
    fn test_memory_bridge_id() {
        let b = memory_bridge();
        assert_eq!(b.id, SisterId::Memory);
        assert_eq!(b.bridge_name, "agentic-memory");
        assert_eq!(b.bridge_version, "0.5.0");
    }

    #[test]
    fn test_memory_bridge_capabilities() {
        let b = memory_bridge();
        // V2 tools
        assert!(b.caps.contains(&"memory_add".to_string()));
        assert!(b.caps.contains(&"memory_query".to_string()));
        assert!(b.caps.contains(&"memory_similar".to_string()));
        assert!(b.caps.contains(&"session_start".to_string()));
        assert!(b.caps.contains(&"conversation_log".to_string()));
        // V3 capture tools (Universal Fix)
        assert!(b.caps.contains(&"memory_capture_message".to_string()));
        assert!(b.caps.contains(&"memory_capture_decision".to_string()));
        assert!(b.caps.contains(&"memory_search_semantic".to_string()));
        // V4 longevity tools
        assert!(b.caps.contains(&"memory_longevity_search".to_string()));
        assert!(b.caps.contains(&"memory_longevity_stats".to_string()));
        assert!(b.caps.contains(&"memory_hierarchy_query".to_string()));
        // Capability reporting
        assert!(b.caps.contains(&"memory_capabilities".to_string()));
        assert!(b.caps.len() >= 40);
    }

    #[test]
    fn test_vision_bridge_id() {
        let b = vision_bridge();
        assert_eq!(b.id, SisterId::Vision);
        assert_eq!(b.bridge_name, "agentic-vision");
        assert_eq!(b.bridge_version, "0.3.0");
    }

    #[test]
    fn test_vision_bridge_capabilities() {
        let b = vision_bridge();
        assert!(b.caps.contains(&"vision_capture".to_string()));
        assert!(b.caps.contains(&"vision_ocr".to_string()));
        assert!(b.caps.contains(&"observation_log".to_string()));
    }

    #[test]
    fn test_codebase_bridge_id() {
        let b = codebase_bridge();
        assert_eq!(b.id, SisterId::Codebase);
        assert_eq!(b.bridge_name, "agentic-codebase");
        assert_eq!(b.bridge_version, "0.3.0");
    }

    #[test]
    fn test_codebase_bridge_capabilities() {
        let b = codebase_bridge();
        assert!(b.caps.contains(&"codebase_core".to_string()));
        assert!(b.caps.contains(&"omniscience_search".to_string()));
        assert!(b.caps.contains(&"analysis_log".to_string()));
    }

    #[test]
    fn test_identity_bridge_id() {
        let b = identity_bridge();
        assert_eq!(b.id, SisterId::Identity);
        assert_eq!(b.bridge_name, "agentic-identity");
    }

    #[test]
    fn test_identity_bridge_capabilities() {
        let b = identity_bridge();
        assert!(b.caps.contains(&"identity_create".to_string()));
        assert!(b.caps.contains(&"action_sign".to_string()));
        assert!(b.caps.contains(&"trust_verify".to_string()));
    }

    #[test]
    fn test_time_bridge_id() {
        let b = time_bridge();
        assert_eq!(b.id, SisterId::Time);
        assert_eq!(b.bridge_name, "agentic-time");
    }

    #[test]
    fn test_time_bridge_capabilities() {
        let b = time_bridge();
        assert!(b.caps.contains(&"time_deadline_add".to_string()));
        assert!(b.caps.contains(&"time_decay_create".to_string()));
    }

    #[test]
    fn test_contract_bridge_id() {
        let b = contract_bridge();
        assert_eq!(b.id, SisterId::Contract);
        assert_eq!(b.bridge_name, "agentic-contract");
    }

    #[test]
    fn test_contract_bridge_capabilities() {
        let b = contract_bridge();
        assert!(b.caps.contains(&"contract_create".to_string()));
        assert!(b.caps.contains(&"policy_check".to_string()));
        assert!(b.caps.contains(&"violation_report".to_string()));
    }

    #[test]
    fn test_comm_bridge_id() {
        let b = comm_bridge();
        assert_eq!(b.id, SisterId::Comm);
        assert_eq!(b.bridge_name, "agentic-comm");
    }

    #[test]
    fn test_comm_bridge_capabilities() {
        let b = comm_bridge();
        assert!(b.caps.contains(&"comm_channel".to_string()));
        assert!(b.caps.contains(&"comm_message".to_string()));
        assert!(b.caps.contains(&"comm_health".to_string()));
    }

    #[test]
    fn test_planning_bridge_id() {
        let b = planning_bridge();
        assert_eq!(b.id, SisterId::Planning);
        assert_eq!(b.bridge_name, "agentic-planning");
    }

    #[test]
    fn test_planning_bridge_capabilities() {
        let b = planning_bridge();
        assert!(b.caps.contains(&"planning_goal".to_string()));
        assert!(b.caps.contains(&"planning_singularity".to_string()));
    }

    #[test]
    fn test_cognition_bridge_id() {
        let b = cognition_bridge();
        assert_eq!(b.id, SisterId::Cognition);
        assert_eq!(b.bridge_name, "agentic-cognition");
    }

    #[test]
    fn test_cognition_bridge_capabilities() {
        let b = cognition_bridge();
        assert!(b.caps.contains(&"cognition_model_create".to_string()));
        assert!(b.caps.contains(&"cognition_bias_detect".to_string()));
    }

    #[test]
    fn test_reality_bridge_id() {
        let b = reality_bridge();
        assert_eq!(b.id, SisterId::Reality);
        assert_eq!(b.bridge_name, "agentic-reality");
    }

    #[test]
    fn test_reality_bridge_capabilities() {
        let b = reality_bridge();
        assert!(b.caps.contains(&"reality_hallucination".to_string()));
        assert!(b.caps.contains(&"reality_verify".to_string()));
    }

    #[test]
    fn test_forge_bridge_id() {
        let b = forge_bridge();
        assert_eq!(b.id, SisterId::Forge);
        assert_eq!(b.bridge_name, "agentic-forge");
        assert_eq!(b.bridge_version, "0.1.0");
    }

    #[test]
    fn test_forge_bridge_capabilities() {
        let b = forge_bridge();
        assert!(b.caps.contains(&"forge_blueprint_create".to_string()));
        assert!(b.caps.contains(&"forge_validate".to_string()));
    }

    #[test]
    fn test_aegis_bridge_id() {
        let b = aegis_bridge();
        assert_eq!(b.id, SisterId::Aegis);
        assert_eq!(b.bridge_name, "agentic-aegis");
        assert_eq!(b.bridge_version, "0.1.0");
    }

    #[test]
    fn test_aegis_bridge_capabilities() {
        let b = aegis_bridge();
        assert!(b.caps.contains(&"aegis_validate_streaming".to_string()));
        assert!(b.caps.contains(&"aegis_scan_security".to_string()));
    }

    #[test]
    fn test_veritas_bridge_id() {
        let b = veritas_bridge();
        assert_eq!(b.id, SisterId::Veritas);
        assert_eq!(b.bridge_name, "agentic-veritas");
        assert_eq!(b.bridge_version, "0.1.0");
    }

    #[test]
    fn test_veritas_bridge_capabilities() {
        let b = veritas_bridge();
        assert!(b.caps.contains(&"veritas_compile_intent".to_string()));
        assert!(b.caps.contains(&"veritas_confidence_score".to_string()));
    }

    #[test]
    fn test_evolve_bridge_id() {
        let b = evolve_bridge();
        assert_eq!(b.id, SisterId::Evolve);
        assert_eq!(b.bridge_name, "agentic-evolve");
        assert_eq!(b.bridge_version, "0.1.0");
    }

    #[test]
    fn test_evolve_bridge_capabilities() {
        let b = evolve_bridge();
        assert!(b.caps.contains(&"evolve_pattern_store".to_string()));
        assert!(b.caps.contains(&"evolve_crystallize".to_string()));
        assert!(b.caps.contains(&"evolve_collective_sync".to_string()));
    }

    // ── all_bridges tests ──────────────────────────────────

    #[test]
    fn test_all_bridges_count() {
        assert_eq!(all_bridges().len(), 14);
    }

    #[test]
    fn test_all_bridges_unique_ids() {
        let bridges = all_bridges();
        let mut ids: Vec<_> = bridges.iter().map(|b| b.id).collect();
        ids.sort_by_key(|id| format!("{:?}", id));
        ids.dedup();
        assert_eq!(ids.len(), 14);
    }

    #[test]
    fn test_all_bridges_have_capabilities() {
        for b in all_bridges() {
            assert!(!b.caps.is_empty(), "{} has no capabilities", b.bridge_name);
        }
    }

    #[test]
    fn test_all_bridges_default_available() {
        for b in all_bridges() {
            assert!(
                b.available.load(std::sync::atomic::Ordering::SeqCst),
                "{} not available by default",
                b.bridge_name
            );
        }
    }

    #[test]
    fn test_all_bridges_default_timeout() {
        for b in all_bridges() {
            assert_eq!(
                b.timeout,
                std::time::Duration::from_secs(5),
                "{} has wrong default timeout",
                b.bridge_name
            );
        }
    }

    // ── McpSisterBridge methods ────────────────────────────

    #[test]
    fn test_set_available() {
        let b = memory_bridge();
        assert!(b.available.load(std::sync::atomic::Ordering::SeqCst));
        b.set_available(false);
        assert!(!b.available.load(std::sync::atomic::Ordering::SeqCst));
        b.set_available(true);
        assert!(b.available.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_set_timeout() {
        let mut b = memory_bridge();
        b.set_timeout(std::time::Duration::from_secs(30));
        assert_eq!(b.timeout, std::time::Duration::from_secs(30));
    }

    // ── SisterBridge trait impl tests ──────────────────────

    #[tokio::test]
    async fn test_bridge_health_check_healthy() {
        let b = memory_bridge();
        assert_eq!(b.health_check().await, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_bridge_health_check_unavailable() {
        let b = memory_bridge();
        b.set_available(false);
        assert_eq!(b.health_check().await, HealthStatus::Unavailable);
    }

    #[tokio::test]
    async fn test_bridge_call_success() {
        let b = memory_bridge();
        let action = SisterAction::new("memory_add", serde_json::json!({"content": "test"}));
        let result = b.call(action).await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.data["sister"], "agentic-memory");
        assert_eq!(r.data["tool"], "memory_add");
    }

    #[tokio::test]
    async fn test_bridge_call_unavailable() {
        let b = memory_bridge();
        b.set_available(false);
        let action = SisterAction::new("memory_add", serde_json::json!({}));
        let result = b.call(action).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.retryable);
        assert_eq!(err.sister_id, SisterId::Memory);
    }

    #[tokio::test]
    async fn test_bridge_batch_call() {
        let b = memory_bridge();
        let actions = vec![
            SisterAction::new("memory_add", serde_json::json!({})),
            SisterAction::new("memory_query", serde_json::json!({})),
        ];
        let results = b.batch_call(actions).await;
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn test_bridge_batch_call_empty() {
        let b = memory_bridge();
        let results = b.batch_call(vec![]).await;
        assert!(results.is_empty());
    }

    #[test]
    fn test_bridge_capabilities_returns_correct_list() {
        let b = memory_bridge();
        let caps = b.capabilities();
        assert!(caps.contains(&"memory_add".to_string()));
    }

    #[test]
    fn test_bridge_sister_id_trait() {
        let b = vision_bridge();
        assert_eq!(b.sister_id(), SisterId::Vision);
    }

    #[test]
    fn test_bridge_name_trait() {
        let b = codebase_bridge();
        assert_eq!(b.name(), "agentic-codebase");
    }

    #[test]
    fn test_bridge_version_trait() {
        let b = identity_bridge();
        assert_eq!(b.version(), "0.3.0");
    }

    // ═══════════════════════════════════════════════════════════
    // UNIVERSAL FIX — V3/V4 Memory Bridge Tests
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_memory_bridge_v3_capture_tools() {
        let b = memory_bridge();
        let v3_capture = [
            "memory_capture_message",
            "memory_capture_tool",
            "memory_capture_file",
            "memory_capture_decision",
            "memory_capture_boundary",
        ];
        for tool in &v3_capture {
            assert!(b.caps.contains(&tool.to_string()),
                "V3 capture tool '{}' missing from memory bridge", tool);
        }
    }

    #[test]
    fn test_memory_bridge_v3_retrieval_tools() {
        let b = memory_bridge();
        let v3_retrieval = [
            "memory_retrieve",
            "memory_resurrect",
            "memory_v3_session_resume",
            "memory_search_temporal",
            "memory_search_semantic",
            "memory_search_entity",
            "memory_v3_stats",
        ];
        for tool in &v3_retrieval {
            assert!(b.caps.contains(&tool.to_string()),
                "V3 retrieval tool '{}' missing from memory bridge", tool);
        }
    }

    #[test]
    fn test_memory_bridge_v4_longevity_tools() {
        let b = memory_bridge();
        let v4_longevity = [
            "memory_longevity_stats",
            "memory_longevity_search",
            "memory_longevity_consolidate",
            "memory_longevity_health",
            "memory_hierarchy_query",
            "memory_hierarchy_navigate",
            "memory_hierarchy_significance",
            "memory_embedding_status",
        ];
        for tool in &v4_longevity {
            assert!(b.caps.contains(&tool.to_string()),
                "V4 longevity tool '{}' missing from memory bridge", tool);
        }
    }

    #[test]
    fn test_memory_bridge_capability_reporting() {
        let b = memory_bridge();
        assert!(b.caps.contains(&"memory_capabilities".to_string()),
            "memory_capabilities tool missing — honest reporting not available");
    }

    #[test]
    fn test_memory_bridge_v2_backward_compat() {
        let b = memory_bridge();
        // V2 tools must still be present for backward compatibility
        let v2_tools = [
            "memory_add", "memory_query", "memory_similar",
            "memory_temporal", "memory_context", "memory_traverse",
            "memory_correct", "memory_resolve", "memory_causal",
            "memory_quality", "memory_stats",
            "memory_ground", "memory_evidence", "memory_suggest",
            "session_start", "session_end", "memory_session_resume",
            "conversation_log",
        ];
        for tool in &v2_tools {
            assert!(b.caps.contains(&tool.to_string()),
                "V2 tool '{}' removed — backward compatibility broken", tool);
        }
    }

    #[test]
    fn test_memory_bridge_workspace_tools_preserved() {
        let b = memory_bridge();
        let workspace_tools = [
            "memory_workspace_create", "memory_workspace_switch",
            "memory_workspace_list", "memory_workspace_delete",
            "memory_workspace_export", "memory_workspace_import",
        ];
        for tool in &workspace_tools {
            assert!(b.caps.contains(&tool.to_string()),
                "Workspace tool '{}' missing", tool);
        }
    }

    #[test]
    fn test_memory_bridge_total_capabilities() {
        let b = memory_bridge();
        // V2 (24) + V3 capture (5) + V3 retrieval (7) + V4 longevity (8) + capabilities (1) + workspace (6) = 51
        // But some are counted in both V2 and workspace, so just check minimum
        assert!(b.caps.len() >= 40,
            "Memory bridge has only {} capabilities, expected >= 40", b.caps.len());
    }

    #[tokio::test]
    async fn test_memory_bridge_v3_tool_call_succeeds() {
        let b = memory_bridge();
        let action = SisterAction::new(
            "memory_capture_message",
            serde_json::json!({
                "role": "user",
                "content": "test message",
                "summary": "testing V3 capture",
            }),
        );
        let result = b.call(action).await;
        assert!(result.is_ok(), "V3 memory_capture_message call failed");
        let r = result.unwrap();
        assert_eq!(r.data["tool"], "memory_capture_message");
    }

    #[tokio::test]
    async fn test_memory_bridge_v4_tool_call_succeeds() {
        let b = memory_bridge();
        let action = SisterAction::new(
            "memory_longevity_search",
            serde_json::json!({
                "query": "test query",
                "limit": 5,
            }),
        );
        let result = b.call(action).await;
        assert!(result.is_ok(), "V4 memory_longevity_search call failed");
        let r = result.unwrap();
        assert_eq!(r.data["tool"], "memory_longevity_search");
    }

    #[tokio::test]
    async fn test_memory_bridge_capabilities_tool_call() {
        let b = memory_bridge();
        let action = SisterAction::new("memory_capabilities", serde_json::json!({}));
        let result = b.call(action).await;
        assert!(result.is_ok(), "memory_capabilities tool call failed");
    }

    #[tokio::test]
    async fn test_memory_bridge_v3_batch_capture() {
        let b = memory_bridge();
        let actions = vec![
            SisterAction::new("memory_capture_message", serde_json::json!({
                "role": "user", "content": "msg1"
            })),
            SisterAction::new("memory_capture_decision", serde_json::json!({
                "decision": "use Rust", "reasoning": "performance"
            })),
            SisterAction::new("memory_capture_boundary", serde_json::json!({
                "boundary_type": "session_end"
            })),
        ];
        let results = b.batch_call(actions).await;
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()),
            "Batch V3 capture failed: {:?}", results);
    }

    #[test]
    fn test_memory_bridge_no_duplicate_tools() {
        let b = memory_bridge();
        let mut seen = std::collections::HashSet::new();
        for cap in &b.caps {
            assert!(seen.insert(cap.clone()),
                "Duplicate tool in memory bridge: {}", cap);
        }
    }

    #[test]
    fn test_other_bridges_unchanged() {
        // Verify non-memory bridges weren't accidentally modified
        assert_eq!(vision_bridge().bridge_version, "0.3.0");
        assert_eq!(codebase_bridge().bridge_version, "0.3.0");
        assert_eq!(identity_bridge().bridge_version, "0.3.0");
        assert_eq!(time_bridge().bridge_version, "0.2.0");
        assert_eq!(contract_bridge().bridge_version, "0.2.0");
        assert_eq!(comm_bridge().bridge_version, "0.2.0");
        assert_eq!(planning_bridge().bridge_version, "0.2.0");
        assert_eq!(cognition_bridge().bridge_version, "0.2.0");
        assert_eq!(reality_bridge().bridge_version, "0.2.0");
        assert_eq!(forge_bridge().bridge_version, "0.1.0");
        assert_eq!(aegis_bridge().bridge_version, "0.1.0");
        assert_eq!(veritas_bridge().bridge_version, "0.1.0");
        assert_eq!(evolve_bridge().bridge_version, "0.1.0");
    }
}

/// Create all 14 bridges
pub fn all_bridges() -> Vec<McpSisterBridge> {
    vec![
        memory_bridge(),
        vision_bridge(),
        codebase_bridge(),
        identity_bridge(),
        time_bridge(),
        contract_bridge(),
        comm_bridge(),
        planning_bridge(),
        cognition_bridge(),
        reality_bridge(),
        forge_bridge(),
        aegis_bridge(),
        veritas_bridge(),
        evolve_bridge(),
    ]
}
