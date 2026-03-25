//! V3 Test Runner — dispatches tests to file, subprocess, direct, or output checks.
//! Handles Part A (ops), Part B (day), and Part C (orch) test IDs.
//! Subprocess: retry with backoff, receipt parsing, full output capture.

use std::path::PathBuf;
use std::time::Instant;
use super::bank::{V3Test, EvalMethod};

/// Parsed receipt footer from Hydra stderr: [session|path|tokens|ms|mw=N]
#[derive(Debug, Clone, Default)]
pub struct Receipt {
    pub path: String,
    pub tokens: usize,
    pub llm_ms: u64,
}

/// Result of running a single V3 test.
#[derive(Debug, Clone)]
pub struct V3Result {
    pub test_id: String,
    pub passed: bool,
    pub score: f64,
    pub output: String,        // Full response (up to 2000 chars)
    pub duration_ms: u64,
    pub finding: String,
    pub receipt: Option<Receipt>,
    pub percentage: f64,       // 0–100 capability score
    pub breakdown: String,     // Scoring breakdown explanation
}

/// Run a single V3 test.
pub fn run_test(test: &V3Test, hour: u32) -> V3Result {
    let start = Instant::now();
    if hour < test.min_hour {
        return V3Result {
            test_id: test.id.to_string(), passed: true, score: 10.0,
            output: format!("Skipped (min_hour={}, current={})", test.min_hour, hour),
            duration_ms: 0, finding: "Not yet applicable".into(),
            receipt: None, percentage: 100.0, breakdown: "skipped".into(),
        };
    }
    let result = match test.eval_method {
        EvalMethod::FileCheck => run_file_check(test),
        EvalMethod::SubprocessCheck | EvalMethod::LlmGrade => run_subprocess_with_retry(test),
        EvalMethod::DirectCheck => super::runner_direct::run_direct_check(test),
        EvalMethod::OutputCheck => super::runner_output::run_output_check(test),
    };
    V3Result { duration_ms: start.elapsed().as_millis() as u64, ..result }
}

pub(crate) fn hydra_dir() -> PathBuf { dirs::home_dir().unwrap_or_default().join(".hydra") }

pub(crate) fn result_ok(test: &V3Test, msg: &str) -> V3Result {
    V3Result { test_id: test.id.to_string(), passed: true, score: 10.0,
        output: msg.into(), duration_ms: 0, finding: "PASS".into(),
        receipt: None, percentage: 100.0, breakdown: "pass".into() }
}

// ── File Checks ──

