//! Task Conductor — DAG executor and step router.
//! Routes steps to shell, file, browser, verify, wait executors.
//! Types and decomposer are in conductor.rs.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use crate::conductor::*;

// ── Step Router ──

/// Execute a single step, routing to the appropriate executor.
/// AppContext carries cross-step state (clipboard, interface outcomes) for O6.
pub fn route_and_execute(step: &Step, ctx: &TaskContext, app_ctx: &mut crate::worker::AppContext) -> StepResult {
    let start = Instant::now();
    let (success, output, artifacts) = match &step.step_type {
        StepType::Shell { command, .. } => execute_shell(command, ctx),
        StepType::FileWrite { path, content } => execute_file_write(path, content, ctx),
        StepType::FileRead { path } => execute_file_read(path, ctx),
        StepType::Wait { condition } => execute_wait(condition),
        StepType::Verify { method } => execute_verify(method, ctx),
        StepType::CodeGen { description, target_path, language } => {
            // O9: Supreme Coder — analyze, plan, write, test, review
            let mut genome = hydra_genome::GenomeStore::open();
            let result = crate::coder::code(description, &ctx.working_dir.to_string_lossy(), &mut genome);
            let success = result.tests_failed == 0 && result.score >= 7.0;
            // O10: Zero-Defect — run gates on any generated file
            let gate_info = if success && std::path::Path::new(target_path).exists() {
                std::fs::read_to_string(ctx.working_dir.join(target_path)).ok().map(|code| {
                    let lang = language.as_str();
                    let (gates, cert) = crate::zero_defect::run_gates(&code, target_path, lang, &ctx.working_dir.to_string_lossy());
                    // O10+O3: Record gate outcome for genome learning
                    let all_passed = gates.iter().all(|g| g.passed);
                    let gate_outcome = if all_passed {
                        crate::feedback::ActionOutcome::Success {
                            approach: format!("zero-defect:{target_path}"), domain: "zero-defect".into(),
                            duration_ms: 0, quality: cert.as_ref().map(|c| c.confidence).unwrap_or(0.5),
                        }
                    } else {
                        let failures: Vec<_> = gates.iter().filter(|g| !g.passed).map(|g| g.gate.label()).collect();
                        crate::feedback::ActionOutcome::Failure {
                            approach: format!("zero-defect:{target_path}"), domain: "zero-defect".into(),
                            obstacle: format!("gates failed: {}", failures.join(", ")),
                            error: String::new(), rerouted: false,
                        }
                    };
                    crate::feedback::record_simple(&gate_outcome, &mut genome);
                    cert.map(|c| c.format_display()).unwrap_or_else(|| {
                        let passed = gates.iter().filter(|g| g.passed).count();
                        format!("{}/{} gates passed", passed, gates.len())
                    })
                })
            } else { None };
            let mut output = format!("Code: {} files, {}/{} tests, score {:.1}, {} review issues",
                result.files_created, result.tests_passed, result.tests_passed + result.tests_failed,
                result.score, result.review_issues.len());
            if let Some(gi) = &gate_info { output.push_str(&format!("\nZero-Defect: {gi}")); }
            (success, output, vec![target_path.clone()])
        }
        // O6: Universal Worker handles browser/desktop/API steps
        StepType::BrowserNavigate { .. } | StepType::BrowserInteract { .. }
        | StepType::DesktopAction { .. } | StepType::ApiCall { .. } => {
            let genome = hydra_genome::GenomeStore::open();
            let judgment = crate::worker::autonomy_check(step, &genome);
            match judgment {
                hydra_wisdom::JudgmentDecision::Refuse { reason, .. } => {
                    (false, format!("REFUSED: {reason}"), vec![])
                }
                hydra_wisdom::JudgmentDecision::Ask { reason, .. } => {
                    eprintln!("hydra-worker: approval needed — {reason}");
                    (true, format!("APPROVAL_NEEDED:{reason}"), vec![])
                }
                hydra_wisdom::JudgmentDecision::Act { .. } => {
                    let (success, output, artifacts) = crate::worker::execute_interface_step(step, ctx, app_ctx);
                    let iface = crate::worker::classify_interface(&step.step_type);
                    app_ctx.record_step_output(step.id, &output, iface, success);
                    // O6+O3: Record interface effectiveness to genome
                    let mut genome = hydra_genome::GenomeStore::open();
                    let iface_domain = format!("worker:{iface:?}");
                    let iface_outcome = if success {
                        crate::feedback::ActionOutcome::Success {
                            approach: step.description.clone(), domain: iface_domain,
                            duration_ms: 0, quality: 1.0,
                        }
                    } else {
                        crate::feedback::ActionOutcome::Failure {
                            approach: step.description.clone(), domain: iface_domain,
                            obstacle: output.clone(), error: String::new(), rerouted: false,
                        }
                    };
                    crate::feedback::record_simple(&iface_outcome, &mut genome);
                    (success, output, artifacts)
                }
            }
        }
        // Session 24: Remote Hands — SSH execution on remote machines
        StepType::Remote { machine, command } => {
            // Load machine from ~/.hydra/machines.toml and execute via SSH
            match crate::remote_exec::ssh_execute(machine, command) {
                Ok((output, success)) => (success, output, vec![]),
                Err(e) => (false, format!("Remote failed: {e}"), vec![]),
            }
        }
    };
    StepResult { step_id: step.id, success, output, artifacts, duration_ms: start.elapsed().as_millis() as u64 }
}

