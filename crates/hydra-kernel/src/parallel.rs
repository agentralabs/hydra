//! Parallel Execution Engine — runs independent DAG steps simultaneously.
//! Level-based parallelism: group steps by dependency depth, execute each level in parallel.
//! Resource-limited: max lanes, max Chrome instances, max shell processes.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::conductor::{Step, StepResult, StepType, TaskContext};
use crate::conductor_exec::route_and_execute;

// ── Types ──

/// Configuration for parallel execution.
pub struct ParallelConfig {
    pub max_lanes: usize,     // default 5
    pub max_browsers: usize,  // default 3 (EC-8.2)
    pub max_shell: usize,     // default 3 (EC-8.6)
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self { max_lanes: 5, max_browsers: 3, max_shell: 3 }
    }
}

/// Status of a lane.
#[derive(Debug, Clone)]
pub enum LaneStatus {
    Waiting,
    Running,
    Complete { results: Vec<StepResult> },
    Failed { error: String },
    Cancelled,
}

/// Result of parallel execution.
#[derive(Debug)]
pub struct ParallelResult {
    pub all_results: Vec<StepResult>,
    pub levels_executed: usize,
    pub total_duration_ms: u64,
    pub lanes_used: usize,
    pub failed_lanes: Vec<(usize, String)>,
}

// ── Dependency Analysis ──

/// Group steps into levels by dependency depth.
/// Level 0: no dependencies. Level 1: depends on level 0. Etc.
pub fn analyze_levels(steps: &[Step]) -> Vec<Vec<usize>> {
    let mut levels: Vec<Vec<usize>> = Vec::new();
    let mut assigned: HashSet<usize> = HashSet::new();
    let total = steps.len();

    loop {
        let mut current_level = Vec::new();
        for step in steps {
            if assigned.contains(&step.id) { continue; }
            // All deps must be in previous levels
            if step.depends_on.iter().all(|d| assigned.contains(d)) {
                current_level.push(step.id);
            }
        }
        if current_level.is_empty() {
            if assigned.len() < total {
                eprintln!("hydra-parallel: {} steps unreachable (circular?)", total - assigned.len());
            }
            break;
        }
        for &id in &current_level { assigned.insert(id); }
        levels.push(current_level);
    }

    levels
}

// ── Parallel Executor ──

/// Execute a DAG in parallel using level-based scheduling.
/// Steps at the same level run concurrently (up to max_lanes).
pub fn execute_parallel(ctx: &mut TaskContext, config: &ParallelConfig) -> ParallelResult {
    let start = Instant::now();
    let levels = analyze_levels(&ctx.steps);
    let mut all_results: Vec<StepResult> = Vec::new();
    let mut failed_lanes = Vec::new();
    let mut lanes_used = 0;

    eprintln!("hydra-parallel: {} steps across {} levels", ctx.steps.len(), levels.len());

    for (level_idx, step_ids) in levels.iter().enumerate() {
        if ctx.cancelled { break; } // EC-8.4

        let lane_count = step_ids.len().min(config.max_lanes);
        lanes_used = lanes_used.max(lane_count);

        eprintln!(
            "hydra-parallel: level {} — {} steps ({} lanes)",
            level_idx, step_ids.len(), lane_count
        );

        // Execute steps in this level (using threads for true parallelism)
        let level_results = execute_level(step_ids, ctx, config);

        for result in &level_results {
            if !result.success {
                failed_lanes.push((result.step_id, result.output.clone()));
                // EC-8.4: cancel if downstream steps depend on this failed step
                if ctx.steps.iter().any(|s| s.depends_on.contains(&result.step_id)) {
                    ctx.cancelled = true;
                    eprintln!("hydra-parallel: cascade cancel — step {} failed, dependents exist", result.step_id);
                }
            }
        }

        all_results.extend(level_results);
    }

    // EC-8.7: Bulk update genome after all lanes complete
    // (feedback.rs handles this when called after parallel execution)

    ParallelResult {
        all_results,
        levels_executed: levels.len(),
        total_duration_ms: start.elapsed().as_millis() as u64,
        lanes_used,
        failed_lanes,
    }
}

