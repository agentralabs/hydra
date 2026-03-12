//! Capability Registry — single source of truth for all Hydra capabilities.
//!
//! Every capability is registered here with trigger patterns, slash commands,
//! and handler routing. This ensures natural language, slash commands, and LLM
//! awareness all stay in sync. Register once, available everywhere.

/// A registered Hydra capability.
#[derive(Debug, Clone)]
pub struct Capability {
    pub name: &'static str,
    pub slash_command: &'static str,
    pub description: &'static str,
    /// Phrases that strongly indicate this capability (score += 2.0 each).
    pub trigger_patterns: &'static [&'static str],
    /// Keywords that weakly indicate this capability (score += 1.0 each).
    pub trigger_keywords: &'static [&'static str],
    pub handler: CapabilityHandler,
    pub requires_url: bool,
}

/// Which handler to dispatch to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityHandler {
    ProjectExec,
    Swarm,
    SisterImprove,
    RemoteExec,
    ThreatCheck,
    EnvironmentProbe,
    TaskList,
}

/// Result of matching user input against the registry.
pub struct CapabilityMatch<'a> {
    pub capability: &'a Capability,
    pub score: f32,
    pub extracted_url: Option<String>,
}

/// Central registry of all capabilities.
pub struct CapabilityRegistry {
    capabilities: Vec<Capability>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: vec![
                Capability {
                    name: "project_exec",
                    slash_command: "/test-repo",
                    description: "Test a GitHub repository autonomously — clone, setup, run tests",
                    trigger_patterns: &[
                        "test repo", "test github", "run tests on",
                        "clone and test", "test this repo", "test that repo",
                        "evaluate this repo", "check this repo",
                    ],
                    trigger_keywords: &["github.com", "gitlab.com", "test-repo"],
                    handler: CapabilityHandler::ProjectExec,
                    requires_url: true,
                },
                Capability {
                    name: "swarm",
                    slash_command: "/swarm",
                    description: "Deploy and manage multiple agents for parallel work",
                    trigger_patterns: &[
                        "deploy agents", "spawn agents", "multiple agents",
                        "test with agents", "agents to test", "create agents",
                        "launch agents", "start agents", "deploy workers",
                        "buyers and sellers", "buy and sell",
                        "parallel agents", "agent army", "agent swarm",
                        "deploy a swarm", "swarm of agents",
                        "multi-agent", "multiagent",
                    ],
                    trigger_keywords: &["agents", "swarm", "spawn"],
                    handler: CapabilityHandler::Swarm,
                    requires_url: false,
                },
                Capability {
                    name: "sister_improve",
                    slash_command: "/improve-sister",
                    description: "Analyze and improve a sister system's code",
                    trigger_patterns: &[
                        "improve sister", "fix sister", "upgrade sister",
                        "patch sister", "make memory better", "improve the",
                    ],
                    trigger_keywords: &[],
                    handler: CapabilityHandler::SisterImprove,
                    requires_url: false,
                },
                Capability {
                    name: "remote_exec",
                    slash_command: "/ssh-exec",
                    description: "Execute commands on remote machines via SSH",
                    trigger_patterns: &[
                        "run on server", "execute on remote", "ssh into",
                        "deploy to server", "check server", "on the server",
                        "run remotely", "execute remotely",
                    ],
                    trigger_keywords: &["ssh"],
                    handler: CapabilityHandler::RemoteExec,
                    requires_url: false,
                },
                Capability {
                    name: "threat_check",
                    slash_command: "/threat",
                    description: "Check threat level and security status",
                    trigger_patterns: &[
                        "threat level", "security status", "any threats",
                        "is it safe", "check threats", "threat assessment",
                    ],
                    trigger_keywords: &["threat"],
                    handler: CapabilityHandler::ThreatCheck,
                    requires_url: false,
                },
                Capability {
                    name: "task_list",
                    slash_command: "/tasks",
                    description: "Show active and interrupted tasks",
                    trigger_patterns: &[
                        "show tasks", "list tasks", "active tasks",
                        "what tasks", "my tasks", "pending tasks",
                    ],
                    trigger_keywords: &[],
                    handler: CapabilityHandler::TaskList,
                    requires_url: false,
                },
                Capability {
                    name: "env_probe",
                    slash_command: "/env",
                    description: "Show environment and system capabilities",
                    trigger_patterns: &[
                        "what environment", "system info", "what's installed",
                        "show environment", "check environment", "what tools",
                    ],
                    trigger_keywords: &[],
                    handler: CapabilityHandler::EnvironmentProbe,
                    requires_url: false,
                },
            ],
        }
    }

    /// Match user input against ALL registered capabilities.
    /// Returns the best match above the threshold, or None.
    /// Also matches explicit slash commands (e.g., `/swarm spawn 3`).
    pub fn match_intent(&self, input: &str) -> Option<CapabilityMatch<'_>> {
        let lower = input.to_lowercase();
        let mut best: Option<CapabilityMatch<'_>> = None;

        for cap in &self.capabilities {
            let mut score: f32 = 0.0;

            // Direct slash command match — highest priority
            if lower.starts_with(cap.slash_command) {
                score += 10.0;
            }

            for pattern in cap.trigger_patterns {
                if lower.contains(pattern) {
                    score += 2.0;
                }
            }
            for keyword in cap.trigger_keywords {
                if lower.contains(keyword) {
                    score += 1.0;
                }
            }

            if score < 2.0 {
                continue;
            }

            let extracted_url = if cap.requires_url {
                extract_url(input)
            } else {
                None
            };

            // If the cap requires a URL but none found, reduce confidence
            if cap.requires_url && extracted_url.is_none() {
                score *= 0.5;
                if score < 2.0 {
                    continue;
                }
            }

            if best.as_ref().map_or(true, |b| score > b.score) {
                best = Some(CapabilityMatch {
                    capability: cap,
                    score,
                    extracted_url,
                });
            }
        }

        best
    }

    /// Generate a capability summary for the LLM system prompt.
    pub fn to_system_prompt_section(&self) -> String {
        let mut s = String::from(
            "YOUR CAPABILITIES (use these — don't describe them, DO them):\n",
        );
        for cap in &self.capabilities {
            s.push_str(&format!(
                "- {} ({}): {}\n",
                cap.name, cap.slash_command, cap.description
            ));
        }
        s.push_str(
            "\nWhen the user asks you to DO something you have a capability for, \
             route to that capability. Don't write an essay about what you COULD do.\n",
        );
        s
    }

    /// List all capabilities (for /help, diagnostics).
    pub fn list(&self) -> &[Capability] {
        &self.capabilities
    }
}

