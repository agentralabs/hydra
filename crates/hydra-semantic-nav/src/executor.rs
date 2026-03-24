//! Executor — converts an ExecutionPlan into CDP interactions via BrowserEngine.
//! Constitutional checks on every action. Human-like delays built in.

use crate::types::*;

/// Execute a plan step-by-step via BrowserEngine.
/// Sends AgentUpdate-compatible messages via the update sender.
pub async fn execute_plan(
    engine: &mut hydra_browser::BrowserEngine,
    plan: &ExecutionPlan,
    update_fn: &(dyn Fn(u32, &str, &str, bool) + Send + Sync),
) -> Result<(), hydra_browser::BrowserError> {
    eprintln!(
        "hydra-semantic-nav: executing plan ({} steps, confidence {:.2}): {}",
        plan.steps.len(), plan.confidence, plan.strategy
    );

    for (i, step) in plan.steps.iter().enumerate() {
        let step_num = (i + 1) as u32;

        // Report step start
        update_fn(step_num, &step.description, "", false);

        // Execute the browser action
        let result = engine.execute(&step.action).await;

        if !result.success {
            let err_msg = result.error.unwrap_or_else(|| "Unknown error".into());
            eprintln!("hydra-semantic-nav: step {} failed: {}", step_num, err_msg);

            // Try fallback: broader selector (strip specifics)
            if let Some(fallback) = broaden_selector(&step.selector) {
                let fallback_action = replace_selector(&step.action, &fallback);
                let retry = engine.execute(&fallback_action).await;
                if retry.success {
                    update_fn(step_num, &step.description, "ok (fallback selector)", false);
                    continue;
                }
            }

            update_fn(step_num, &step.description, &format!("failed: {err_msg}"), false);
            return Err(hydra_browser::BrowserError::ActionFailed {
                action: step.description.clone(),
                reason: err_msg,
            });
        }

        // Short wait for page to settle (faster than vision's 1000ms)
        engine.execute(&hydra_browser::BrowserAction::Wait { ms: 300 }).await;

        let is_last = i == plan.steps.len() - 1;
        update_fn(step_num, &step.description, &result.data, is_last);
    }

    Ok(())
}

/// Broaden a selector by removing specifics (nth-child, attribute values).
fn broaden_selector(selector: &str) -> Option<String> {
    // If it's an ID selector, no fallback
    if selector.starts_with('#') { return None; }
    // If it has an attribute, try just the tag
    if let Some(bracket) = selector.find('[') {
        let tag = &selector[..bracket];
        if !tag.is_empty() { return Some(tag.to_string()); }
    }
    None
}

/// Replace the selector in a BrowserAction.
fn replace_selector(action: &hydra_browser::BrowserAction, new_selector: &str) -> hydra_browser::BrowserAction {
    match action {
        hydra_browser::BrowserAction::Click { .. } => {
            hydra_browser::BrowserAction::Click { selector: new_selector.to_string() }
        }
        hydra_browser::BrowserAction::Type { text, .. } => {
            hydra_browser::BrowserAction::Type { selector: new_selector.to_string(), text: text.clone() }
        }
        hydra_browser::BrowserAction::Hover { .. } => {
            hydra_browser::BrowserAction::Hover { selector: new_selector.to_string() }
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broaden_attribute_selector() {
        assert_eq!(broaden_selector("button[aria-label=\"Post\"]"), Some("button".into()));
        assert_eq!(broaden_selector("[name=\"email\"]"), None); // no tag prefix
        assert_eq!(broaden_selector("#submit-btn"), None); // ID — no fallback
    }

    #[test]
    fn replace_click_selector() {
        let action = hydra_browser::BrowserAction::Click { selector: "old".into() };
        let replaced = replace_selector(&action, "new");
        assert!(matches!(replaced, hydra_browser::BrowserAction::Click { selector } if selector == "new"));
    }
}
