//! Live test for semantic affordance navigation.
//! Run with: cargo run -p hydra-semantic-nav --bin nav_test

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        println!("=== Hydra Semantic Affordance Navigation — Live Test ===\n");

        // Test 1: Parse a real website's DOM
        println!("[TEST 1] Launch Chrome + navigate to example.com");
        let mut engine = hydra_browser::BrowserEngine::new();
        match engine.launch().await {
            Ok(_) => println!("  Chrome launched"),
            Err(e) => { println!("  SKIP: Chrome not available ({e})"); return; }
        }

        if let Err(e) = engine.navigate("https://example.com").await {
            println!("  SKIP: Navigation failed ({e})");
            engine.close().await;
            return;
        }
        println!("  Navigated to example.com");

        // Test 2: Get elements + HTML
        println!("\n[TEST 2] Parse DOM into semantic elements");
        let html = engine.html().await.unwrap_or_default();
        let el_result = engine.execute(&hydra_browser::BrowserAction::GetElements).await;
        let el_json = if el_result.success { &el_result.data } else { "[]" };

        let is_parseable = hydra_semantic_nav::dom_parser::is_dom_parseable(&html);
        println!("  DOM parseable: {is_parseable}");

        let elements = hydra_semantic_nav::dom_parser::parse_page(el_json, &html);
        println!("  Semantic elements found: {}", elements.len());
        for (i, el) in elements.iter().take(5).enumerate() {
            println!("    {}: {:?} '{}' [{}]", i, el.role, el.label, el.selector);
        }

        // Test 3: Build page constitution
        println!("\n[TEST 3] Build page constitution");
        let constitution = hydra_semantic_nav::affordance::build_constitution(
            elements, &html, "https://example.com"
        );
        println!("  Title: {}", constitution.title);
        println!("  Forms: {}", constitution.forms.len());
        println!("  Nav links: {}", constitution.navigation.len());
        println!("  Primary CTA: {:?}", constitution.primary_action);
        println!("  Search input: {:?}", constitution.search_input);
        println!("  Guards: {}", constitution.guards.len());

        // Test 4: Try semantic nav on GitHub
        println!("\n[TEST 4] Semantic nav on GitHub search page");
        if let Err(e) = engine.navigate("https://github.com/search").await {
            println!("  SKIP: Navigation failed ({e})");
        } else {
            let html2 = engine.html().await.unwrap_or_default();
            let el2 = engine.execute(&hydra_browser::BrowserAction::GetElements).await;
            let el2_json = if el2.success { &el2.data } else { "[]" };
            let elements2 = hydra_semantic_nav::dom_parser::parse_page(el2_json, &html2);
            let constitution2 = hydra_semantic_nav::affordance::build_constitution(
                elements2, &html2, "https://github.com/search"
            );
            println!("  Elements: {}", constitution2.elements.len());
            println!("  Search input: {:?}", constitution2.search_input);

            // Try routing "search for hydra"
            let plan = hydra_semantic_nav::intent_router::route("search for hydra", &constitution2);
            match plan {
                Some(p) => {
                    println!("  Plan: {} ({} steps, confidence {:.2})", p.strategy, p.steps.len(), p.confidence);
                    for step in &p.steps {
                        println!("    → {}: {}", step.description, step.selector);
                    }
                    println!("  PASS: Intent routed to search affordance");
                }
                None => println!("  INFO: No matching affordance (GitHub may need JS rendering)"),
            }
        }

        // Test 5: Full semantic nav attempt
        println!("\n[TEST 5] Full try_semantic_nav_with_url on example.com");
        let _ = engine.navigate("https://example.com").await;
        let update = |step: u32, desc: &str, obs: &str, done: bool| {
            println!("  Step {step}: {desc} [{obs}] done={done}");
        };
        let result = hydra_semantic_nav::try_semantic_nav_with_url(
            &mut engine, "click more information", "https://example.com", &update
        ).await;
        match result {
            hydra_semantic_nav::NavResult::Success => println!("  PASS: Semantic nav succeeded"),
            hydra_semantic_nav::NavResult::Unparseable(r) => println!("  INFO: Fell back to vision — {r}"),
        }

        // Test 6: Constitution cache
        println!("\n[TEST 6] Constitution cache");
        let mut cache = hydra_semantic_nav::constitution_cache::ConstitutionCache::new();
        let cached = cache.check("https://example.com");
        println!("  Cache hit for example.com: {}", cached.is_some());

        engine.close().await;
        println!("\n=== Done ===");
    });
}
