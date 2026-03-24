//! Orchestrator — the full semantic navigation pipeline.
//! parse → afford → route → execute → verify. 50ms vs 5,000ms.
//! Single entry point: try_semantic_nav().

use crate::affordance;
use crate::constitution_cache::ConstitutionCache;
use crate::dom_parser;
use crate::executor;
use crate::intent_router;
use crate::types::*;
use crate::verifier;

/// The single entry point for semantic navigation.
/// Returns NavResult::Success if handled, NavResult::Unparseable to fall back to vision.
pub async fn try_semantic_nav(
    engine: &mut hydra_browser::BrowserEngine,
    goal: &str,
    update_fn: &(dyn Fn(u32, &str, &str, bool) + Send + Sync),
) -> NavResult {
    let start = std::time::Instant::now();
    let mut cache = ConstitutionCache::new();

    // 1. Get current URL for cache key
    let url = get_current_url(engine).await;

    // 2. Check constitution cache (instant if hit)
    if let Some(cached) = cache.check(&url) {
        eprintln!("hydra-semantic-nav: cache hit for {url}");
        return execute_with_constitution(engine, goal, cached, &url, update_fn).await;
    }

    // 3. Get page data via CDP
    let html = match engine.html().await {
        Ok(h) => h,
        Err(e) => return NavResult::Unparseable(format!("Cannot read HTML: {e}")),
    };

    // 4. Check if DOM is parseable
    if !dom_parser::is_dom_parseable(&html) {
        return NavResult::Unparseable("DOM not parseable (canvas/SVG only)".into());
    }

    // 5. Get interactive elements via JS
    let elements_result = engine.execute(&hydra_browser::BrowserAction::GetElements).await;
    let elements_json = if elements_result.success { &elements_result.data } else { "[]" };

    // 6. Parse DOM into semantic elements
    let elements = dom_parser::parse_page(elements_json, &html);
    if elements.is_empty() {
        return NavResult::Unparseable("No interactive elements found".into());
    }
    eprintln!("hydra-semantic-nav: parsed {} semantic elements", elements.len());

    // 7. Build page constitution
    let constitution = affordance::build_constitution(elements, &html, &url);

    // 8. Handle guards (cookie banners, modals) first
    if !constitution.guards.is_empty() {
        eprintln!("hydra-semantic-nav: dismissing {} guards", constitution.guards.len());
        for guard in &constitution.guards {
            let _ = engine.execute(&hydra_browser::BrowserAction::Click {
                selector: guard.selector.clone(),
            }).await;
            engine.execute(&hydra_browser::BrowserAction::Wait { ms: 500 }).await;
        }
        // Re-parse after dismissing guards
        let html2 = engine.html().await.unwrap_or_default();
        let elements_result2 = engine.execute(&hydra_browser::BrowserAction::GetElements).await;
        let elements2 = dom_parser::parse_page(
            if elements_result2.success { &elements_result2.data } else { "[]" }, &html2);
        let constitution2 = affordance::build_constitution(elements2, &html2, &url);
        return execute_with_constitution(engine, goal, &constitution2, &url, update_fn).await;
    }

    // 9. Execute with constitution
    let result = execute_with_constitution(engine, goal, &constitution, &url, update_fn).await;

    // 10. Cache the constitution for future visits
    cache.store(&constitution);

    let ms = start.elapsed().as_millis();
    eprintln!("hydra-semantic-nav: completed in {ms}ms");

    result
}

async fn execute_with_constitution(
    engine: &mut hydra_browser::BrowserEngine,
    goal: &str,
    constitution: &PageConstitution,
    url: &str,
    update_fn: &(dyn Fn(u32, &str, &str, bool) + Send + Sync),
) -> NavResult {
    // Route intent to execution plan
    let plan = match intent_router::route(goal, constitution) {
        Some(p) => p,
        None => return NavResult::Unparseable(
            "No affordance matches the goal with sufficient confidence".into(),
        ),
    };

    eprintln!(
        "hydra-semantic-nav: plan '{}' ({} steps, confidence {:.2})",
        plan.strategy, plan.steps.len(), plan.confidence
    );

    // Execute the plan
    match executor::execute_plan(engine, &plan, update_fn).await {
        Ok(()) => {}
        Err(e) => {
            eprintln!("hydra-semantic-nav: execution failed: {e}");
            return NavResult::Unparseable(format!("Execution failed: {e}"));
        }
    }

    // Verify success
    let verify = verifier::verify(engine, url, &plan.strategy).await;
    if verify.success {
        eprintln!("hydra-semantic-nav: verified — {}", verify.observation);
        NavResult::Success
    } else {
        eprintln!("hydra-semantic-nav: verification failed — {}", verify.observation);
        NavResult::Unparseable(format!("Verification failed: {}", verify.observation))
    }
}

async fn get_current_url(_engine: &hydra_browser::BrowserEngine) -> String {
    // BrowserEngine doesn't expose a current_url() method.
    // The caller should use try_semantic_nav_with_url() instead.
    "unknown".to_string()
}

/// Convenience: try semantic nav with a known URL (called from agent_task).
pub async fn try_semantic_nav_with_url(
    engine: &mut hydra_browser::BrowserEngine,
    goal: &str,
    url: &str,
    update_fn: &(dyn Fn(u32, &str, &str, bool) + Send + Sync),
) -> NavResult {
    let start = std::time::Instant::now();
    let mut cache = ConstitutionCache::new();

    // Check cache
    if let Some(cached) = cache.check(url) {
        eprintln!("hydra-semantic-nav: cache hit for {url}");
        return execute_with_constitution(engine, goal, cached, url, update_fn).await;
    }

    // Get page data
    let html = match engine.html().await {
        Ok(h) => h,
        Err(e) => return NavResult::Unparseable(format!("Cannot read HTML: {e}")),
    };
    if !dom_parser::is_dom_parseable(&html) {
        return NavResult::Unparseable("DOM not parseable".into());
    }

    let el_result = engine.execute(&hydra_browser::BrowserAction::GetElements).await;
    let el_json = if el_result.success { &el_result.data } else { "[]" };
    let elements = dom_parser::parse_page(el_json, &html);
    if elements.is_empty() {
        return NavResult::Unparseable("No interactive elements".into());
    }

    let constitution = affordance::build_constitution(elements, &html, url);

    // Dismiss guards
    for guard in &constitution.guards {
        let _ = engine.execute(&hydra_browser::BrowserAction::Click {
            selector: guard.selector.clone(),
        }).await;
        engine.execute(&hydra_browser::BrowserAction::Wait { ms: 500 }).await;
    }

    let result = execute_with_constitution(engine, goal, &constitution, url, update_fn).await;
    cache.store(&constitution);

    eprintln!("hydra-semantic-nav: completed in {}ms", start.elapsed().as_millis());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_result_is_success_or_unparseable() {
        let s = NavResult::Success;
        assert!(matches!(s, NavResult::Success));
        let u = NavResult::Unparseable("test".into());
        assert!(matches!(u, NavResult::Unparseable(_)));
    }
}