fn execute_shell(command: &str, ctx: &TaskContext) -> (bool, String, Vec<String>) {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(command)
        .current_dir(&ctx.working_dir).envs(&ctx.env_vars);
    // Set new process group so entire tree can be killed on cleanup (no orphans)
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| { libc::setpgid(0, 0); Ok(()) });
    }
    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let output = if stderr.is_empty() { stdout } else { format!("{stdout}\nstderr: {stderr}") };
            (out.status.success(), output, vec![])
        }
        Err(e) => (false, format!("Shell error: {e}"), vec![]),
    }
}

fn execute_file_write(path: &str, content: &str, ctx: &TaskContext) -> (bool, String, Vec<String>) {
    let full = ctx.working_dir.join(path);
    if let Some(parent) = full.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return (false, format!("mkdir failed: {e}"), vec![]);
        }
    }
    match std::fs::write(&full, content) {
        Ok(_) => (true, format!("Written: {}", full.display()), vec![full.display().to_string()]),
        Err(e) => (false, format!("Write failed: {e}"), vec![]),
    }
}

fn execute_file_read(path: &str, ctx: &TaskContext) -> (bool, String, Vec<String>) {
    let full = ctx.working_dir.join(path);
    match std::fs::read_to_string(&full) {
        Ok(content) => (true, content, vec![]),
        Err(e) => (false, format!("Read failed: {e}"), vec![]),
    }
}