/// Extract a URL from free-form text.
fn extract_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let w = word.trim_matches(|c| c == '<' || c == '>' || c == '"' || c == '\'');
        if w.starts_with("https://") || w.starts_with("http://") || w.starts_with("git@") {
            return Some(w.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_natural_language() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("deploy 10 agents to test buying and selling");
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.capability.handler, CapabilityHandler::Swarm);
        assert!(m.score >= 2.0);
    }

    #[test]
    fn test_project_exec_with_url() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("test this repo https://github.com/user/project");
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.capability.handler, CapabilityHandler::ProjectExec);
        assert!(m.extracted_url.is_some());
    }

    #[test]
    fn test_project_exec_without_url_no_match() {
        let reg = CapabilityRegistry::new();
        // "test repo" pattern matches but no URL → score halved below threshold
        let m = reg.match_intent("test repo");
        // Score: 2.0 * 0.5 = 1.0 < 2.0 → None
        assert!(m.is_none());
    }

    #[test]
    fn test_threat_natural_language() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("what's the threat level right now?");
        assert!(m.is_some());
        assert_eq!(m.unwrap().capability.handler, CapabilityHandler::ThreatCheck);
    }

    #[test]
    fn test_no_match_general_conversation() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("hello, how are you today?");
        assert!(m.is_none());
    }

    #[test]
    fn test_swarm_buy_sell_pattern() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("deploy multiple agents to test by buying and selling");
        assert!(m.is_some());
        assert_eq!(m.unwrap().capability.handler, CapabilityHandler::Swarm);
    }

    #[test]
    fn test_system_prompt_section() {
        let reg = CapabilityRegistry::new();
        let section = reg.to_system_prompt_section();
        assert!(section.contains("swarm"));
        assert!(section.contains("project_exec"));
        assert!(section.contains("CAPABILITIES"));
    }

    #[test]
    fn test_remote_exec() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("ssh into the production server and check logs");
        assert!(m.is_some());
        assert_eq!(m.unwrap().capability.handler, CapabilityHandler::RemoteExec);
    }

    // ── Slash command matching (Desktop parity) ──

    #[test]
    fn test_slash_swarm_spawn() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("/swarm spawn 5");
        assert!(m.is_some());
        assert_eq!(m.unwrap().capability.handler, CapabilityHandler::Swarm);
    }

    #[test]
    fn test_slash_swarm_status() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("/swarm status");
        assert!(m.is_some());
        assert_eq!(m.unwrap().capability.handler, CapabilityHandler::Swarm);
    }

    #[test]
    fn test_slash_swarm_kill_all() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("/swarm kill-all");
        assert!(m.is_some());
        assert_eq!(m.unwrap().capability.handler, CapabilityHandler::Swarm);
    }

    #[test]
    fn test_slash_threat() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("/threat");
        assert!(m.is_some());
        assert_eq!(m.unwrap().capability.handler, CapabilityHandler::ThreatCheck);
    }

    #[test]
    fn test_slash_env() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("/env");
        assert!(m.is_some());
        assert_eq!(m.unwrap().capability.handler, CapabilityHandler::EnvironmentProbe);
    }

    #[test]
    fn test_slash_tasks() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("/tasks");
        assert!(m.is_some());
        assert_eq!(m.unwrap().capability.handler, CapabilityHandler::TaskList);
    }

    #[test]
    fn test_slash_test_repo_with_url() {
        let reg = CapabilityRegistry::new();
        let m = reg.match_intent("/test-repo https://github.com/user/repo");
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.capability.handler, CapabilityHandler::ProjectExec);
        assert!(m.extracted_url.is_some());
    }
}
