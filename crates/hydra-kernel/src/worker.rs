//! O6 Universal Worker — coordination layer for multi-interface task execution.
//! Classifies interfaces, computes blast radius, manages cross-app context,
//! gates autonomy via Judgment Gate, and provides workflow templates.

use crate::conductor::{Step, StepType, SHELL_TIMEOUT_MS};

// ── Types ──

/// Which interface a step targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interface { Browser, Desktop, Shell, Api }

/// Cross-app context that persists across steps within a single task.
/// Carries clipboard data, focused app, and step artifacts for cross-app flow.
#[derive(Debug, Clone, Default)]
pub struct AppContext {
    pub active_interfaces: Vec<Interface>,
    /// Last data extracted — available for cross-app paste (EC-6.2).
    pub clipboard: Option<String>,
    /// Current focused app for desktop agent context.
    pub focused_app: Option<String>,
    /// Screenshot hash for stale detection (EC-6.4).
    pub last_screenshot_hash: Option<u64>,
    /// Step outputs keyed by step_id for cross-step data flow.
    pub step_artifacts: std::collections::HashMap<usize, String>,
    /// Interface effectiveness tracking: (interface, success) per step.
    pub interface_outcomes: Vec<(Interface, bool)>,
}

impl AppContext {
    pub fn new() -> Self { Self::default() }

    /// Record that a step produced output. Stores in artifacts and auto-clips small outputs.
    pub fn record_step_output(&mut self, step_id: usize, output: &str, interface: Interface, success: bool) {
        self.step_artifacts.insert(step_id, output.to_string());
        if !self.active_interfaces.contains(&interface) {
            self.active_interfaces.push(interface);
        }
        if !output.is_empty() && output.len() < 4096 {
            self.clipboard = Some(output.to_string());
        }
        // Track interface effectiveness per domain
        self.interface_outcomes.push((interface, success));
    }

    /// Get interface success rate summary for genome feedback.
    pub fn interface_summary(&self) -> String {
        if self.interface_outcomes.is_empty() { return String::new(); }
        let total = self.interface_outcomes.len();
        let successes = self.interface_outcomes.iter().filter(|(_, s)| *s).count();
        let interfaces: Vec<String> = self.active_interfaces.iter().map(|i| format!("{i:?}")).collect();
        format!("interfaces={} success={}/{}", interfaces.join("+"), successes, total)
    }

    /// Check if UI state might be stale (EC-6.4).
    pub fn is_stale(&self, new_hash: u64) -> bool {
        self.last_screenshot_hash.map(|h| h != new_hash).unwrap_or(false)
    }
}

// ── Interface Classification ──

/// Classify which interface a conductor Step needs.
/// Works on typed StepType enum — no string ambiguity (EC-6.1).
pub fn classify_interface(step_type: &StepType) -> Interface {
    match step_type {
        StepType::BrowserNavigate { .. } | StepType::BrowserInteract { .. } => Interface::Browser,
        StepType::DesktopAction { .. } => Interface::Desktop,
        StepType::ApiCall { .. } => Interface::Api,
        _ => Interface::Shell,
    }
}

// ── Blast Radius Computation ──

/// Compute the blast radius for a conductor step.
/// Replaces the hardcoded `BlastRadius::Contained` in intelligence middleware.
pub fn blast_radius_for_step(step: &Step) -> hydra_wisdom::BlastRadius {
    use hydra_wisdom::BlastRadius;
    match &step.step_type {
        StepType::FileRead { .. } | StepType::Wait { .. } | StepType::Verify { .. } => BlastRadius::Contained,
        StepType::FileWrite { .. } | StepType::CodeGen { .. } => BlastRadius::Contained,
        StepType::BrowserNavigate { .. } => BlastRadius::Contained,
        StepType::Shell { command, .. } => classify_shell_blast(command),
        StepType::BrowserInteract { goal } => classify_browser_blast(goal),
        StepType::DesktopAction { goal } => classify_browser_blast(goal),
        StepType::ApiCall { method, .. } => match method.to_uppercase().as_str() {
            "GET" | "HEAD" | "OPTIONS" => BlastRadius::Contained,
            "DELETE" => BlastRadius::Irreversible,
            _ => BlastRadius::Visible,
        },
        StepType::Remote { .. } => BlastRadius::Visible, // Remote commands are visible actions
    }
}

