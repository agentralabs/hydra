//! Cognitive sister dispatch — 17 sisters, 5 phases.
//!
//! This module contains the `Sisters` struct that holds all 17 sister connections
//! and provides the PERCEIVE, THINK (prompt building), DECIDE (risk), ACT, and LEARN
//! phase dispatch methods.

use std::sync::Arc;

use super::connection::SisterConnection;

#[path = "cognitive_dispatch.rs"]
mod dispatch;

/// Holds all 17 connected sister processes — the full constellation
pub struct Sisters {
    // Foundation Sisters (7)
    pub memory: Option<SisterConnection>,
    pub identity: Option<SisterConnection>,
    pub codebase: Option<SisterConnection>,
    pub vision: Option<SisterConnection>,
    pub comm: Option<SisterConnection>,
    pub contract: Option<SisterConnection>,
    pub time: Option<SisterConnection>,
    // Cognitive Sisters (3)
    pub planning: Option<SisterConnection>,
    pub cognition: Option<SisterConnection>,
    pub reality: Option<SisterConnection>,
    // Astral Sisters (4)
    pub forge: Option<SisterConnection>,
    pub aegis: Option<SisterConnection>,
    pub veritas: Option<SisterConnection>,
    pub evolve: Option<SisterConnection>,
    // Utility Sisters (3)
    pub data: Option<SisterConnection>,
    pub connect: Option<SisterConnection>,
    pub workflow: Option<SisterConnection>,
}

impl Sisters {
    /// Spawn ALL 17 sisters in PARALLEL. Non-blocking: sisters that fail are None.
    pub async fn spawn_all() -> Self {
        let home = hydra_native_state::utils::home_dir();
        // Configurable via HYDRA_SISTER_BIN_DIR env var (default: ~/.local/bin)
        let bin_dir = std::env::var("HYDRA_SISTER_BIN_DIR")
            .unwrap_or_else(|_| format!("{}/.local/bin", home));
        // Binary suffix: .exe on Windows, empty on Unix
        let ext = if cfg!(windows) { ".exe" } else { "" };

        // Pre-compute all paths (cross-platform binary names)
        let bin = |name: &str| format!("{}/{}{}", bin_dir, name, ext);
        let memory_bin = bin("agentic-memory-mcp");
        let identity_bin = bin("agentic-identity-mcp");
        let codebase_bin = bin("agentic-codebase-mcp");
        let vision_bin = bin("agentic-vision-mcp");
        let comm_bin = bin("agentic-comm-mcp");
        let contract_bin = bin("agentic-contract-mcp");
        let time_bin = bin("agentic-time-mcp");
        let planning_bin = bin("agentic-planning-mcp");
        let cognition_bin = bin("agentic-cognition-mcp");
        let reality_bin = bin("agentic-reality-mcp");
        let forge_bin = bin("agentic-forge-mcp");
        let aegis_bin = bin("agentic-aegis-mcp");
        let veritas_bin = bin("agentic-veritas-mcp");
        let evolve_bin = bin("agentic-evolve-mcp");
        let data_bin = bin("agentic-data-mcp");
        let connect_bin = bin("agentic-connect-mcp");
        let workflow_bin = bin("agentic-workflow-mcp");

        // Auto-create memory directory if missing
        let memory_dir = format!("{}/.hydra/memory", home);
        let _ = std::fs::create_dir_all(&memory_dir);
        let hydra_memory = format!("{}/hydra.amem", memory_dir);
        let memory_args: Vec<&str> = vec!["serve", "--memory", &hydra_memory];

        // Spawn ALL 17 sisters in parallel for fastest startup
        let (memory, identity, codebase, vision, comm, contract, time,
             planning, cognition, reality, forge, aegis, veritas, evolve,
             data, connect, workflow) = tokio::join!(
            // Foundation (use "serve")
            Self::try_spawn("memory", &memory_bin, &memory_args),
            Self::try_spawn("identity", &identity_bin, &["serve"]),
            Self::try_spawn("codebase", &codebase_bin, &["serve"]),
            Self::try_spawn("vision", &vision_bin, &["serve"]),
            Self::try_spawn("comm", &comm_bin, &["serve"]),
            Self::try_spawn("contract", &contract_bin, &[]),
            Self::try_spawn("time", &time_bin, &["serve"]),
            // Cognitive
            Self::try_spawn("planning", &planning_bin, &["serve"]),
            Self::try_spawn("cognition", &cognition_bin, &[]),
            Self::try_spawn("reality", &reality_bin, &[]),
            // Astral (no args, stdio mode)
            Self::try_spawn("forge", &forge_bin, &[]),
            Self::try_spawn("aegis", &aegis_bin, &[]),
            Self::try_spawn("veritas", &veritas_bin, &[]),
            Self::try_spawn("evolve", &evolve_bin, &[]),
            // Utility
            Self::try_spawn("data", &data_bin, &[]),
            Self::try_spawn("connect", &connect_bin, &[]),
            Self::try_spawn("workflow", &workflow_bin, &[]),
        );

        let s = Self {
            memory, identity, codebase, vision, comm, contract, time,
            planning, cognition, reality,
            forge, aegis, veritas, evolve,
            data, connect, workflow,
        };
        let all = s.all_sisters();
        let total = all.len();
        let connected = all.iter().filter(|(_, opt)| opt.is_some()).count();
        eprintln!("[hydra] ═══ {}/{} sisters connected ═══", connected, total);
        // Verify critical tools are present — scream if missing
        let missing = s.verify_critical_tools();
        for (sister, tools) in &missing {
            eprintln!("[hydra] ⚠ {} sister MISSING critical tools: {}", sister, tools.join(", "));
        }
        s
    }

