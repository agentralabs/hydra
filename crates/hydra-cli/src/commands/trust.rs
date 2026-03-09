use crate::client::HydraClient;
use crate::colors;
use crate::output;

pub fn execute() {
    output::print_header("Trust & Autonomy");
    println!();

    let client = HydraClient::new();
    match client.get("/api/system/trust") {
        Ok(data) => {
            let score = data["trust_score"].as_f64().unwrap_or(0.0);
            let level = data["autonomy_level"].as_str().unwrap_or("unknown");

            // Trust bar visualization
            let bar_width = 30;
            let filled = (score * bar_width as f64) as usize;
            let empty = bar_width - filled;
            let bar = format!(
                "[{}{}] {:.0}%",
                "\u{2588}".repeat(filled),
                "\u{2591}".repeat(empty),
                score * 100.0
            );

            output::print_kv("Trust score", &colors::bold(&bar));
            output::print_kv("Autonomy level", &colors::bold(level));
            println!();

            // Show level descriptions
            let levels = [
                ("Observer", "0-20%", "Read-only, asks permission for everything"),
                ("Apprentice", "20-40%", "Can write files with approval"),
                ("Assistant", "40-60%", "Autonomous for safe operations"),
                ("Partner", "60-80%", "Full autonomy except destructive ops"),
                ("Autonomous", "80-100%", "Full autonomy including deployments"),
            ];

            for (name, range, desc) in &levels {
                let indicator = if level.to_lowercase().contains(&name.to_lowercase()) {
                    colors::green("\u{25c9}")
                } else {
                    colors::dim("\u{25cb}")
                };
                println!(
                    "  {} {} {} {}",
                    indicator,
                    colors::bold(name),
                    colors::dim(range),
                    colors::dim(desc)
                );
            }
        }
        Err(e) => {
            output::print_error(&format!("Cannot reach server: {}", e));
            output::print_dimmed("Start the server with: hydra serve");
        }
    }
}