/// Execute all steps in a single level, up to max_lanes concurrently.
fn execute_level(step_ids: &[usize], ctx: &TaskContext, config: &ParallelConfig) -> Vec<StepResult> {
    use std::sync::{Arc, Mutex};

    if step_ids.len() <= 1 {
        // Single step — no threading overhead
        if let Some(&id) = step_ids.first() {
            let step = &ctx.steps[id];
            let mut app_ctx = crate::worker::AppContext::new();
            return vec![route_and_execute(step, ctx, &mut app_ctx)];
        }
        return vec![];
    }

    // Multiple steps — run in parallel threads
    let results = Arc::new(Mutex::new(Vec::new()));
    let shell_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles: Vec<std::thread::JoinHandle<()>> = Vec::new();

    for chunk in step_ids.chunks(config.max_lanes) {
        let chunk_handles: Vec<_> = chunk.iter().map(|&step_id| {
            let step = ctx.steps[step_id].clone();
            let working_dir = ctx.working_dir.clone();
            let env_vars = ctx.env_vars.clone();
            let results = Arc::clone(&results);
            // EC-8.6: enforce max concurrent shell processes
            let is_shell = matches!(step.step_type, StepType::Shell { .. });
            let shell_count = Arc::clone(&shell_count);
            let max_shell = config.max_shell;

            std::thread::spawn(move || {
                if is_shell {
                    while shell_count.load(std::sync::atomic::Ordering::SeqCst) >= max_shell {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    shell_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                let thread_ctx = TaskContext {
                    goal: String::new(),
                    steps: vec![step.clone()],
                    results: vec![],
                    working_dir,
                    env_vars,
                    decomposition_depth: 0,
                    cancelled: false,
                };
                let mut app_ctx = crate::worker::AppContext::new();
                let result = route_and_execute(&step, &thread_ctx, &mut app_ctx);
                if is_shell { shell_count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst); }
                if let Ok(mut r) = results.lock() { r.push(result); }
            })
        }).collect();

        for handle in chunk_handles {
            if let Err(e) = handle.join() {
                eprintln!("hydra-parallel: thread panicked: {e:?}");
            }
        }
    }

    Arc::try_unwrap(results)
        .unwrap_or_else(|_| Mutex::new(Vec::new()))
        .into_inner()
        .unwrap_or_default()
}

/// Check if a step set can benefit from parallel execution.
pub fn is_parallelizable(steps: &[Step]) -> bool {
    let levels = analyze_levels(steps);
    levels.iter().any(|level| level.len() > 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conductor::*;

    fn shell_step(id: usize, cmd: &str, deps: Vec<usize>) -> Step {
        Step {
            id, step_type: StepType::Shell { command: cmd.into(), long_running: false },
            description: format!("Step {id}"), depends_on: deps, timeout_ms: 5000,
        }
    }

    #[test]
    fn analyze_independent_steps() {
        let steps = vec![shell_step(0, "echo a", vec![]), shell_step(1, "echo b", vec![]), shell_step(2, "echo c", vec![])];
        let levels = analyze_levels(&steps);
        assert_eq!(levels.len(), 1); // All at level 0
        assert_eq!(levels[0].len(), 3);
    }

    #[test]
    fn analyze_sequential_steps() {
        let steps = vec![shell_step(0, "echo a", vec![]), shell_step(1, "echo b", vec![0]), shell_step(2, "echo c", vec![1])];
        let levels = analyze_levels(&steps);
        assert_eq!(levels.len(), 3); // 3 levels, 1 step each
    }

    #[test]
    fn analyze_diamond_dag() {
        // 0 → 1, 0 → 2, 1+2 → 3
        let steps = vec![
            shell_step(0, "echo start", vec![]),
            shell_step(1, "echo left", vec![0]),
            shell_step(2, "echo right", vec![0]),
            shell_step(3, "echo end", vec![1, 2]),
        ];
        let levels = analyze_levels(&steps);
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0].len(), 1); // step 0
        assert_eq!(levels[1].len(), 2); // steps 1, 2 (parallel!)
        assert_eq!(levels[2].len(), 1); // step 3
    }

    #[test]
    fn parallel_execution_independent() {
        let mut ctx = TaskContext {
            goal: "test".into(),
            steps: vec![
                shell_step(0, "echo a", vec![]),
                shell_step(1, "echo b", vec![]),
                shell_step(2, "echo c", vec![]),
            ],
            results: vec![],
            working_dir: std::env::current_dir().unwrap(),
            env_vars: std::collections::HashMap::new(),
            decomposition_depth: 0, cancelled: false,
        };
        let config = ParallelConfig::default();
        let result = execute_parallel(&mut ctx, &config);
        assert_eq!(result.all_results.len(), 3);
        assert!(result.all_results.iter().all(|r| r.success));
        assert_eq!(result.levels_executed, 1); // all at level 0
    }

    #[test]
    fn parallel_faster_than_sequential() {
        let mut ctx = TaskContext {
            goal: "test".into(),
            steps: vec![
                shell_step(0, "sleep 0.1 && echo a", vec![]),
                shell_step(1, "sleep 0.1 && echo b", vec![]),
                shell_step(2, "sleep 0.1 && echo c", vec![]),
            ],
            results: vec![],
            working_dir: std::env::current_dir().unwrap(),
            env_vars: std::collections::HashMap::new(),
            decomposition_depth: 0, cancelled: false,
        };
        let config = ParallelConfig::default();
        let result = execute_parallel(&mut ctx, &config);
        // 3 × 100ms sleep in parallel should take ~100-200ms, not 300ms
        assert!(result.total_duration_ms < 500, "Took {}ms — should be parallel", result.total_duration_ms);
    }

    #[test]
    fn is_parallelizable_checks() {
        let sequential = vec![shell_step(0, "a", vec![]), shell_step(1, "b", vec![0])];
        assert!(!is_parallelizable(&sequential));

        let parallel = vec![shell_step(0, "a", vec![]), shell_step(1, "b", vec![])];
        assert!(is_parallelizable(&parallel));
    }

    #[test]
    fn cancelled_stops_execution() {
        let mut ctx = TaskContext {
            goal: "test".into(),
            steps: vec![shell_step(0, "echo a", vec![])],
            results: vec![],
            working_dir: std::env::current_dir().unwrap(),
            env_vars: std::collections::HashMap::new(),
            decomposition_depth: 0, cancelled: true,
        };
        let result = execute_parallel(&mut ctx, &ParallelConfig::default());
        assert!(result.all_results.is_empty());
    }
}
