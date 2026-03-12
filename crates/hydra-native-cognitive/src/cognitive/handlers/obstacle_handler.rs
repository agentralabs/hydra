//! Obstacle resolution handler — wires ObstacleResolver into the cognitive loop.
//!
//! Called when any phase in the cognitive loop encounters an error.
//! Detects the obstacle pattern, checks for known solutions, and if needed
//! uses LLM to diagnose and generate fix strategies.

use tokio::sync::mpsc;
use crate::cognitive::loop_runner::CognitiveUpdate;
use crate::cognitive::obstacles::{
    Obstacle, ObstacleResolver, Resolution, ResolverConfig,
    diagnoser,
};

/// Attempt to resolve an obstacle that occurred during the cognitive loop.
///
/// Returns `Some(resolution)` if the resolver could handle it,
/// or `None` if it should be escalated to the user directly.
pub(crate) async fn try_resolve_obstacle(
    error_msg: &str,
    task_context: &str,
    resolver: &mut ObstacleResolver,
    llm_config: &hydra_model::LlmConfig,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Option<Resolution> {
    let obstacle = Obstacle::from_error(error_msg, task_context);

    // Notify UI that an obstacle was detected
    let _ = tx.send(CognitiveUpdate::ObstacleDetected {
        pattern: obstacle.pattern.label().to_string(),
        error_summary: error_msg.lines().next().unwrap_or("unknown").to_string(),
    });

    // 1. Check for known solution first (no LLM needed)
    if let Some(_solution) = resolver.lookup_known_solution(&obstacle) {
        let resolution = resolver.resolve_with_strategies(&obstacle, None, vec![]);
        let _ = tx.send(CognitiveUpdate::ObstacleResolved {
            pattern: obstacle.pattern.label().to_string(),
            resolution: resolution.summary(),
            attempts: 0,
        });
        return Some(resolution);
    }

    // 2. Skip LLM diagnosis for non-auto-resolvable patterns
    if !obstacle.pattern.is_auto_resolvable() {
        let resolution = Resolution::NeedsApproval {
            pattern: obstacle.pattern.clone(),
        };
        let _ = tx.send(CognitiveUpdate::ObstacleResolved {
            pattern: obstacle.pattern.label().to_string(),
            resolution: resolution.summary(),
            attempts: 0,
        });
        return Some(resolution);
    }

    // 3. Try LLM diagnosis
    let diagnosis_prompt = resolver.build_diagnosis_prompt(&obstacle);
    let diagnosis = match call_llm_for_obstacle(llm_config, &diagnosis_prompt).await {
        Some(response) => diagnoser::parse_diagnosis(&response).ok(),
        None => None,
    };

    // 4. Try LLM strategy generation
    let strategies = if let Some(ref diag) = diagnosis {
        let strategy_prompt = resolver.build_strategy_prompt(&obstacle, diag);
        match call_llm_for_obstacle(llm_config, &strategy_prompt).await {
            Some(response) => diagnoser::parse_strategies(&response).unwrap_or_default(),
            None => vec![],
        }
    } else {
        vec![]
    };

    // 5. Resolve with whatever we got
    let resolution = resolver.resolve_with_strategies(&obstacle, diagnosis, strategies.clone());

    // 6. Store successful solution for future
    if resolution.is_fixed() {
        if let Some(strategy) = strategies.first() {
            resolver.store_solution(&obstacle, strategy);
        }
    }

    // 7. Notify UI
    let attempts = match &resolution {
        Resolution::Fixed { attempts, .. } => *attempts,
        _ => 0,
    };
    let _ = tx.send(CognitiveUpdate::ObstacleResolved {
        pattern: obstacle.pattern.label().to_string(),
        resolution: resolution.summary(),
        attempts,
    });

    Some(resolution)
}

/// Simple LLM call for obstacle diagnosis (reuses existing infrastructure).
async fn call_llm_for_obstacle(
    llm_config: &hydra_model::LlmConfig,
    prompt: &str,
) -> Option<String> {
    let system = "You are a Rust expert debugging assistant. Analyze errors and suggest fixes.";
    match hydra_kernel::self_modify_llm::call_llm(prompt, system, 2000, llm_config).await {
        Ok(response) => Some(response),
        Err(e) => {
            eprintln!("[hydra:obstacle] LLM call failed: {}", e);
            None
        }
    }
}

/// Create a default ObstacleResolver for the cognitive loop.
pub(crate) fn create_resolver() -> ObstacleResolver {
    ObstacleResolver::new(ResolverConfig::default())
}

/// Format an obstacle resolution for display in the conversation.
pub(crate) fn format_resolution_message(
    pattern: &str,
    resolution: &str,
    original_error: &str,
) -> String {
    format!(
        "**Obstacle detected:** {}\n\n\
         > {}\n\n\
         **Resolution:** {}",
        pattern,
        original_error.lines().next().unwrap_or("unknown error"),
        resolution,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_resolver() {
        let r = create_resolver();
        assert_eq!(r.stats().obstacles_seen, 0);
    }

    #[test]
    fn test_format_resolution_message() {
        let msg = format_resolution_message(
            "Compilation Error",
            "Fixed after 1 attempt(s): add missing import",
            "error[E0433]: unresolved import\n  --> src/lib.rs:5",
        );
        assert!(msg.contains("Compilation Error"));
        assert!(msg.contains("add missing import"));
        assert!(msg.contains("error[E0433]"));
    }

    #[test]
    fn test_resolve_known_solution_sync() {
        let mut resolver = create_resolver();
        let obstacle = Obstacle::from_error("error[E0433]: test", "task");
        let strategy = crate::cognitive::obstacles::Strategy {
            description: "add import".into(),
            actions: vec![],
            risk_level: crate::cognitive::obstacles::RiskLevel::Low,
        };
        resolver.store_solution(&obstacle, &strategy);

        // Now resolving should find from memory
        let resolution = resolver.resolve_with_strategies(&obstacle, None, vec![]);
        assert!(resolution.is_fixed());
    }

    #[test]
    fn test_resolve_non_auto_resolvable() {
        let mut resolver = create_resolver();
        let obstacle = Obstacle::from_error("permission denied: /etc/passwd", "task");
        let resolution = resolver.resolve_with_strategies(&obstacle, None, vec![]);
        assert!(matches!(resolution, Resolution::NeedsApproval { .. }));
    }
}