    async fn try_spawn(name: &str, cmd: &str, args: &[&str]) -> Option<SisterConnection> {
        match SisterConnection::spawn(name, cmd, args).await {
            Ok(conn) => {
                // Log first 5 tool names for debugging capability mismatches
                let sample: Vec<&str> = conn.tools.iter().take(5).map(|s| s.as_str()).collect();
                eprintln!("[hydra] {} sister connected ({} tools): {}{}",
                    conn.name, conn.tools.len(), sample.join(", "),
                    if conn.tools.len() > 5 { ", ..." } else { "" });
                Some(conn)
            }
            Err(e) => {
                eprintln!("[hydra] {} sister unavailable: {}", name, e);
                None
            }
        }
    }

    /// Get specific tools from a sister by name. Returns matching tool names.
    /// Used by the tool router to send only relevant tools to the LLM.
    pub fn tools_for_sister(&self, sister: &str, names: &[&str]) -> Vec<String> {
        let conn = match sister {
            "memory" => self.memory.as_ref(),
            "identity" => self.identity.as_ref(),
            "codebase" => self.codebase.as_ref(),
            "vision" => self.vision.as_ref(),
            "comm" => self.comm.as_ref(),
            "contract" => self.contract.as_ref(),
            "time" => self.time.as_ref(),
            "planning" => self.planning.as_ref(),
            "cognition" => self.cognition.as_ref(),
            "reality" => self.reality.as_ref(),
            "forge" => self.forge.as_ref(),
            "aegis" => self.aegis.as_ref(),
            "veritas" => self.veritas.as_ref(),
            "evolve" => self.evolve.as_ref(),
            "data" => self.data.as_ref(),
            "connect" => self.connect.as_ref(),
            "workflow" => self.workflow.as_ref(),
            _ => None,
        };
        match conn {
            Some(c) => c.tools.iter()
                .filter(|t| names.iter().any(|n| t.contains(n)))
                .cloned()
                .collect(),
            None => vec![],
        }
    }

    /// Discover MCP tools from all connected sisters and return tool names per server.
    /// Returns a list of (server_name, tool_name) tuples.
    pub fn discover_mcp_tools(&self) -> Vec<(String, String)> {
        let mut discovered = Vec::new();
        for (name, opt) in self.all_sisters() {
            if let Some(conn) = opt {
                for tool_name in &conn.tools {
                    discovered.push((name.to_string(), tool_name.clone()));
                }
            }
        }
        discovered
    }

    // perceive_beliefs, _save_to_memory, log_conversation — extracted to cognitive_dispatch.rs
    // perceive, perceive_simple — extracted to sisters/perceive.rs
    // learn — extracted to sisters/learn.rs
    // Delegation methods — extracted to sisters/delegation.rs

    /// Get list of which sisters are actually connected (for accurate reporting)
    pub fn connected_sisters_list(&self) -> Vec<String> {
        self.all_sisters()
            .iter()
            .filter_map(|(name, opt)| if opt.is_some() { Some(name.to_string()) } else { None })
            .collect()
    }