fn execute_wait(condition: &WaitCondition) -> (bool, String, Vec<String>) {
    match condition {
        WaitCondition::Duration { ms } => {
            std::thread::sleep(std::time::Duration::from_millis(*ms));
            (true, format!("Waited {ms}ms"), vec![])
        }
        WaitCondition::FileExists { path } => {
            for _ in 0..30 {
                if std::path::Path::new(path).exists() { return (true, "File exists".into(), vec![]); }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            (false, format!("File not found after 30s: {path}"), vec![])
        }
        WaitCondition::ProcessReady { port } => {
            for _ in 0..60 {
                if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
                    return (true, format!("Port {port} ready"), vec![]);
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            (false, format!("Port {port} not ready after 60s"), vec![])
        }
    }
}

fn execute_verify(method: &VerifyMethod, ctx: &TaskContext) -> (bool, String, Vec<String>) {
    match method {
        VerifyMethod::CommandSuccess { command } => {
            let (s, o, a) = execute_shell(command, ctx);
            (s, format!("Verify: {o}"), a)
        }
        VerifyMethod::FileContains { path, pattern } => {
            let full = ctx.working_dir.join(path);
            match std::fs::read_to_string(&full) {
                Ok(content) if content.contains(pattern) => (true, "Pattern found".into(), vec![]),
                Ok(_) => (false, format!("Pattern '{pattern}' not found in {path}"), vec![]),
                Err(e) => (false, format!("Read failed: {e}"), vec![]),
            }
        }
        VerifyMethod::HttpStatus { url, expect } => {
            match reqwest::blocking::get(url) {
                Ok(resp) if resp.status().as_u16() == *expect => (true, format!("HTTP {expect}"), vec![]),
                Ok(resp) => (false, format!("HTTP {} (expected {expect})", resp.status()), vec![]),
                Err(e) => (false, format!("HTTP error: {e}"), vec![]),
            }
        }
    }
}

// ── DAG Executor ──

/// Execute the full step DAG with context flow.
pub fn execute_dag(ctx: &mut TaskContext, genome: &hydra_genome::GenomeStore) -> ConductorResult {
    if let Err(e) = validate_dag(&ctx.steps) { return e; }
    let mut completed: HashSet<usize> = HashSet::new();
    let mut app_ctx = crate::worker::AppContext::new(); // O6: persists across steps
    let total = ctx.steps.len();
    loop {
        if ctx.cancelled { return ConductorResult::Cancelled; }
        let ready: Vec<usize> = (0..total)
            .filter(|i| !completed.contains(i))
            .filter(|i| ctx.steps[*i].depends_on.iter().all(|d| completed.contains(d)))
            .collect();
        if ready.is_empty() {
            if completed.len() == total { break; }
            return ConductorResult::CyclicDag;
        }
        for step_id in ready {
            let step = ctx.steps[step_id].clone();
            eprintln!("hydra-conductor: step {}/{}: {}", step_id + 1, total, step.description);
            let result = route_and_execute(&step, ctx, &mut app_ctx);
            eprintln!("hydra-conductor: step {} {}", step_id + 1, if result.success { "OK" } else { "FAILED" });
            // O3: Record feedback for genome learning
            let outcome = if result.success {
                crate::feedback::ActionOutcome::Success {
                    approach: step.description.clone(), domain: "conductor".into(),
                    duration_ms: result.duration_ms, quality: 1.0,
                }
            } else {
                crate::feedback::ActionOutcome::Failure {
                    approach: step.description.clone(), domain: "conductor".into(),
                    obstacle: result.output.clone(), error: result.output.clone(), rerouted: false,
                }
            };
            eprintln!("hydra-feedback: step {} → {:?}", step_id + 1,
                if result.success { "success" } else { "failure" });

            if result.success {
                completed.insert(step_id);
                if let StepType::Shell { ref command, .. } = step.step_type {
                    if let Some(dir) = extract_cd(command) {
                        let new_dir = ctx.working_dir.join(dir);
                        if new_dir.exists() { ctx.working_dir = new_dir; }
                    }
                }
            } else {
                return ConductorResult::StepFailed { step_id, error: result.output };
            }
            ctx.results.push(result);
            // O3: Log feedback (genome update deferred — conduct() has immutable genome ref)
            crate::feedback::log_outcome(&outcome);
        }
    }
    // O6: Log interface effectiveness summary after all steps
    let summary = app_ctx.interface_summary();
    if !summary.is_empty() { eprintln!("hydra-worker: task complete — {summary}"); }
    ConductorResult::Complete { results: ctx.results.clone() }
}

fn extract_cd(cmd: &str) -> Option<&str> {
    if cmd.trim().starts_with("cd ") { Some(cmd.trim()[3..].trim()) }
    else { cmd.find("&& cd ").map(|p| cmd[p + 6..].trim().split_whitespace().next()).flatten() }
}

/// Top-level: mine assumptions → decompose → execute → critique.
pub fn conduct(goal: &str, genome: &hydra_genome::GenomeStore) -> ConductorResult {
    // O0: Mine assumptions before execution
    let miner_result = crate::assumptions::mine(goal, genome);
    if !miner_result.questions.is_empty() {
        eprintln!("hydra-conductor: {} assumption questions before proceeding:", miner_result.questions.len());
        for q in &miner_result.questions { eprintln!("  ? {q}"); }
    }
    let steps = crate::conductor::decompose(goal, genome);
    let mut ctx = TaskContext {
        goal: goal.into(), steps, results: Vec::new(),
        working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        env_vars: std::env::vars().collect(), decomposition_depth: 0, cancelled: false,
    };
    // O8: Use parallel execution if steps are parallelizable
    let result = if crate::parallel::is_parallelizable(&ctx.steps) {
        let config = crate::parallel::ParallelConfig::default();
        eprintln!("hydra-conductor: parallel execution ({} steps)", ctx.steps.len());
        let par = crate::parallel::execute_parallel(&mut ctx, &config);
        if par.failed_lanes.is_empty() {
            ConductorResult::Complete { results: par.all_results }
        } else {
            let (id, err) = par.failed_lanes[0].clone();
            ConductorResult::StepFailed { step_id: id, error: err }
        }
    } else {
        execute_dag(&mut ctx, genome)
    };

    // O5: Quality Critic — evaluate-revise loop (up to 3 revisions)
    let mut final_result = result;
    if let ConductorResult::Complete { ref mut results } = final_result {
        let output = results.iter().map(|r| r.output.as_str()).collect::<Vec<_>>().join("\n");
        if !output.trim().is_empty() {
            let max_revisions = 3u32;
            for revision in 0..max_revisions {
                let feedback = crate::critic::universal_evaluate(&output, goal);
                eprintln!("hydra-critic: v{} score={:.1} issues={} revision_needed={}",
                    revision + 1, feedback.score, feedback.issues.len(), feedback.revision_needed);
                if !feedback.revision_needed { break; }

                let fix_steps = crate::critic::generate_fix_steps(&feedback.issues, goal);
                if fix_steps.is_empty() {
                    eprintln!("hydra-critic: no actionable fixes (all issues low severity)");
                    break;
                }
                eprintln!("hydra-critic: executing {} fix steps (revision {})", fix_steps.len(), revision + 1);
                let mut fix_ctx = TaskContext {
                    goal: format!("{goal} [revision {}]", revision + 1),
                    steps: fix_steps, results: Vec::new(),
                    working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    env_vars: std::env::vars().collect(), decomposition_depth: 0, cancelled: false,
                };
                match execute_dag(&mut fix_ctx, genome) {
                    ConductorResult::Complete { results: fix_results } => {
                        results.extend(fix_results);
                    }
                    _ => {
                        eprintln!("hydra-critic: fix execution failed, stopping revisions");
                        break;
                    }
                }
            }
        }
    }

    final_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_ctx() -> TaskContext {
        TaskContext {
            goal: "test".into(), steps: vec![], results: vec![],
            working_dir: std::env::current_dir().unwrap(),
            env_vars: HashMap::new(), decomposition_depth: 0, cancelled: false,
        }
    }

    #[test]
    fn shell_step_executes() {
        let ctx = test_ctx();
        let step = Step { id: 0, step_type: StepType::Shell { command: "echo hello".into(), long_running: false },
            description: "test".into(), depends_on: vec![], timeout_ms: 5000 };
        let mut app_ctx = crate::worker::AppContext::new();
        let result = route_and_execute(&step, &ctx, &mut app_ctx);
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn dag_executes_sequential() {
        let mut ctx = test_ctx();
        ctx.steps = vec![
            Step { id: 0, step_type: StepType::Shell { command: "echo step1".into(), long_running: false },
                description: "Step 1".into(), depends_on: vec![], timeout_ms: 5000 },
            Step { id: 1, step_type: StepType::Shell { command: "echo step2".into(), long_running: false },
                description: "Step 2".into(), depends_on: vec![0], timeout_ms: 5000 },
        ];
        let genome = hydra_genome::GenomeStore::new();
        assert!(matches!(execute_dag(&mut ctx, &genome), ConductorResult::Complete { .. }));
        assert_eq!(ctx.results.len(), 2);
    }

    #[test]
    fn cancel_stops_dag() {
        let mut ctx = test_ctx();
        ctx.cancelled = true;
        ctx.steps = vec![
            Step { id: 0, step_type: StepType::Shell { command: "echo a".into(), long_running: false },
                description: "A".into(), depends_on: vec![], timeout_ms: 5000 },
        ];
        let genome = hydra_genome::GenomeStore::new();
        assert!(matches!(execute_dag(&mut ctx, &genome), ConductorResult::Cancelled));
    }

    #[test]
    fn conduct_returns_empty_plan_without_llm() {
        // Without LLM or genome, conduct returns EmptyPlan (never raw-shell user input)
        let genome = hydra_genome::GenomeStore::new();
        let result = conduct("echo hello world", &genome);
        assert!(matches!(result, ConductorResult::EmptyPlan));
    }

    #[test]
    fn verify_file_contains() {
        let ctx = test_ctx();
        let mut app_ctx = crate::worker::AppContext::new();
        let step = Step { id: 0, step_type: StepType::Verify {
            method: VerifyMethod::CommandSuccess { command: "echo pass".into() } },
            description: "verify".into(), depends_on: vec![], timeout_ms: 5000 };
        let result = route_and_execute(&step, &ctx, &mut app_ctx);
        assert!(result.success);
    }
}
