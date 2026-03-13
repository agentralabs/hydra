//! Sister diagnostics handler — direct sister health check (no LLM needed).
//!
//! Extracted from implement_diagnose.rs for file size compliance.

use tokio::sync::mpsc;

use crate::sisters::SistersHandle;

use super::super::super::loop_runner::CognitiveUpdate;
use super::super::super::intent_router::{IntentCategory, ClassifiedIntent};

/// Handle sister diagnostics — direct sister health check (no LLM needed).
pub(crate) async fn handle_sister_diagnose(
    text: &str,
    intent: &ClassifiedIntent,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    // Skip SisterDiagnose if the user is asking about policies/rules/capabilities — let LLM handle it
    let lower_for_policy = text.to_lowercase();
    let is_policy_query = lower_for_policy.contains("policy") || lower_for_policy.contains("policies")
        || lower_for_policy.contains("rules") || lower_for_policy.contains("what does")
        || lower_for_policy.contains("capabilities") || lower_for_policy.contains("what can");
    if intent.category != IntentCategory::SisterDiagnose || is_policy_query {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Diagnostics".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    if let Some(ref sh) = sisters_handle {
        let target_sister = intent.target.clone();
        let mut report = String::new();

        // Header
        report.push_str("## Sister Diagnostics\n\n");

        // Overall status
        let connected = sh.connected_count();
        report.push_str(&format!("**{}/14 sisters connected**\n\n", connected));

        // Per-sister detail
        report.push_str("| Sister | Status | Tools |\n|--------|--------|-------|\n");
        for (name, opt) in sh.all_sisters() {
            let (status, tools) = if let Some(conn) = opt {
                ("ONLINE", conn.tools.len().to_string())
            } else {
                ("OFFLINE", "-".to_string())
            };
            let icon = if opt.is_some() { "🟢" } else { "🔴" };
            report.push_str(&format!("| {} {} | {} | {} |\n", icon, name, status, tools));
        }

        // If user asked about a specific sister, do a deeper probe
        if let Some(ref target) = target_sister {
            report.push_str(&format!("\n### Deep Probe: {}\n\n", target));
            let probe_result = deep_probe_sister(sh, target).await;
            report.push_str(&probe_result);
        }

        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: report,
            css_class: "message hydra diagnostics".into(),
        });
    } else {
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: "No sisters available — running in offline mode.".into(),
            css_class: "message hydra error".into(),
        });
    }

    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Deep probe a specific sister by name.
async fn deep_probe_sister(sh: &SistersHandle, target: &str) -> String {
    match target.to_lowercase().as_str() {
        "memory" | "agenticmemory" => {
            if let Some(mem) = &sh.memory {
                let r = mem.call_tool("memory_longevity_stats", serde_json::json!({})).await;
                match r {
                    Ok(v) => format!("Memory stats: {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                    Err(e) => format!("Memory probe FAILED: {}", e),
                }
            } else {
                "Memory sister is NOT connected.".to_string()
            }
        }
        "identity" | "agenticidentity" => {
            if let Some(id) = &sh.identity {
                let r = id.call_tool("identity_whoami", serde_json::json!({})).await;
                match r {
                    Ok(v) => format!("Identity probe: {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                    Err(e) => format!("Identity probe FAILED: {}", e),
                }
            } else {
                "Identity sister is NOT connected.".to_string()
            }
        }
        "cognition" | "agenticcognition" => {
            if let Some(cog) = &sh.cognition {
                let r = cog.call_tool("cognition_model_query", serde_json::json!({"context": "diagnostic"})).await;
                match r {
                    Ok(v) => format!("Cognition probe: {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                    Err(e) => format!("Cognition probe FAILED: {}", e),
                }
            } else {
                "Cognition sister is NOT connected.".to_string()
            }
        }
        _ => {
            // Generic: check if the named sister is connected
            let found = sh.all_sisters().iter()
                .find(|(n, _)| n.to_lowercase() == target.to_lowercase())
                .map(|(_, opt)| opt.is_some());
            match found {
                Some(true) => format!("{} sister is connected and responsive.", target),
                Some(false) => format!("{} sister is NOT connected. It failed to spawn at startup.", target),
                None => format!("Unknown sister: {}. Known sisters: Memory, Identity, Codebase, Vision, Comm, Contract, Time, Planning, Cognition, Reality, Forge, Aegis, Veritas, Evolve.", target),
            }
        }
    }
}
