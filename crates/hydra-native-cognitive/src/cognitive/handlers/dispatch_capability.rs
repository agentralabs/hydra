//! Capability-based dispatch — routes matched capabilities to their handlers.
//!
//! This runs BEFORE intent classification. If a capability matches,
//! we skip the LLM classifier entirely and route directly.
//! Heavy handlers live in dispatch_capability_exec.rs (400-line split).
//!
//! NOTE: Only LOCAL operations belong here (swarm, env, threat, tasks).
//! LLM-powered operations (sister improve, self-implement) route via intent dispatch.

use std::sync::Arc;
use tokio::sync::mpsc;
use parking_lot::RwLock;

use crate::cognitive::capability_registry::{CapabilityHandler, CapabilityMatch, CapabilityRegistry};
use crate::cognitive::loop_runner::CognitiveUpdate;
use crate::sisters::SistersHandle;
use crate::swarm::SwarmManager;
use crate::threat::ThreatCorrelator;
use crate::remote::RemoteExecutor;
use super::dispatch_capability_exec::{
    handle_swarm, handle_threat, handle_remote_exec, send_msg,
};

/// Check if user input matches a registered capability and dispatch it.
/// Returns `true` if handled (caller should return early).
pub(crate) async fn handle_capability_match(
    text: &str,
    swarm_manager: &Option<Arc<SwarmManager>>,
    threat_correlator: &Option<Arc<RwLock<ThreatCorrelator>>>,
    remote_executor: &Option<Arc<RwLock<RemoteExecutor>>>,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let registry = CapabilityRegistry::new();
    let matched = match registry.match_intent(text) {
        Some(m) => m,
        None => return false,
    };

    eprintln!(
        "[hydra:capability] Matched '{}' (score={:.1}) → {:?}",
        matched.capability.name, matched.score, matched.capability.handler
    );

    match matched.capability.handler {
        CapabilityHandler::ProjectExec => handle_project_exec(&matched, sisters_handle, tx).await,
        CapabilityHandler::Swarm => handle_swarm(text, swarm_manager, sisters_handle, tx).await,
        CapabilityHandler::ThreatCheck => handle_threat(threat_correlator, tx),
        CapabilityHandler::EnvironmentProbe => handle_env_probe(tx),
        CapabilityHandler::RemoteExec => handle_remote_exec(text, remote_executor, tx).await,
        CapabilityHandler::TaskList => handle_tasks(tx),
    }
}

async fn handle_project_exec(
    matched: &CapabilityMatch<'_>,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let url = match &matched.extracted_url {
        Some(u) => u.clone(),
        None => {
            send_msg(tx, "I need a repository URL to test.\n\nExample: `test https://github.com/user/repo`");
            return true;
        }
    };
    if let Some(ref sh) = sisters_handle {
        sh.comm_broadcast("hydra", "project_exec", &format!("Testing repo: {}", url)).await;
    }
    super::dispatch_actions::handle_project_exec_natural(
        &format!("test {}", url), sisters_handle, tx,
    ).await
}

fn handle_env_probe(tx: &mpsc::UnboundedSender<CognitiveUpdate>) -> bool {
    let _ = tx.send(CognitiveUpdate::Phase("Environment Probe".into()));
    let profile = crate::sisters::reality_deep::local_probe();
    let mut c = String::from("**Environment Profile**\n\n");
    c.push_str(&format!("  OS: {} ({})\n", profile.os, profile.arch));
    if !profile.tools.is_empty() {
        c.push_str(&format!("  Tools: {}\n", profile.tools.join(", ")));
    }
    if !profile.services.is_empty() {
        c.push_str(&format!("  Services: {}\n", profile.services.join(", ")));
    }
    if let Some(ref rt) = profile.container_runtime {
        c.push_str(&format!("  Container: {}\n", rt));
    }
    send_msg(tx, &c);
    true
}

fn handle_tasks(tx: &mpsc::UnboundedSender<CognitiveUpdate>) -> bool {
    let _ = tx.send(CognitiveUpdate::Phase("Tasks".into()));
    let persister = crate::task_persistence::TaskPersister::new();
    match persister.list_incomplete() {
        Ok(tasks) if !tasks.is_empty() => {
            let mut msg = format!("**Tasks** ({} interrupted/active)\n\n", tasks.len());
            for t in &tasks {
                msg.push_str(&format!("  - `{}` — {} ({:.0}% done, phase: {})\n",
                    t.task_id, format!("{:?}", t.task_type), t.progress * 100.0, t.phase));
            }
            send_msg(tx, &msg);
        }
        _ => send_msg(tx, "No active or interrupted tasks."),
    }
    true
}

/// Auto-resolve a project name from natural language to a sibling workspace directory.
/// Scans the workspace parent for directories whose names match words in the text.
/// Fully generic — works on any system, any project layout.
pub(crate) fn resolve_project_from_text(text: &str) -> Option<std::path::PathBuf> {
    let lower = text.to_lowercase();
    let workspace_parent = std::env::current_dir().ok()?.parent()?.to_path_buf();

    let stop = ["improve", "the", "sister", "fix", "upgrade", "patch", "make",
                 "better", "add", "to", "for", "a", "an", "my", "at", "in", "on"];
    let words: Vec<&str> = lower.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-'))
        .filter(|w| w.len() >= 2 && !stop.contains(w))
        .collect();

    let entries = std::fs::read_dir(&workspace_parent).ok()?;
    let mut best: Option<(std::path::PathBuf, usize)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }
        let dir_name = path.file_name()?.to_str()?.to_lowercase();
        let score = words.iter().filter(|w| dir_name.contains(*w)).count();
        if score > 0 && best.as_ref().map_or(true, |b| score > b.1) {
            best = Some((path, score));
        }
    }
    best.map(|(p, _)| p)
}

#[cfg(test)]
mod tests {
    use super::super::dispatch_capability_exec::extract_number;

    #[test]
    fn test_extract_number() {
        assert_eq!(extract_number("deploy 10 agents"), Some(10));
        assert_eq!(extract_number("spawn 5 workers"), Some(5));
        assert_eq!(extract_number("hello world"), None);
    }
}