fn classify_shell_blast(cmd: &str) -> hydra_wisdom::BlastRadius {
    use hydra_wisdom::BlastRadius;
    let lower = cmd.to_lowercase();
    if lower.contains("rm ") || lower.contains("drop ") || lower.contains("truncate ")
        || lower.contains("--force") || lower.contains("-rf") {
        BlastRadius::Irreversible
    } else if lower.contains("deploy") || lower.contains("push") || lower.contains("publish") {
        BlastRadius::Catastrophic
    } else if lower.contains("curl") || lower.contains("docker") || lower.contains("kubectl") {
        BlastRadius::Visible
    } else {
        BlastRadius::Contained
    }
}

fn classify_browser_blast(goal: &str) -> hydra_wisdom::BlastRadius {
    use hydra_wisdom::BlastRadius;
    let lower = goal.to_lowercase();
    if lower.contains("delete") || lower.contains("send") || lower.contains("pay")
        || lower.contains("purchase") || lower.contains("submit") || lower.contains("transfer") {
        BlastRadius::Irreversible
    } else if lower.contains("post") || lower.contains("reply") || lower.contains("comment") {
        BlastRadius::Visible
    } else {
        BlastRadius::Contained
    }
}

// ── Autonomy Check ──

/// Check if a step can execute autonomously or needs human approval.
/// Uses O29 Autonomy Gradient (continuous 0-1) and falls back to Judgment Gate.
pub fn autonomy_check(step: &Step, genome: &hydra_genome::GenomeStore) -> hydra_wisdom::JudgmentDecision {
    let blast = blast_radius_for_step(step);
    // O29: Compute continuous autonomy score
    let autonomy = hydra_wisdom::autonomy::autonomy_from_genome(&step.description, genome, &blast);
    eprintln!("hydra-autonomy: '{}' → {:.2} ({})",
        &step.description[..step.description.len().min(40)], autonomy.value, autonomy.decision.label());
    // Map AutonomyDecision to JudgmentDecision for backward compat
    match autonomy.decision {
        hydra_wisdom::autonomy::AutonomyDecision::ActSilently => hydra_wisdom::JudgmentDecision::Act {
            reason: format!("Autonomy {:.2} — acting silently", autonomy.value),
            confidence: autonomy.confidence,
        },
        hydra_wisdom::autonomy::AutonomyDecision::ActAndNotify { msg } => hydra_wisdom::JudgmentDecision::Act {
            reason: msg, confidence: autonomy.confidence,
        },
        hydra_wisdom::autonomy::AutonomyDecision::AskFirst { question } => hydra_wisdom::JudgmentDecision::Ask {
            reason: question.clone(), confidence: autonomy.confidence,
            what_could_go_wrong: format!("rev={:.2} blast={:.2}", autonomy.reversibility, autonomy.blast_radius),
        },
        hydra_wisdom::autonomy::AutonomyDecision::Refuse { reason } => hydra_wisdom::JudgmentDecision::Refuse {
            reason, confidence: autonomy.confidence,
        },
    }
}

// ── Workflow Templates ──

/// Returns pre-built steps for common multi-app patterns.
/// Thin fallback — prefer TOML operations in skills/.
pub fn expand_workflow(goal: &str) -> Option<Vec<Step>> {
    let lower = goal.to_lowercase();
    if (lower.contains("email") || lower.contains("mail")) && lower.contains("about") {
        return Some(vec![
            make_step(0, StepType::BrowserNavigate { url: "https://mail.google.com".into() },
                "Open email", vec![]),
            make_step(1, StepType::BrowserInteract { goal: goal.into() },
                "Compose and send", vec![0]),
        ]);
    }
    if lower.contains("slack") && (lower.contains("message") || lower.contains("send")) {
        return Some(vec![
            make_step(0, StepType::BrowserNavigate { url: "https://app.slack.com".into() },
                "Open Slack", vec![]),
            make_step(1, StepType::BrowserInteract { goal: goal.into() },
                "Send message", vec![0]),
        ]);
    }
    None
}

fn make_step(id: usize, st: StepType, desc: &str, deps: Vec<usize>) -> Step {
    Step { id, step_type: st, description: desc.into(), depends_on: deps, timeout_ms: SHELL_TIMEOUT_MS }
}

// ── Interface Step Execution ──

