//! Execution handlers for capability dispatch — swarm, remote, threat.
//! Split from dispatch_capability.rs for the 400-line file limit.

use std::sync::Arc;
use tokio::sync::mpsc;
use parking_lot::RwLock;

use crate::cognitive::loop_runner::CognitiveUpdate;
use crate::sisters::SistersHandle;
use crate::swarm::SwarmManager;
use crate::threat::ThreatCorrelator;
use crate::remote::RemoteExecutor;

/// Route to swarm manager — handles both natural language and slash subcommands.
pub(crate) async fn handle_swarm(
    text: &str,
    swarm_manager: &Option<Arc<SwarmManager>>,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let mgr = match swarm_manager {
        Some(m) => m.clone(),
        None => {
            send_msg(tx, "Swarm manager not initialized.");
            return true;
        }
    };
    let _ = tx.send(CognitiveUpdate::Phase("Agent Swarm".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    let lower = text.to_lowercase();
    let sub = if lower.starts_with("/swarm") {
        lower.strip_prefix("/swarm").unwrap_or("").trim()
            .split_whitespace().next().unwrap_or("").to_string()
    } else { String::new() };

    match sub.as_str() {
        "status" => { send_msg(tx, &mgr.status_summary()); true }
        "results" => {
            let report = mgr.collect_results();
            let _ = tx.send(CognitiveUpdate::SwarmResults {
                total: report.total_agents, succeeded: report.succeeded,
                failed: report.failed, summary: report.display(),
            });
            send_msg(tx, &report.display()); true
        }
        "kill-all" => {
            let tx_c = tx.clone();
            let sh = sisters_handle.clone();
            tokio::spawn(async move {
                mgr.kill_all().await;
                if let Some(ref s) = sh { s.comm_broadcast("hydra", "swarm_event", "All agents terminated").await; }
                send_msg(&tx_c, "All agents terminated.");
            });
            true
        }
        "kill" => {
            let args = text.split_whitespace().nth(2).unwrap_or("");
            if args.is_empty() {
                send_msg(tx, "Usage: `/swarm kill <agent-id-prefix>`");
            } else if let Some(id) = mgr.find_agent(args) {
                let tx_c = tx.clone();
                tokio::spawn(async move {
                    match mgr.kill_agent(&id).await {
                        Ok(()) => send_msg(&tx_c, &format!("Terminated agent `{}`", &id[..id.len().min(12)])),
                        Err(e) => send_msg(&tx_c, &format!("Failed: {}", e)),
                    }
                });
            } else {
                send_msg(tx, &format!("Agent matching '{}' not found.", args));
            }
            true
        }
        "assign" => {
            let goal = text.split_whitespace().skip(2).collect::<Vec<_>>().join(" ");
            if goal.is_empty() {
                send_msg(tx, "Usage: `/swarm assign <goal>`");
            } else {
                let assignments = mgr.assign_task(&goal);
                if assignments.is_empty() {
                    send_msg(tx, "No idle agents. Spawn first: `/swarm spawn <N>`.");
                } else {
                    let mut msg = format!("Assigned **{}** tasks:\n\n", assignments.len());
                    for a in &assignments {
                        let _ = tx.send(CognitiveUpdate::SwarmTaskAssigned {
                            agent_id: a.agent_id[..a.agent_id.len().min(12)].to_string(),
                            task_desc: a.task.description.clone(),
                        });
                        msg.push_str(&format!("  - `{}` → {}\n",
                            &a.agent_id[..a.agent_id.len().min(12)], a.task.description));
                    }
                    send_msg(tx, &msg);
                }
            }
            true
        }
        "scale" => {
            let n = text.split_whitespace().nth(2)
                .and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
            let tx_c = tx.clone();
            tokio::spawn(async move { send_msg(&tx_c, &mgr.scale_to(n).await); });
            true
        }
        "help" | "" if lower.starts_with("/swarm") && !lower.contains("spawn") => {
            send_msg(tx, SWARM_HELP); true
        }
        _ if is_stop_request(&lower) => {
            let tx_c = tx.clone();
            let sh = sisters_handle.clone();
            tokio::spawn(async move {
                mgr.kill_all().await;
                if let Some(ref s) = sh { s.comm_broadcast("hydra", "swarm_event", "All agents terminated").await; }
                send_msg(&tx_c, "All agents terminated.");
                let _ = tx_c.send(CognitiveUpdate::ResetIdle);
            });
            true
        }
        _ => handle_swarm_spawn(text, &mgr, sisters_handle, tx).await,
    }
}

async fn handle_swarm_spawn(
    text: &str, mgr: &SwarmManager, sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let lower = text.to_lowercase();
    let count = extract_number(&lower).unwrap_or(3);
    let mgr = mgr.clone_handle();
    let tx_c = tx.clone();
    let text_owned = text.to_string();
    let sh = sisters_handle.clone();
    tokio::spawn(async move {
        let results = mgr.spawn_local(count, crate::swarm::AgentRole::Worker, vec![]).await;
        let ok = results.iter().filter(|r| r.is_ok()).count();
        let ids: Vec<String> = results.iter().filter_map(|r| r.as_ref().ok().cloned()).collect();
        let _ = tx_c.send(CognitiveUpdate::SwarmSpawned { count: ok, agent_ids: ids.clone() });
        let mut msg = format!("Spawned **{}** agents\n\n", ok);
        for id in &ids { msg.push_str(&format!("  - `{}`\n", &id[..id.len().min(12)])); }
        let has_task = !text_owned.to_lowercase().starts_with("/swarm")
            && !extract_roles(&text_owned.to_lowercase()).is_empty();
        if has_task {
            let assignments = mgr.assign_task(&text_owned);
            if !assignments.is_empty() {
                msg.push_str(&format!("\nAssigned **{}** tasks:\n", assignments.len()));
                for a in &assignments {
                    let _ = tx_c.send(CognitiveUpdate::SwarmTaskAssigned {
                        agent_id: a.agent_id[..a.agent_id.len().min(12)].to_string(),
                        task_desc: a.task.description.clone(),
                    });
                    msg.push_str(&format!("  - `{}` → {}\n",
                        &a.agent_id[..a.agent_id.len().min(12)], a.task.description));
                }
            }
        }
        msg.push_str(&format!("\n{}", mgr.status_summary()));
        // Broadcast swarm event via Comm for cross-session awareness
        if let Some(ref s) = sh {
            s.comm_broadcast("hydra", "swarm_event", &format!("Spawned {} agents", ok)).await;
        }
        send_msg(&tx_c, &msg);
        let _ = tx_c.send(CognitiveUpdate::Phase("Done".into()));
        let _ = tx_c.send(CognitiveUpdate::IconState("success".into()));
    });
    true
}

pub(crate) fn handle_threat(
    correlator: &Option<Arc<RwLock<ThreatCorrelator>>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let _ = tx.send(CognitiveUpdate::Phase("Threat Intelligence".into()));
    match correlator {
        Some(c) => {
            let mut tc = c.write();
            let assessments = tc.correlate();
            let mut msg = tc.summary();
            msg.push_str("\n\n");
            msg.push_str(&tc.patterns_summary());
            if !assessments.is_empty() {
                msg.push_str(&format!("\n\n**Active Threats:** {}\n", assessments.len()));
                for a in &assessments {
                    msg.push_str(&format!("  - {} (level {:?})\n", a.description, a.threat_level));
                }
            }
            msg.push_str(&format!("\n{}", tc.signal_history(10)));
            send_msg(tx, &msg);
        }
        None => {
            let tc = ThreatCorrelator::new();
            send_msg(tx, &format!("{}\n\n{}", tc.summary(), tc.patterns_summary()));
        }
    }
    true
}

pub(crate) async fn handle_remote_exec(
    text: &str,
    _executor: &Option<Arc<RwLock<RemoteExecutor>>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let _ = tx.send(CognitiveUpdate::Phase("Remote Execution".into()));
    let parts: Vec<&str> = text.split_whitespace().collect();
    let target = extract_ssh_target(&parts);
    let (user_host, cmd_start) = match target {
        Some((uh, idx)) => (uh, idx),
        None => {
            send_msg(tx, "Usage: `/ssh-exec user@host command`\n\nOr: `run on user@server ls -la`");
            return true;
        }
    };
    let (user, host) = match user_host.split_once('@') {
        Some((u, h)) => (u.to_string(), h.to_string()),
        None => ("root".to_string(), user_host.to_string()),
    };
    let command = parts[cmd_start..].join(" ");
    if command.is_empty() {
        send_msg(tx, &format!("Target: {}@{}\nProvide a command to execute.", user, host));
        return true;
    }
    match crate::remote::classify_command(&command) {
        crate::remote::CommandSafety::Blocked(r) => { send_msg(tx, &format!("**Blocked**: {}", r)); return true; }
        crate::remote::CommandSafety::RequiresApproval(r) => { send_msg(tx, &format!("**Requires approval**: {}", r)); return true; }
        crate::remote::CommandSafety::Safe => {}
    }
    // Execute SSH directly — no lock needed across await
    let tx_c = tx.clone();
    tokio::spawn(async move {
        match crate::remote::ssh_execute(&host, &user, &command).await {
            Ok(output) => {
                let mut msg = format!("**{}@{}** `{}`\n\n", user, host, command);
                if !output.stdout.is_empty() { msg.push_str(&format!("```\n{}\n```\n", output.stdout)); }
                if !output.stderr.is_empty() { msg.push_str(&format!("**stderr:**\n```\n{}\n```\n", output.stderr)); }
                msg.push_str(&format!("Exit code: {}", output.exit_code));
                send_msg(&tx_c, &msg);
            }
            Err(e) => send_msg(&tx_c, &format!("SSH execution failed: {}", e)),
        }
    });
    true
}

fn extract_ssh_target<'a>(parts: &[&'a str]) -> Option<(&'a str, usize)> {
    for (i, part) in parts.iter().enumerate() {
        if part.contains('@') { return Some((part, i + 1)); }
    }
    None
}

