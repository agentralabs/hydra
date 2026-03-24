//! Live test for swarm browser.
//! Run: cargo run -p hydra-swarm-browser --bin swarm_test

fn main() {
    println!("=== Hydra Swarm Browser — Live Test ===\n");

    println!("[TEST 1] Swarm: 'learn about rust ownership'");
    let start = std::time::Instant::now();
    match hydra_swarm_browser::execute_swarm_blocking("learn about rust ownership") {
        Ok(text) => {
            let ms = start.elapsed().as_millis();
            let lines = text.lines().count();
            println!("  PASS: {} lines, {}ms", lines, ms);
            for line in text.lines().take(8) { println!("  | {line}"); }
            println!("  ...\n");
        }
        Err(e) => println!("  FAIL: {e}\n"),
    }

    println!("[TEST 2] Decomposer with YouTube URL");
    let goal = hydra_swarm_browser::SwarmGoal::new(
        "watch https://youtube.com/watch?v=dQw4w9WgXcQ and learn", 3
    );
    let tasks = hydra_swarm_browser::decomposer::decompose(&goal);
    let has_yt = tasks.iter().any(|t|
        matches!(t.task_type, hydra_swarm_browser::types::SwarmTaskType::YouTubeTranscript { .. })
    );
    println!("  Tasks: {}, YouTube task: {}", tasks.len(), has_yt);
    for task in &tasks {
        println!("    - {:?}: {}", task.task_type, task.query);
    }
    if has_yt { println!("  PASS\n"); } else { println!("  FAIL: no YouTube task\n"); }

    println!("[TEST 3] Merger consensus");
    let results = vec![
        make_result("Rust ownership ensures memory safety without GC", 0.8),
        make_result("The ownership system in Rust guarantees memory safety", 0.7),
        make_result("Rust achieves memory safety through its ownership model", 0.9),
    ];
    let consensus = hydra_swarm_browser::merger::check_consensus(&results);
    println!("  Consensus reached: {consensus}");
    if consensus { println!("  PASS\n"); } else { println!("  FAIL\n"); }

    println!("=== Done ===");
}

fn make_result(content: &str, confidence: f64) -> hydra_swarm_browser::WorkerResult {
    hydra_swarm_browser::WorkerResult {
        task_id: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        content: content.into(),
        source_url: "https://example.com".into(),
        confidence,
        duration_ms: 100,
        error: None,
    }
}
