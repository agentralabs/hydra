//! LLM tool routing — maps intent categories to the top MCP tools the LLM should know about.
//!
//! Extracted from llm_helpers_commands.rs to allow expanded routing across all 14 sisters.
//! Each intent gets 10-25 of the most relevant tools from the ~735 available.

use crate::cognitive::intent_router::{IntentCategory, ClassifiedIntent};
use crate::sisters::Sisters;

/// Build the tool list for a classified intent. Returns tool names the LLM can call.
pub(crate) fn route_tools_for_intent(
    intent: &ClassifiedIntent,
    sisters: &Sisters,
    user_text: &str,
    complexity: &str,
    is_action: bool,
) -> Vec<String> {
    let mut tools: Vec<String> = Vec::new();

    match intent.category {
        // Memory recall → memory + cognition + counterfactual
        IntentCategory::MemoryRecall => {
            tools.extend(sisters.tools_for_sister("memory", &[
                "memory_query", "memory_similar", "memory_temporal",
                "memory_context", "memory_search",
                "memory_traverse", "memory_prophecy", "memory_counterfactual_what_if",
                "memory_dejavu_check", "memory_predict",
                "memory_immortal_stats", "memory_compress", "memory_v3_search_semantic",
                "memory_core", "memory_longevity_stats", "memory_hierarchy_query",
            ]));
            tools.extend(sisters.tools_for_sister("cognition", &[
                "cognition_belief_query", "cognition_belief_graph",
                "cognition_model_portrait",
            ]));
        }
        // Code tasks → full code agent pipeline
        IntentCategory::CodeBuild | IntentCategory::CodeFix | IntentCategory::CodeExplain => {
            tools.extend(sisters.tools_for_sister("forge", &[
                "forge_blueprint_create", "forge_blueprint_validate",
                "forge_skeleton_create", "forge_test_generate",
                "forge_dependency_resolve", "forge_wiring_create",
                "forge_structure_generate",
            ]));
            tools.extend(sisters.tools_for_sister("aegis", &[
                "aegis_scan_security", "aegis_validate_complete",
                "aegis_confidence_score", "aegis_correction_hint",
            ]));
            tools.extend(sisters.tools_for_sister("codebase", &[
                "search_semantic", "concept_find", "impact_analyze",
                "symbol_lookup", "architecture_infer", "pattern_check",
                "hallucination_check", "prophecy", "regression_predict",
                "concept_explain", "omniscience_best", "omniscience_vuln",
                "analyse_unit", "explain_coupling", "codebase_core",
            ]));
            tools.extend(sisters.tools_for_sister("memory", &["memory_query", "memory_similar"]));
            tools.extend(sisters.tools_for_sister("veritas", &[
                "veritas_extract_claims", "veritas_check_consistency",
            ]));
            tools.extend(sisters.tools_for_sister("identity", &[
                "identity_competence_show", "identity_competence_prove",
            ]));
        }
        // Planning → planning + time + memory + reality + counterfactual + contract
        IntentCategory::PlanningQuery => {
            tools.extend(sisters.tools_for_sister("planning", &[
                "planning_goal", "planning_progress", "planning_decision",
                "planning_counterfactual", "planning_chain", "planning_suggest",
                "planning_consensus", "planning_dream",
            ]));
            tools.extend(sisters.tools_for_sister("time", &[
                "time_deadline_check", "time_deadline_add", "time_schedule_create",
                "time_duration_estimate", "time_debt_analyze",
                "time_future_memory", "time_timeline_fork",
            ]));
            tools.extend(sisters.tools_for_sister("memory", &["memory_query", "memory_temporal"]));
            tools.extend(sisters.tools_for_sister("reality", &["reality_stakes", "reality_coherence"]));
            tools.extend(sisters.tools_for_sister("contract", &[
                "contract_create", "policy_check", "contract_stats",
            ]));
        }
        // Web/browse → full Vision browser agent toolkit
        IntentCategory::WebBrowse => {
            tools.extend(sisters.tools_for_sister("vision", &[
                "vision_dom_extract", "vision_intent_extract",
                "vision_grammar_learn", "vision_grammar_get",
                "vision_perception_route", "vision_capture",
                "vision_query", "vision_ocr", "vision_web_map",
                "vision_diff", "vision_compare", "vision_ground", "vision_evidence",
                "vision_forensic_diff", "vision_semantic_analyze",
                "vision_hallucination_check", "vision_prophecy",
                "vision_truth_check", "vision_compare_sites",
            ]));
        }
        // Communication → comm + affect + collaboration + identity
        IntentCategory::Communicate => {
            tools.extend(sisters.tools_for_sister("comm", &[
                "comm_message", "comm_channel", "comm_federation", "comm_send",
                "comm_notify", "comm_affect", "comm_collaboration", "comm_semantic",
                "comm_trust", "comm_forensics", "comm_workspace",
            ]));
            tools.extend(sisters.tools_for_sister("identity", &[
                "identity_trust_level", "identity_team_create", "identity_team_act",
            ]));
        }
        // Unknown/Question → smart routing by keyword detection
        IntentCategory::Unknown | IntentCategory::Question => {
            route_unknown_intent(&mut tools, sisters, user_text, complexity, is_action);
        }
        _ => {}
    }

    tools
}

