//! Orchestrator — the full swarm browser pipeline.
//! Decompose → spawn N workers → collect → merge → consensus → store.

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::constants::WORKER_TIMEOUT_SECS;
use crate::decomposer;
use crate::merger;
use crate::types::*;
use crate::worker;

/// Execute a full swarm research operation.
pub async fn execute_swarm(
    goal: SwarmGoal,
    update_tx: mpsc::Sender<SwarmUpdate>,
) -> SwarmResponse {
    let start = Instant::now();

    // 1. DECOMPOSE goal into sub-tasks
    let _ = update_tx.send(SwarmUpdate::Decomposing {
        goal: goal.description.clone(),
    }).await;
    let tasks = decomposer::decompose(&goal);
    eprintln!("hydra-swarm: {} tasks for '{}' across {} workers",
        tasks.len(), goal.description, goal.max_workers);

    // 2. SPAWN workers in parallel
    let (result_tx, mut result_rx) = mpsc::channel::<WorkerResult>(64);
    let worker_count = tasks.len();

    for task in tasks {
        let result_tx = result_tx.clone();
        let update_tx = update_tx.clone();
        let worker_id = uuid::Uuid::new_v4();

        tokio::spawn(async move {
            let result = worker::run_worker(task, worker_id, update_tx).await;
            let _ = result_tx.send(result).await;
        });
    }
    drop(result_tx); // Close sender so receiver completes when all workers finish

    // 3. COLLECT results with timeout
    let mut results = Vec::new();
    let timeout = tokio::time::Duration::from_secs(WORKER_TIMEOUT_SECS);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        tokio::select! {
            result = result_rx.recv() => {
                match result {
                    Some(r) => results.push(r),
                    None => break, // All senders dropped = all workers done
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                eprintln!("hydra-swarm: timeout after {}s, collected {}/{} results",
                    WORKER_TIMEOUT_SECS, results.len(), worker_count);
                break;
            }
        }
    }

    eprintln!("hydra-swarm: collected {} results", results.len());

    // 4. MERGE results
    let _ = update_tx.send(SwarmUpdate::Merging { count: results.len() }).await;
    let merged = merger::merge_results(&results, &goal);

    // 5. CONSENSUS check
    let consensus_reached = merger::check_consensus(&results);
    eprintln!("hydra-swarm: consensus: {consensus_reached}");

    let total_duration_ms = start.elapsed().as_millis() as u64;

    // 6. BUILD response
    let response = SwarmResponse {
        goal,
        results,
        merged,
        consensus_reached,
        genome_entries_created: 0, // TODO: wire genome storage
        total_duration_ms,
    };

    let _ = update_tx.send(SwarmUpdate::Complete(response.clone())).await;
    response
}

/// Synchronous wrapper for TUI command handlers.
pub fn execute_swarm_blocking(goal_description: &str) -> Result<String, String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4) // Need enough threads for parallel Chrome instances
        .enable_all()
        .build()
        .map_err(|e| format!("Runtime: {e}"))?;

    let goal = SwarmGoal::new(goal_description, crate::constants::DEFAULT_POOL_SIZE);
    let (tx, _rx) = mpsc::channel(128);

    let response = rt.block_on(execute_swarm(goal, tx));
    Ok(response.format_display())
}

/// Spawn a swarm operation asynchronously. Returns a receiver for progress.
pub fn spawn_swarm(
    rt: &tokio::runtime::Runtime,
    goal_description: String,
) -> mpsc::Receiver<SwarmUpdate> {
    let (tx, rx) = mpsc::channel(128);
    let goal = SwarmGoal::new(goal_description, crate::constants::DEFAULT_POOL_SIZE);
    rt.spawn(async move { execute_swarm(goal, tx).await });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swarm_goal_creates() {
        let goal = SwarmGoal::new("test", 5);
        assert_eq!(goal.max_workers, 5);
    }
}