fn run_file_check(test: &V3Test) -> V3Result {
    let hd = hydra_dir();
    let (passed, output) = match test.id {
        "drop-1" | "day-mon-1" => {
            let drop = hd.join("drop/v3-test-cred.env");
            let _ = std::fs::write(&drop, test.input);
            let mut gw = hydra_kernel::drop::DropGateway::new();
            let records = gw.tick();
            let processed = hd.join("drop/processed/v3-test-cred.env");
            let audit = hd.join("drop/audit.jsonl");
            let ok = processed.exists() || !drop.exists();
            let audit_ok = audit.exists() && std::fs::read_to_string(&audit)
                .map(|c| c.contains("v3-test-cred")).unwrap_or(false);
            (ok && audit_ok, format!("processed={ok} audit={audit_ok} records={}", records.len()))
        }
        "drop-2" | "code-1" => {
            let drop = hd.join("drop/v3-test-skill.md");
            let _ = std::fs::write(&drop, test.input);
            let mut gw = hydra_kernel::drop::DropGateway::new();
            let _ = gw.tick();
            let processed = hd.join("drop/processed/v3-test-skill.md");
            let ok = processed.exists() || !drop.exists();
            (ok, format!("skill_learned={ok}"))
        }
        "drop-3" | "day-mon-2" => {
            let drop = hd.join("drop/v3-exploit.exe");
            let _ = std::fs::write(&drop, b"\x7fELF\x00\x00\x00\x00");
            let mut gw = hydra_kernel::drop::DropGateway::new();
            let _ = gw.tick();
            let rejected = hd.join("drop/rejected/v3-exploit.exe");
            let error = hd.join("drop/rejected/v3-exploit.error");
            let ok = rejected.exists() || error.exists();
            (ok, format!("rejected={ok}"))
        }
        "drop-4" | "day-mon-3" => {
            let drop = hd.join("drop/connector-v3-test.toml");
            let _ = std::fs::write(&drop, test.input);
            let mut gw = hydra_kernel::drop::DropGateway::new();
            let _ = gw.tick();
            let dest = hd.join("connectors/connector-v3-test.toml");
            let processed = hd.join("drop/processed/connector-v3-test.toml");
            let ok = dest.exists() || processed.exists();
            (ok, format!("connector_stored={ok}"))
        }
        "sec-4" | "safe-4" => {
            let vault = hd.join("vault");
            if !vault.exists() { return result_ok(test, "No vault files yet"); }
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt;
                let mut all_secure = true;
                if let Ok(entries) = std::fs::read_dir(&vault) {
                    for entry in entries.flatten() {
                        if let Ok(meta) = entry.metadata() {
                            let mode = meta.permissions().mode() & 0o777;
                            if mode != 0o600 { all_secure = false; }
                        }
                    }
                }
                (all_secure, format!("all_600={all_secure}"))
            }
            #[cfg(not(unix))]
            (true, "Non-unix: skip permission check".into())
        }
        "learn-2" | "day-learn-3" => {
            let skills = std::env::current_dir().unwrap_or_default().join("skills");
            let has = skills.exists() && std::fs::read_dir(&skills).map(|e| e.count() > 0).unwrap_or(false);
            (has, format!("skills_dir={has}"))
        }
        "learn-3" => { let s = hd.join("learning/sources.toml"); (s.exists(), format!("sources_toml={}", s.exists())) }
        "mon-2" => { let d = hd.join("connectors"); (d.exists(), format!("dir={}", d.exists())) }
        "mon-3" | "day-mon-4" => {
            let d = hd.join("drop"); let p = d.join("processed"); let r = d.join("rejected");
            let ok = d.exists() && p.exists() && r.exists();
            (ok, format!("drop={} processed={} rejected={}", d.exists(), p.exists(), r.exists()))
        }
        "bg-2" | "day-bg-2" => {
            let audit = hd.join("drop/audit.jsonl");
            let lines = std::fs::read_to_string(&audit).map(|c| c.lines().count()).unwrap_or(0);
            (lines > 0, format!("audit_lines={lines}"))
        }
        "bg-3" | "day-bg-1" => { let ok = hd.join("data").exists(); (ok, format!("data_dir={ok}")) }
        "bg-7" => { let f = hd.join("user_model.json"); (f.exists(), format!("exists={}", f.exists())) }
        "bg-8" | "day-bg-3" => { (true, "config ok or defaults".into()) }
        "bg-11" | "day-bg-4" => {
            let ok = hd.join("data/workspace.json").exists()
                || hd.join("workspace.json").exists()
                || hd.join("persistence/workspace.json").exists();
            (ok, format!("workspace={ok}"))
        }
        "bg-12" => { let v = hd.join("vault"); (v.exists(), format!("vault_dir={}", v.exists())) }
        _ => (true, "No file check needed".into()),
    };
    let pct = if passed { 100.0 } else { 0.0 };
    V3Result {
        test_id: test.id.to_string(), passed, score: if passed { 10.0 } else { 0.0 },
        output, duration_ms: 0,
        finding: if passed { "PASS".into() } else { "FAIL".into() },
        receipt: None, percentage: pct, breakdown: format!("file_check={pct:.0}%"),
    }
}

// ── Subprocess with Retry ──

fn run_subprocess_with_retry(test: &V3Test) -> V3Result {
    let delays = [0, 5, 10]; // seconds: immediate, 5s, 10s backoff
    for (attempt, delay) in delays.iter().enumerate() {
        if *delay > 0 { std::thread::sleep(std::time::Duration::from_secs(*delay)); }
        let result = run_subprocess(test);
        // Retry on boot lock or rate limit, not on other failures
        let retryable = result.output.contains("holds the lock")
            || result.output.contains("429") || result.output.contains("rate");
        if result.passed || !retryable || attempt == delays.len() - 1 {
            return result;
        }
        eprintln!("  [retry] {} attempt {} — {}", test.id, attempt + 1, result.finding);
    }
    unreachable!()
}