/// Route tools for Unknown/Question intents using keyword detection.
fn route_unknown_intent(
    tools: &mut Vec<String>,
    sisters: &Sisters,
    user_text: &str,
    complexity: &str,
    is_action: bool,
) {
    let lower = user_text.to_lowercase();
    let needs_identity = lower.contains("receipt") || lower.contains("prove")
        || lower.contains("trust") || lower.contains("what did you")
        || lower.contains("what have you") || lower.contains("last action")
        || lower.contains("identity") || lower.contains("spawn")
        || lower.contains("competence") || lower.contains("reputation");
    let needs_time = lower.contains("deadline") || lower.contains("schedule")
        || lower.contains("when") || lower.contains("how long")
        || lower.contains("timeline") || lower.contains("decay")
        || lower.contains("anchor") || lower.contains("future memory");
    let needs_planning = lower.contains("goal") || lower.contains("plan")
        || lower.contains("what should") || lower.contains("next step");
    let needs_memory = lower.contains("remember") || lower.contains("recall")
        || lower.contains("forgot") || lower.contains("last time")
        || lower.contains("prophecy") || lower.contains("deja vu");
    let needs_code = lower.contains("code") || lower.contains("function")
        || lower.contains("file") || lower.contains("bug") || lower.contains("error");
    let needs_vision = lower.contains("page") || lower.contains("website")
        || lower.contains("screenshot") || lower.contains("browse");
    let needs_contract = lower.contains("policy") || lower.contains("approval")
        || lower.contains("permission") || lower.contains("allowed")
        || lower.contains("contract") || lower.contains("obligation")
        || lower.contains("violation") || lower.contains("risk limit");
    let needs_cognition = lower.contains("belief") || lower.contains("model")
        || lower.contains("personality") || lower.contains("predict")
        || lower.contains("simulate") || lower.contains("soul");

    if needs_identity {
        tools.extend(sisters.tools_for_sister("identity", &[
            "identity_show", "receipt_list", "identity_trust_level",
            "identity_receipt_search", "identity_trust_history",
            "identity_competence_show", "identity_reputation_get",
            "identity_ground", "identity_fingerprint_build",
        ]));
    }
    if needs_time {
        tools.extend(sisters.tools_for_sister("time", &[
            "time_schedule_create", "time_deadline_add", "time_deadline_check",
            "time_duration_estimate", "time_timeline_fork", "time_anchor_create",
            "time_future_memory", "time_debt_analyze",
        ]));
    }
    if needs_planning {
        tools.extend(sisters.tools_for_sister("planning", &[
            "planning_goal", "planning_progress", "planning_suggest",
        ]));
    }
    if needs_memory {
        tools.extend(sisters.tools_for_sister("memory", &[
            "memory_query", "memory_similar", "memory_traverse",
            "memory_prophecy", "memory_dejavu_check",
            "memory_predict", "memory_counterfactual_what_if",
        ]));
    }
    if needs_code {
        tools.extend(sisters.tools_for_sister("codebase", &[
            "search_semantic", "symbol_lookup", "impact_analyze", "concept_find",
        ]));
    }
    if needs_vision {
        tools.extend(sisters.tools_for_sister("vision", &[
            "vision_dom_extract", "vision_capture", "vision_web_map",
            "vision_semantic_analyze", "vision_truth_check",
        ]));
    }
    if needs_contract {
        tools.extend(sisters.tools_for_sister("contract", &[
            "policy_check", "approval_request", "risk_limit_check",
            "contract_stats", "contract_list", "contract_ground",
            "obligation_check", "violation_list",
            "trust_gradient_evaluate", "risk_prophecy_forecast",
            "contract_simulation_run", "smart_escalation_route",
            "policy_omniscience_query", "violation_precognition_predict",
        ]));
    }
    if needs_cognition {
        tools.extend(sisters.tools_for_sister("cognition", &[
            "cognition_belief_query", "cognition_belief_graph",
            "cognition_model_portrait", "cognition_predict",
            "cognition_simulate", "cognition_soul_reflect",
            "cognition_shadow_map",
        ]));
    }

    if complexity == "complex" || is_action {
        if !needs_memory {
            tools.extend(sisters.tools_for_sister("memory", &[
                "memory_query", "memory_context", "memory_similar",
            ]));
        }
        if !needs_code {
            tools.extend(sisters.tools_for_sister("codebase", &[
                "symbol_lookup", "search_semantic",
            ]));
        }
        if !needs_identity {
            tools.extend(sisters.tools_for_sister("identity", &[
                "identity_show", "receipt_list", "identity_trust_level",
            ]));
        }
        if !needs_planning {
            tools.extend(sisters.tools_for_sister("planning", &[
                "planning_goal", "planning_progress",
            ]));
        }
        if !needs_cognition {
            tools.extend(sisters.tools_for_sister("cognition", &[
                "cognition_predict", "cognition_shadow_map",
            ]));
        }
        if !needs_time {
            tools.extend(sisters.tools_for_sister("time", &[
                "time_deadline_check", "time_duration_estimate",
            ]));
        }
        tools.extend(sisters.tools_for_sister("reality", &[
            "reality_deployment", "reality_environment",
        ]));
        tools.extend(sisters.tools_for_sister("veritas", &[
            "veritas_compile_intent", "veritas_check_consistency",
        ]));
        tools.extend(sisters.tools_for_sister("aegis", &["aegis_validate_complete"]));
        if !needs_contract {
            tools.extend(sisters.tools_for_sister("contract", &[
                "policy_check", "contract_stats",
            ]));
        }
        tools.truncate(30);
    }
}
