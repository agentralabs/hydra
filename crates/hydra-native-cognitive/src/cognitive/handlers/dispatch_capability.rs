//! Capability-based dispatch — routes matched capabilities to their handlers.
//!
//! This runs BEFORE intent classification. If a capability matches,
//! we skip the LLM classifier entirely and route directly.
//! Heavy handlers live in dispatch_capability_exec.rs (400-line split).

use std::sync::Arc;
use tokio::sync::mpsc;
use parking_lot::RwLock;

use crate::cognitive::capability_registry::{CapabilityHandler, CapabilityMatch, CapabilityRegistry};
use crate::cognitive::loop_runner::CognitiveUpdate;
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
        CapabilityHandler::ProjectExec => handle_project_exec(&matched, tx).await,
        CapabilityHandler::Swarm => handle_swarm(text, swarm_manager, tx).await,
        CapabilityHandler::ThreatCheck => handle_threat(threat_correlator, tx),
        CapabilityHandler::EnvironmentProbe => handle_env_probe(tx),
        CapabilityHandler::SisterImprove => handle_sister_improve(text, tx).await,
        CapabilityHandler::RemoteExec => handle_remote_exec(text, remote_executor, tx).await,
        CapabilityHandler::TaskList => handle_tasks(tx),
    }
}

async fn handle_project_exec(
    matched: &CapabilityMatch<'_>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let url = match &matched.extracted_url {
        Some(u) => u.clone(),
        None => {
            send_msg(tx, "I need a repository URL to test.\n\nExample: `test https://github.com/user/repo`");
            return true;
        }
    };
    super::dispatch_actions::handle_project_exec_natural(
        &format!("test {}", url), tx,
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

async fn handle_sister_improve(
    text: &str,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let _ = tx.send(CognitiveUpdate::Phase("Sister Improvement".into()));
    let path = match crate::sister_improve::extract_sister_path(text) {
        Some(p) => p,
        None => {
            send_msg(tx, "I need a path to the sister project.\n\n\
                Example: `improve sister at ../agentic-memory add retry logic`");
            return true;
        }
    };
    let goal = crate::sister_improve::extract_goal(text);
    let tx_c = tx.clone();
    tokio::spawn(async move {
        let improver = crate::sister_improve::SisterImprover::new();
        let (improve_tx, mut _rx) = mpsc::channel(100);
        let report = improver.improve(&path, &goal, &improve_tx).await;
        send_msg(&tx_c, &format!("**Result:** {}", report.summary()));
    });
    true
}

#[cfg(test)]
mod tests {
    use super::super::dispatch_capability_exec::{extract_number};

    #[test]
    fn test_extract_number() {
        assert_eq!(extract_number("deploy 10 agents"), Some(10));
        assert_eq!(extract_number("spawn 5 workers"), Some(5));
        assert_eq!(extract_number("hello world"), None);
    }
}
