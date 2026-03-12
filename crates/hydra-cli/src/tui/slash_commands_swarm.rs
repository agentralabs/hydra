//! Swarm slash commands — /swarm, /swarm-spawn, /swarm-status, etc.
//!
//! Manages up to 100+ local/remote agents via SwarmManager (P9).

use super::app::{App, Message, MessageRole};
use hydra_native::swarm::AgentRole;

impl App {
    /// Route /swarm <subcommand> to the right handler.
    pub(crate) fn slash_cmd_swarm(&mut self, args: &str, timestamp: &str) {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];
        let sub_args = parts.get(1).copied().unwrap_or("");

        match sub {
            "spawn"    => self.slash_cmd_swarm_spawn(sub_args, timestamp),
            "status"   => self.slash_cmd_swarm_status(timestamp),
            "assign"   => self.slash_cmd_swarm_assign(sub_args, timestamp),
            "results"  => self.slash_cmd_swarm_results(timestamp),
            "kill"     => self.slash_cmd_swarm_kill(sub_args, timestamp),
            "kill-all" => self.slash_cmd_swarm_kill_all(timestamp),
            "scale"    => self.slash_cmd_swarm_scale(sub_args, timestamp),
            "" | "help" => self.swarm_help(timestamp),
            _ => {
                self.push_system(timestamp, format!(
                    "Unknown swarm command: {}. Try /swarm help", sub,
                ));
            }
        }
    }

    fn swarm_help(&mut self, timestamp: &str) {
        self.push_system(timestamp, "\
Swarm Commands:
  /swarm spawn <N>              Spawn N local worker agents
  /swarm status                 Show all agent statuses
  /swarm assign <goal>          Distribute goal across idle agents
  /swarm results                Show aggregated results
  /swarm kill <id-prefix>       Terminate a specific agent
  /swarm kill-all               Terminate all agents
  /swarm scale <N>              Scale to exactly N agents".into());
    }

    /// /swarm-spawn <count> [role] — spawn local agents.
    pub(crate) fn slash_cmd_swarm_spawn(&mut self, args: &str, timestamp: &str) {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let count: usize = parts.first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        if count == 0 || count > 100 {
            self.push_system(timestamp, "Spawn count must be 1-100.".into());
            return;
        }

        let role = match parts.get(1).copied() {
            Some("monitor") => AgentRole::Monitor,
            Some("generalist") => AgentRole::Generalist,
            Some(s) if s.starts_with("specialist:") => {
                AgentRole::Specialist(s.trim_start_matches("specialist:").to_string())
            }
            _ => AgentRole::Worker,
        };

        let skills: Vec<String> = parts.iter().skip(2).map(|s| s.to_string()).collect();

        self.push_system(timestamp, format!("Spawning {} {:?} agents...", count, role));

        let mgr = self.swarm_manager.clone_handle();
        tokio::spawn(async move {
            let results = mgr.spawn_local(count, role, skills).await;
            let ok = results.iter().filter(|r| r.is_ok()).count();
            let fail = results.iter().filter(|r| r.is_err()).count();
            eprintln!("[swarm] Spawned {} agents ({} failed). Total: {}",
                ok, fail, mgr.agent_count());
        });
    }

    /// /swarm-status — show all agents.
    pub(crate) fn slash_cmd_swarm_status(&mut self, timestamp: &str) {
        self.push_system(timestamp, self.swarm_manager.status_summary());
    }

    /// /swarm-assign <goal> — distribute goal across idle agents.
    pub(crate) fn slash_cmd_swarm_assign(&mut self, args: &str, timestamp: &str) {
        if args.trim().is_empty() {
            self.push_system(timestamp, "Usage: /swarm assign <goal>".into());
            return;
        }

        let assignments = self.swarm_manager.assign_task(args);
        let msg = if assignments.is_empty() {
            "No idle agents. Spawn agents first with /swarm spawn <N>".into()
        } else {
            let mut out = format!("Assigned {} tasks:\n", assignments.len());
            for a in &assignments {
                let short_id = if a.agent_id.len() >= 8 { &a.agent_id[..8] } else { &a.agent_id };
                out.push_str(&format!("  [{}] {}\n", short_id, a.task.description));
            }
            out
        };
        self.push_system(timestamp, msg);
    }

    /// /swarm-results — show aggregated results.
    pub(crate) fn slash_cmd_swarm_results(&mut self, timestamp: &str) {
        let report = self.swarm_manager.collect_results();
        let content = if report.total_agents == 0 {
            "No results yet. Assign tasks first with /swarm assign <goal>".into()
        } else {
            report.display()
        };
        self.push_system(timestamp, content);
    }

    /// /swarm-kill <id-prefix> — terminate one agent.
    pub(crate) fn slash_cmd_swarm_kill(&mut self, args: &str, timestamp: &str) {
        let prefix = args.trim();
        if prefix.is_empty() {
            self.push_system(timestamp, "Usage: /swarm kill <agent-id-prefix>".into());
            return;
        }

        match self.swarm_manager.find_agent(prefix) {
            Some(id) => {
                let short = if id.len() >= 8 { &id[..8] } else { &id };
                self.push_system(timestamp, format!("Terminating agent {}...", short));
                let mgr = self.swarm_manager.clone_handle();
                let id_clone = id.clone();
                tokio::spawn(async move {
                    let _ = mgr.kill_agent(&id_clone).await;
                    eprintln!("[swarm] Agent {} terminated", &id_clone[..8]);
                });
            }
            None => {
                self.push_system(timestamp, format!("No agent found matching '{}'", prefix));
            }
        }
    }

    /// /swarm-kill-all — terminate all agents.
    pub(crate) fn slash_cmd_swarm_kill_all(&mut self, timestamp: &str) {
        let count = self.swarm_manager.agent_count();
        if count == 0 {
            self.push_system(timestamp, "No agents to terminate.".into());
            return;
        }
        self.push_system(timestamp, format!("Terminating {} agents...", count));
        let mgr = self.swarm_manager.clone_handle();
        tokio::spawn(async move {
            mgr.kill_all().await;
            eprintln!("[swarm] All agents terminated");
        });
    }

    /// /swarm-scale <N> — scale to exactly N agents.
    pub(crate) fn slash_cmd_swarm_scale(&mut self, args: &str, timestamp: &str) {
        let target: usize = match args.trim().parse() {
            Ok(n) if n <= 100 => n,
            _ => {
                self.push_system(timestamp, "Usage: /swarm scale <0-100>".into());
                return;
            }
        };
        self.push_system(timestamp, format!("Scaling to {} agents...", target));
        let mgr = self.swarm_manager.clone_handle();
        tokio::spawn(async move {
            let result = mgr.scale_to(target).await;
            eprintln!("[swarm] {}", result);
        });
    }

    /// Push a system message (swarm helper).
    fn push_system(&mut self, timestamp: &str, content: String) {
        self.messages.push(Message {
            role: MessageRole::System,
            content,
            timestamp: timestamp.to_string(),
            phase: Some("swarm".into()),
        });
    }
}