/// Execute a browser/desktop/api step through the appropriate agent.
/// For browser/desktop: returns AGENT_DISPATCH signal for TUI to pick up.
/// For API: executes via shell curl (no reqwest dependency needed).
pub fn execute_interface_step(
    step: &Step, _ctx: &crate::conductor::TaskContext, app_ctx: &mut AppContext,
) -> (bool, String, Vec<String>) {
    let interface = classify_interface(&step.step_type);
    match &step.step_type {
        StepType::BrowserNavigate { url } => {
            app_ctx.focused_app = Some("browser".into());
            eprintln!("hydra-worker: browser navigate (visible) → {url}");
            // VISIBLE: open in user's default browser (not headless CDP)
            let result = std::process::Command::new("sh")
                .arg("-c").arg(format!("open '{}'", url.replace('\'', "'\\''")))
                .status();
            match result {
                Ok(s) if s.success() => (true, format!("[Browser: opened {url}]"), vec![]),
                Ok(s) => (false, format!("Browser open failed (exit {})", s.code().unwrap_or(-1)), vec![]),
                Err(e) => (false, format!("Browser open error: {e}"), vec![]),
            }
        }
        StepType::BrowserInteract { goal } => {
            app_ctx.focused_app = Some("browser".into());
            eprintln!("hydra-worker: browser interact → {goal}");
            // Browser interact uses the full computer use agent with vision
            // For now, launch browser + navigate to goal as URL or search
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let goal = goal.clone();
                let result = tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        let mut engine = hydra_browser::BrowserEngine::new();
                        engine.launch().await.map_err(|e| format!("Browser launch: {e}"))?;
                        // If goal looks like a URL, navigate; otherwise search
                        let url = if goal.starts_with("http") { goal.clone() }
                            else { format!("https://html.duckduckgo.com/html/?q={}", goal.replace(' ', "+")) };
                        engine.navigate(&url).await.map_err(|e| format!("Navigate: {e}"))?;
                        Ok::<String, String>(format!("Browser: {goal}"))
                    })
                });
                match result {
                    Ok(msg) => (true, msg, vec![]),
                    Err(e) => (false, e, vec![]),
                }
            } else {
                (true, format!("[Browser: {goal}]"), vec![])
            }
        }
        StepType::DesktopAction { goal } => {
            app_ctx.focused_app = Some("desktop".into());
            eprintln!("hydra-worker: desktop action → {goal}");
            // Fast-path: "open <app>" → visible launch via shell (no vision loop needed)
            let lower_goal = goal.to_lowercase();
            if lower_goal.starts_with("open ") || lower_goal.starts_with("launch ") {
                let target = if lower_goal.starts_with("open ") { &goal[5..] } else { &goal[7..] };
                let cmd = if target.contains("http") || target.contains('.') && target.contains('/') {
                    format!("open '{}'", target.trim().replace('\'', "'\\''"))
                } else {
                    format!("open -a '{}'", target.trim().replace('\'', "'\\''"))
                };
                if let Ok(s) = std::process::Command::new("sh").arg("-c").arg(&cmd).status() {
                    if s.success() {
                        return (true, format!("[Desktop: opened {target}]"), vec![]);
                    }
                }
                // Fall through to vision loop if shell open failed
            }
            // Wire to real DesktopAgent via tokio block_in_place
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let goal = goal.clone();
                let result = tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        let vision = match crate::vision_bridge::LlmVisionProvider::new() {
                            Some(v) => v,
                            None => return Err(hydra_desktop::DesktopError::VisionError(
                                "No API key for vision — set ANTHROPIC_API_KEY".into())),
                        };
                        // Channel for step updates — drained below for logging
                        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
                        let agent = hydra_desktop::agent::DesktopAgent::new();
                        let result = agent.execute_task_v2(&goal, &vision, tx).await;
                        // Drain step updates for logging (TUI has its own channel)
                        while let Ok(update) = rx.try_recv() {
                            eprintln!("hydra-desktop: step {} — {}", update.step, update.action);
                        }
                        result
                    })
                });
                match result {
                    Ok(r) => (r.completed, r.final_observation, vec![]),
                    Err(e) => (false, format!("Desktop error: {e}"), vec![]),
                }
            } else {
                (true, format!("[Desktop: {goal}]"), vec![])
            }
        }
        StepType::ApiCall { method, url, body } => {
            let escaped_url = url.replace('\'', "'\\''");
            let body_arg = body.as_ref().map(|b| format!(" -d '{}'", b.replace('\'', "'\\''"))).unwrap_or_default();
            let cmd = format!("curl -s --connect-timeout 15 -X {method} '{escaped_url}'{body_arg} -w '\\n%{{http_code}}'");
            let mut command = std::process::Command::new("sh");
            command.arg("-c").arg(&cmd);
            #[cfg(unix)]
            unsafe {
                use std::os::unix::process::CommandExt;
                command.pre_exec(|| { libc::setpgid(0, 0); Ok(()) });
            }
            match command.output() {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout).to_string();
                    let success = out.status.success();
                    app_ctx.record_step_output(step.id, &text, interface, success);
                    (success, format!("API {method} {url}: {}", &text[..text.len().min(500)]), vec![])
                }
                Err(e) => (false, format!("API call failed: {e}"), vec![]),
            }
        }
        _ => (false, "Not an interface step".into(), vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_interface_routes_correctly() {
        assert_eq!(classify_interface(&StepType::BrowserNavigate { url: "x".into() }), Interface::Browser);
        assert_eq!(classify_interface(&StepType::BrowserInteract { goal: "x".into() }), Interface::Browser);
        assert_eq!(classify_interface(&StepType::DesktopAction { goal: "x".into() }), Interface::Desktop);
        assert_eq!(classify_interface(&StepType::Shell { command: "ls".into(), long_running: false }), Interface::Shell);
        assert_eq!(classify_interface(&StepType::ApiCall { method: "GET".into(), url: "x".into(), body: None }), Interface::Api);
    }

    #[test]
    fn blast_radius_dangerous_commands() {
        let step = |st| Step { id: 0, step_type: st, description: "test".into(), depends_on: vec![], timeout_ms: 5000 };
        assert_eq!(blast_radius_for_step(&step(StepType::FileRead { path: "x".into() })),
            hydra_wisdom::BlastRadius::Contained);
        assert_eq!(blast_radius_for_step(&step(StepType::Shell { command: "rm -rf /".into(), long_running: false })),
            hydra_wisdom::BlastRadius::Irreversible);
        assert_eq!(blast_radius_for_step(&step(StepType::Shell { command: "git push".into(), long_running: false })),
            hydra_wisdom::BlastRadius::Catastrophic);
        assert_eq!(blast_radius_for_step(&step(StepType::BrowserInteract { goal: "delete account".into() })),
            hydra_wisdom::BlastRadius::Irreversible);
        assert_eq!(blast_radius_for_step(&step(StepType::ApiCall { method: "DELETE".into(), url: "x".into(), body: None })),
            hydra_wisdom::BlastRadius::Irreversible);
        assert_eq!(blast_radius_for_step(&step(StepType::ApiCall { method: "GET".into(), url: "x".into(), body: None })),
            hydra_wisdom::BlastRadius::Contained);
    }

    #[test]
    fn workflow_template_email() {
        let steps = expand_workflow("email john about the quarterly report");
        assert!(steps.is_some());
        let steps = steps.unwrap();
        assert_eq!(steps.len(), 2);
        assert!(matches!(steps[0].step_type, StepType::BrowserNavigate { .. }));
        assert!(matches!(steps[1].step_type, StepType::BrowserInteract { .. }));
        assert_eq!(steps[1].depends_on, vec![0]);
    }

    #[test]
    fn workflow_no_match_returns_none() {
        assert!(expand_workflow("compile the rust project").is_none());
    }

    #[test]
    fn app_context_cross_step_data() {
        let mut ctx = AppContext::new();
        ctx.record_step_output(0, "spreadsheet data", Interface::Browser, true);
        assert_eq!(ctx.clipboard.as_deref(), Some("spreadsheet data"));
        assert_eq!(ctx.active_interfaces, vec![Interface::Browser]);
        ctx.record_step_output(1, "processed", Interface::Shell, true);
        assert_eq!(ctx.active_interfaces, vec![Interface::Browser, Interface::Shell]);
    }

    #[test]
    fn stale_detection() {
        let mut ctx = AppContext::new();
        ctx.last_screenshot_hash = Some(12345);
        assert!(ctx.is_stale(99999));
        assert!(!ctx.is_stale(12345));
    }
}
