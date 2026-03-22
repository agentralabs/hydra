//! runner.rs — Runs Hydra as a subprocess for each input.
//! Captures stdout (the response) and stderr (the receipt footer).
//! Never calls the LLM directly — always goes through the Hydra binary.
//! This tests the full pipeline, not just individual crates.
//!
//! Retries on boot lock collision (up to 3 attempts with 5s backoff).

use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct HydraResponse {
    pub input:       String,
    pub output:      String,
    pub receipt:     Option<ReceiptFooter>,
    pub duration_ms: u64,
    pub error:       Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReceiptFooter {
    pub session_id:  String,
    pub path:        String,   // zero-token | llm-short | llm-long
    pub tokens:      u64,
    pub duration_ms: u64,
}

impl ReceiptFooter {
    /// Parse "[a3f8b2c1|llm-short|234tok|1847ms|mw=8]" or "[a3f8b2c1|llm-short|234tok|1847ms]"
    pub fn parse(line: &str) -> Option<Self> {
        let inner = line.trim().trim_start_matches('[').trim_end_matches(']');
        let parts: Vec<&str> = inner.split('|').collect();
        // Accept 4 or 5 parts (with or without mw= suffix)
        if parts.len() < 4 { return None; }
        let tokens = parts[2].trim_end_matches("tok").parse().ok()?;
        let duration_ms = parts[3].trim_end_matches("ms").parse().ok()?;
        Some(Self {
            session_id:  parts[0].to_string(),
            path:        parts[1].to_string(),
            tokens,
            duration_ms,
        })
    }

    pub fn is_zero_token(&self) -> bool {
        self.path == "zero-token"
    }
}

/// Run a single input through the Hydra binary.
/// Retries up to 3 times with 5s backoff on boot lock collision.
pub fn run_hydra(input: &str, api_key: &str) -> HydraResponse {
    let start = Instant::now();

    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return HydraResponse {
                input:       input.to_string(),
                output:      String::new(),
                receipt:     None,
                duration_ms,
                error:       Some(format!("Failed to get cwd: {e}")),
            };
        }
    };

    for attempt in 0..3u32 {
        if attempt > 0 {
            eprintln!(
                "  [retry] boot lock collision, attempt {} (waiting 5s)",
                attempt + 1
            );
            std::thread::sleep(Duration::from_secs(5));
        }

        let output = Command::new("cargo")
            .args([
                "run", "-p", "hydra-kernel", "--bin", "hydra",
                "--quiet", "--", input,
            ])
            .env("ANTHROPIC_API_KEY", api_key)
            .env("HYDRA_LOG", "error")
            .current_dir(&cwd)
            .output();

        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();

                // Boot lock collision: empty stdout + lock error in stderr
                if stdout.trim().is_empty()
                    && stderr.contains("Another Hydra instance")
                {
                    continue; // retry
                }

                // Parse receipt from stderr — last bracket-delimited line
                let receipt = stderr
                    .lines()
                    .filter(|l| l.starts_with('[') && l.ends_with(']'))
                    .next_back()
                    .and_then(ReceiptFooter::parse);

                // Strip header lines from stdout
                let response = stdout
                    .lines()
                    .skip_while(|l| {
                        l.starts_with("Hydra —")
                            || l.starts_with("Provider:")
                            || l.trim() == "---"
                            || l.trim().is_empty()
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string();

                return HydraResponse {
                    input:       input.to_string(),
                    output:      response,
                    receipt,
                    duration_ms,
                    error:       if out.status.success() {
                        None
                    } else {
                        Some(format!("exit code: {}", out.status))
                    },
                };
            }
            Err(e) => {
                return HydraResponse {
                    input:       input.to_string(),
                    output:      String::new(),
                    receipt:     None,
                    duration_ms,
                    error:       Some(e.to_string()),
                };
            }
        }
    }

    // All retries exhausted
    HydraResponse {
        input:       input.to_string(),
        output:      String::new(),
        receipt:     None,
        duration_ms: start.elapsed().as_millis() as u64,
        error:       Some("boot lock collision: all retries exhausted".into()),
    }
}
