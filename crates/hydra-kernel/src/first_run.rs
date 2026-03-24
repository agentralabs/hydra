//! First-run wizard — interactive setup on first boot.
//! Detects no ~/.hydra directory = first time running Hydra.
//! Creates directories, prompts for LLM provider + API key, writes .env.

use std::io::{self, Write};
use std::path::PathBuf;

/// Check if this is the first run (no ~/.hydra directory exists).
pub fn is_first_run() -> bool {
    let hydra_dir = hydra_home();
    !hydra_dir.exists()
}

/// Run the first-run wizard. Returns true if setup completed.
pub fn run_wizard() -> bool {
    println!("\n  Welcome to Hydra.\n");
    println!("  This is your first time running Hydra. Let's set up.\n");

    // Step 1: Choose LLM provider
    let provider = prompt_choice(
        "  LLM provider",
        &["anthropic", "openai", "ollama", "gemini"],
        "anthropic",
    );

    // Step 2: API key (skip for ollama)
    let api_key = if provider == "ollama" {
        println!("  Ollama uses local models — no API key needed.");
        String::new()
    } else {
        let key_name = match provider.as_str() {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            "gemini" => "GOOGLE_API_KEY",
            _ => "API_KEY",
        };
        prompt_input(&format!("  {key_name}"))
    };

    // Step 3: Create directory structure
    let home = hydra_home();
    let dirs = [
        home.join("data"),
        home.join("backups"),
        home.join("logs"),
    ];

    for dir in &dirs {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("  Error creating {}: {e}", dir.display());
            return false;
        }
    }

    // Step 4: Write .env file
    let env_path = home.join(".env");
    let mut env_content = format!("HYDRA_LLM_PROVIDER={provider}\n");
    if !api_key.is_empty() {
        let key_name = match provider.as_str() {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            "gemini" => "GOOGLE_API_KEY",
            _ => "API_KEY",
        };
        env_content.push_str(&format!("{key_name}={api_key}\n"));
    }
    if let Err(e) = std::fs::write(&env_path, &env_content) {
        eprintln!("  Error writing .env: {e}");
        return false;
    }

    println!("\n  Setup complete!");
    println!("  Data directory: {}", home.display());
    println!("  Provider: {provider}");
    println!("  Config: {}\n", env_path.display());
    println!("  You can change settings anytime with /settings in the TUI.");
    println!("  Or set HYDRA_* environment variables.\n");

    true
}

fn hydra_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hydra")
}

fn prompt_choice(prompt: &str, options: &[&str], default: &str) -> String {
    let options_str = options.join("/");
    print!("{prompt} [{options_str}] (default: {default}): ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        if options.contains(&trimmed.as_str()) {
            return trimmed;
        }
    }
    default.to_string()
}

fn prompt_input(prompt: &str) -> String {
    print!("{prompt}: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        return input.trim().to_string();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydra_home_under_user_dir() {
        let home = hydra_home();
        assert!(home.to_string_lossy().contains(".hydra"));
    }

    #[test]
    fn first_run_check_works() {
        // If ~/.hydra exists, it's not first run
        // Just verify the function doesn't panic
        let _ = is_first_run();
    }
}