/// Detect if natural language is asking to stop/kill/terminate agents.
fn is_stop_request(lower: &str) -> bool {
    (lower.contains("stop") || lower.contains("kill") || lower.contains("terminate")
        || lower.contains("shut down") || lower.contains("shutdown"))
        && (lower.contains("agent") || lower.contains("swarm") || lower.contains("worker"))
}

pub(super) fn send_msg(tx: &mpsc::UnboundedSender<CognitiveUpdate>, content: &str) {
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: content.to_string(),
        css_class: "message hydra".into(),
    });
    let _ = tx.send(CognitiveUpdate::ResetIdle);
}

pub(super) fn extract_number(text: &str) -> Option<usize> {
    for word in text.split_whitespace() {
        if let Ok(n) = word.parse::<usize>() {
            if n > 0 && n <= 100 { return Some(n); }
        }
    }
    None
}

fn extract_roles(text: &str) -> Vec<String> {
    let mut roles = Vec::new();
    for word in &["buyer", "seller", "monitor", "worker", "tester", "validator"] {
        if text.contains(word) { roles.push(word.to_string()); }
    }
    roles
}

const SWARM_HELP: &str = "**Agent Swarm Commands**\n\n\
    `/swarm spawn <N> [role]` — spawn N agents\n\
    `/swarm status` — show all agents\n\
    `/swarm assign <goal>` — distribute task to idle agents\n\
    `/swarm results` — show aggregated results\n\
    `/swarm kill <id>` — terminate one agent\n\
    `/swarm kill-all` — terminate all agents\n\
    `/swarm scale <N>` — scale to exactly N agents\n\n\
    Or use natural language: \"deploy 5 agents to test my API\"";
