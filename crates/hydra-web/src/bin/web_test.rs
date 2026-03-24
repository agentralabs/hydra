//! Live test for the hydra-web engine.
//! Run with: cargo run -p hydra-web --bin web_test

fn main() {
    let mut orch = hydra_web::SearchOrchestrator::new();

    println!("=== Hydra Web Engine — Live Test ===\n");

    // Test 1: Fresh search
    println!("[TEST 1] Fresh search: \"rust ownership model\"");
    let start = std::time::Instant::now();
    match orch.search_blocking("rust ownership model") {
        Ok(text) => {
            let ms = start.elapsed().as_millis();
            let lines = text.lines().count();
            println!("  PASS: {} lines, {}ms", lines, ms);
            // Print first 5 lines
            for line in text.lines().take(5) { println!("  | {line}"); }
            println!("  ...");
        }
        Err(e) => println!("  FAIL: {e}"),
    }

    println!();

    // Test 2: Semantic cache hit
    println!("[TEST 2] Semantic cache: \"how does rust ownership work\"");
    let start = std::time::Instant::now();
    match orch.search_blocking("how does rust ownership work") {
        Ok(text) => {
            let ms = start.elapsed().as_millis();
            let cached = text.contains("(from cache)");
            if cached {
                println!("  PASS: SEMANTIC CACHE HIT in {}ms", ms);
            } else {
                println!("  INFO: Not cached ({}ms) — terms may not overlap enough", ms);
            }
        }
        Err(e) => println!("  FAIL: {e}"),
    }

    println!();

    // Test 3: Code-focused search
    println!("[TEST 3] Code search: \"tokio async runtime tutorial\"");
    let start = std::time::Instant::now();
    match orch.search_blocking("tokio async runtime tutorial") {
        Ok(text) => {
            let ms = start.elapsed().as_millis();
            let has_github = text.to_lowercase().contains("github");
            let has_stackoverflow = text.to_lowercase().contains("stackoverflow") || text.to_lowercase().contains("stack overflow");
            println!("  PASS: {}ms, GitHub: {}, StackOverflow: {}", ms, has_github, has_stackoverflow);
        }
        Err(e) => println!("  FAIL: {e}"),
    }

    println!();

    // Test 4: Wikipedia-heavy search
    println!("[TEST 4] Academic search: \"quantum entanglement\"");
    let start = std::time::Instant::now();
    match orch.search_blocking("quantum entanglement") {
        Ok(text) => {
            let ms = start.elapsed().as_millis();
            let has_wiki = text.to_lowercase().contains("wikipedia");
            println!("  PASS: {}ms, Wikipedia: {}", ms, has_wiki);
        }
        Err(e) => println!("  FAIL: {e}"),
    }

    println!("\n=== Done ===");
}