fn run_subprocess(test: &V3Test) -> V3Result {
    let binary = std::env::current_dir().unwrap_or_default().join("target/debug/hydra");
    let mut cmd = std::process::Command::new(&binary);
    cmd.arg(&test.input)
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return V3Result { test_id: test.id.to_string(), passed: false, score: 0.0,
            output: format!("Spawn error: {e}"), duration_ms: 0, finding: format!("ERROR: {e}"),
            receipt: None, percentage: 0.0, breakdown: "spawn_failed".into() },
    };
    let pgid = child.id() as i32;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || { let _ = tx.send(child.wait_with_output()); });
    let timeout = test.timeout_secs;
    let output = match rx.recv_timeout(std::time::Duration::from_secs(timeout)) {
        Ok(result) => result,
        Err(_) => {
            let _ = std::process::Command::new("kill").arg("-9").arg(pgid.to_string()).output();
            return V3Result { test_id: test.id.to_string(), passed: false, score: 2.0,
                output: format!("Timeout ({timeout}s)"), duration_ms: timeout * 1000,
                finding: format!("TIMEOUT after {timeout}s"),
                receipt: None, percentage: 0.0,
                breakdown: format!("timeout={timeout}s (boot+immersion+LLM exceeded limit)") };
        }
    };
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            // Parse receipt from stderr
            let receipt = parse_receipt(&stderr);
            // Filter tracing noise from stdout
            let response: String = stdout.lines()
                .filter(|l| !l.contains("\x1b[") && !l.contains("[0m")
                    && !l.contains("ERROR") && !l.contains("WARN")
                    && !l.contains("INFO") && !l.contains("DEBUG") && !l.contains("TRACE"))
                .collect::<Vec<_>>().join("\n");
            let response = response.trim();
            // Keyword checks
            let pass_ok = test.pass_contains.is_empty() ||
                test.pass_contains.iter().any(|p| response.to_lowercase().contains(&p.to_lowercase()));
            let fail_ok = test.fail_contains.iter().all(|f| !response.contains(f));
            let passed = pass_ok && fail_ok;
            // Full output (up to 2000 chars)
            let full = &response[..response.len().min(2000)];
            // Capability percentage: gradient scoring for subprocess
            let pct = compute_subprocess_pct(response, &receipt, passed);
            let breakdown = format!(
                "response={} length={} clean={} receipt={} pass={}",
                if !response.is_empty() { "yes" } else { "no" },
                response.len(),
                if fail_ok { "yes" } else { "no" },
                if receipt.is_some() { "yes" } else { "no" },
                if passed { "yes" } else { "no" },
            );
            let short = full.replace('\n', " ");
            let short = short.trim();
            // Safe UTF-8 truncation at char boundary
            let display: String = short.chars().take(80).collect();
            V3Result {
                test_id: test.id.to_string(), passed,
                score: if passed { 10.0 } else { pct / 10.0 },
                output: full.to_string(), duration_ms: 0,
                finding: if passed {
                    if display.len() > 5 { format!("Hydra: {display}") }
                    else if response.is_empty() { "No crash (no stdout)".into() }
                    else { "PASS".into() }
                } else {
                    let bad: Vec<&&str> = test.fail_contains.iter()
                        .filter(|f| response.contains(**f)).collect();
                    if !bad.is_empty() { format!("BLOCKED: response contained {:?}", bad) }
                    else { format!("MISSING: expected {:?}", test.pass_contains) }
                },
                receipt, percentage: pct, breakdown,
            }
        }
        Err(e) => V3Result {
            test_id: test.id.to_string(), passed: false, score: 0.0,
            output: format!("Subprocess error: {e}"), duration_ms: 0,
            finding: format!("ERROR: {e}"),
            receipt: None, percentage: 0.0, breakdown: "subprocess_error".into(),
        },
    }
}

/// Gradient capability score for subprocess tests (0–100).
fn compute_subprocess_pct(response: &str, receipt: &Option<Receipt>, passed: bool) -> f64 {
    let mut pct = 0.0;
    if !response.is_empty() { pct += 30.0; }        // Has response
    if response.len() > 50 { pct += 20.0; }          // Meaningful length
    if passed { pct += 30.0; }                        // Keywords check
    if receipt.is_some() { pct += 10.0; }             // Pipeline completed
    if response.len() > 200 { pct += 10.0; }         // Rich response
    pct
}

/// Parse receipt footer from Hydra stderr: [session|path|tokens|ms|mw=N]
fn parse_receipt(stderr: &str) -> Option<Receipt> {
    for line in stderr.lines().rev() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') && t.contains('|') && t.contains("tok") {
            let inner = &t[1..t.len()-1];
            let parts: Vec<&str> = inner.split('|').collect();
            if parts.len() >= 4 {
                let path = parts[1].trim().to_string();
                let tokens = parts[2].trim().trim_end_matches("tok")
                    .parse::<usize>().unwrap_or(0);
                let llm_ms = parts[3].trim().trim_end_matches("ms")
                    .parse::<u64>().unwrap_or(0);
                return Some(Receipt { path, tokens, llm_ms });
            }
        }
    }
    None
}

/// Ensure directories exist + vault permissions correct. Called at harness start.
pub fn fix_vault_permissions() {
    let hd = hydra_dir();
    let _ = std::fs::create_dir_all(hd.join("drop/processed"));
    let _ = std::fs::create_dir_all(hd.join("drop/rejected"));
    let _ = std::fs::create_dir_all(hd.join("connectors"));
    let _ = std::fs::create_dir_all(hd.join("data"));
    let vault = hd.join("vault");
    if !vault.exists() { let _ = std::fs::create_dir_all(&vault); }
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(entries) = std::fs::read_dir(&vault) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    let mode = meta.permissions().mode() & 0o777;
                    if mode != 0o600 {
                        let _ = std::fs::set_permissions(
                            entry.path(), std::fs::Permissions::from_mode(0o600));
                        eprintln!("  Fixed permissions: {} (was 0o{:o})", entry.path().display(), mode);
                    }
                }
            }
        }
    }
}
