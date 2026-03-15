use crate::bridge::*;
use super::bridges_core::McpSisterBridge;

// ═══════════════════════════════════════════════════════════
// REMAINING BRIDGE CONSTRUCTORS — time through evolve
// ═══════════════════════════════════════════════════════════

// Foundation Sisters (continued)

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

// ═══════════════════════════════════════════════════════════
// UTILITY SISTERS — data, connect, workflow
// ═══════════════════════════════════════════════════════════

pub fn data_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Data,
        "agentic-data",
        "0.1.0",
        &[
            "data_schema_infer",
            "data_schema_validate",
            "data_format_detect",
            "data_format_convert",
            "data_quality_score",
            "data_quality_report",
            "data_dna_trace",
            "data_dna_lineage",
            "data_query_natural",
            "data_query_structured",
            "data_transform_apply",
            "data_redact_detect",
            "data_vault_store",
            "data_vault_retrieve",
        ],
    )
}

pub fn connect_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Connect,
        "agentic-connect",
        "0.1.0",
        &[
            "connect_protocol_detect",
            "connect_protocol_test",
            "connect_auth_configure",
            "connect_auth_test",
            "connect_soul_inspect",
            "connect_soul_refresh",
            "connect_retry_configure",
            "connect_retry_status",
            "connect_api_request",
            "connect_api_graphql",
            "connect_browse_navigate",
            "connect_browse_extract",
            "connect_security_tls",
            "connect_security_sentinel",
        ],
    )
}

pub fn workflow_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Workflow,
        "agentic-workflow",
        "0.1.0",
        &[
            "workflow_dag_create",
            "workflow_dag_validate",
            "workflow_execute_start",
            "workflow_execute_status",
            "workflow_schedule_create",
            "workflow_schedule_list",
            "workflow_trigger_add",
            "workflow_trigger_remove",
            "workflow_resilience_retry",
            "workflow_resilience_circuit",
            "workflow_governance_approve",
            "workflow_governance_audit",
            "workflow_template_create",
            "workflow_template_apply",
        ],
    )
}