    // degradation_report — extracted to cognitive_dispatch.rs
    // detects_code, detects_visual, classify_complexity, assess_risk — extracted to cognitive_dispatch.rs
    // status_summary — extracted to cognitive_dispatch.rs

    /// Create an empty Sisters instance with no connections (for tests)
    pub fn empty() -> Self {
        Self {
            memory: None, identity: None, codebase: None, vision: None,
            comm: None, contract: None, time: None,
            planning: None, cognition: None, reality: None,
            forge: None, aegis: None, veritas: None, evolve: None,
            data: None, connect: None, workflow: None,
        }
    }

    /// Count connected sisters
    pub fn connected_count(&self) -> usize {
        self.all_sisters().iter().filter(|(_, s)| s.is_some()).count()
    }

    /// All 17 sisters as name/option pairs
    pub fn all_sisters(&self) -> Vec<(&str, &Option<SisterConnection>)> {
        vec![
            ("Memory", &self.memory), ("Identity", &self.identity),
            ("Codebase", &self.codebase), ("Vision", &self.vision),
            ("Comm", &self.comm), ("Contract", &self.contract),
            ("Time", &self.time),
            ("Planning", &self.planning), ("Cognition", &self.cognition),
            ("Reality", &self.reality),
            ("Forge", &self.forge), ("Aegis", &self.aegis),
            ("Veritas", &self.veritas), ("Evolve", &self.evolve),
            ("Data", &self.data), ("Connect", &self.connect),
            ("Workflow", &self.workflow),
        ]
    }

    // capabilities_prompt — extracted to cognitive_dispatch.rs

    /// Verify each connected sister has a minimum tool count.
    /// Uses the REAL tool list from MCP handshake — never guesses tool names.
    /// Returns (sister_name, issue_description) for failures.
    pub fn verify_critical_tools(&self) -> Vec<(String, Vec<String>)> {
        // Minimum expected tool counts per sister (from real MCP handshakes)
        let minimums: &[(&str, usize)] = &[
            ("Memory", 5), ("Identity", 3), ("Codebase", 3), ("Comm", 3),
            ("Contract", 2), ("Planning", 3), ("Cognition", 2), ("Forge", 2),
            ("Aegis", 2), ("Veritas", 2), ("Evolve", 2), ("Vision", 1),
            ("Reality", 1), ("Time", 2),
            ("Data", 3), ("Connect", 3), ("Workflow", 3),
        ];
        let mut warnings = Vec::new();
        for (name, min) in minimums {
            if let Some(conn) = self.all_sisters().iter()
                .find(|(n, opt)| *n == *name && opt.is_some())
                .and_then(|(_, opt)| opt.as_ref())
            {
                if conn.tools.len() < *min {
                    warnings.push((name.to_string(),
                        vec![format!("expected {}+ tools, got {}", min, conn.tools.len())]));
                }
            }
        }
        warnings
    }

    /// Graceful session shutdown — called on app close.
    /// Runs Ghost Writer summary, session end for Memory/Comm/Time, and agent deregistration.
    pub async fn shutdown_session(&self, user_name: &str, history: &[(String, String)]) {
        eprintln!("[hydra:shutdown] Starting graceful session shutdown...");
        // 1. Ghost Writer summary (captures session narrative before shutdown)
        let summary = if let Some(ghost) = self.memory_ghost_write(history).await {
            eprintln!("[hydra:shutdown] Ghost Writer summary: {} chars", ghost.len());
            ghost
        } else {
            let msg_count = history.len();
            format!("Session with {} messages completed", msg_count)
        };
        // 2. End all session trackers in parallel
        let mem_end = self.memory_session_end(&summary);
        let comm_end = self.comm_session_end(&summary);
        let time_end = self.time_session_end(&summary);
        let comm_dereg = self.comm_deregister_agent(user_name);
        let aegis_end = self.aegis_session_end(&summary);
        let contract_end = self.contract_session_end(&summary);
        tokio::join!(mem_end, comm_end, time_end, comm_dereg, aegis_end, contract_end);
        eprintln!("[hydra:shutdown] Session shutdown complete.");
    }
}

/// Shared handle to sisters, safe to clone across async tasks
pub type SistersHandle = Arc<Sisters>;

/// Spawn sisters and return a shared handle
pub async fn init_sisters() -> SistersHandle {
    Arc::new(Sisters::spawn_all().await)
}

#[cfg(test)]
#[path = "cognitive_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cognitive_tests_extra.rs"]
mod tests_extra;
