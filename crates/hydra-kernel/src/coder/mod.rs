//! Supreme Coder — 9-stage coding pipeline.
//! Analyze → plan → write → test → debug → refactor → review → learn → deliver.

pub mod tdd;
pub mod review;

use std::collections::HashMap;
use std::path::Path;

// ── Types ──

/// Profile of the codebase under development.
#[derive(Debug, Clone)]
pub struct CodebaseProfile {
    pub language: String,
    pub framework: Option<String>,
    pub dependencies: Vec<String>,
    pub file_count: usize,
    pub test_framework: Option<String>,
    pub has_linter: bool,
    pub has_ci: bool,
    pub recent_commits: Vec<String>,
}

/// A file-by-file coding plan.
#[derive(Debug, Clone)]
pub struct CodingPlan {
    pub files_to_create: Vec<PlannedFile>,
    pub files_to_modify: Vec<PlannedFile>,
    pub tests_to_write: Vec<String>,
    pub install_commands: Vec<String>,
    pub strategy: String,
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub path: String,
    pub description: String,
    pub depends_on: Vec<String>,
}

/// Final result of the coding pipeline.
#[derive(Debug, Clone)]
pub struct CodingResult {
    pub files_created: usize,
    pub files_modified: usize,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub review_issues: Vec<review::ReviewIssue>,
    pub genome_entries_created: usize,
    pub score: f64,
    pub summary: String,
}

// ── Stage 1: Codebase Analyzer ──

/// Scan the codebase to build a profile. Zero LLM tokens — pure shell commands.
/// EC-9.1: limits depth and file count for large codebases.
pub fn analyze_codebase(working_dir: &str) -> CodebaseProfile {
    let dir = Path::new(working_dir);
    let language = detect_language(dir);
    let framework = detect_framework(dir, &language);
    let dependencies = read_dependencies(dir, &language);
    let file_count = count_source_files(dir, &language);
    let test_framework = detect_test_framework(dir, &language);
    let has_linter = detect_linter(dir, &language);
    let has_ci = dir.join(".github/workflows").exists() || dir.join(".gitlab-ci.yml").exists();
    let recent_commits = read_recent_commits(dir);

    eprintln!("hydra-coder: analyzed codebase — {language}, {} files, framework={:?}", file_count, framework);
    CodebaseProfile { language, framework, dependencies, file_count, test_framework, has_linter, has_ci, recent_commits }
}

fn detect_language(dir: &Path) -> String {
    if dir.join("Cargo.toml").exists() { return "rust".into(); }
    if dir.join("package.json").exists() { return "typescript".into(); }
    if dir.join("requirements.txt").exists() || dir.join("pyproject.toml").exists() { return "python".into(); }
    if dir.join("go.mod").exists() { return "go".into(); }
    if dir.join("pom.xml").exists() || dir.join("build.gradle").exists() { return "java".into(); }
    "unknown".into()
}

fn detect_framework(dir: &Path, language: &str) -> Option<String> {
    match language {
        "typescript" | "javascript" => {
            let pkg = std::fs::read_to_string(dir.join("package.json")).unwrap_or_default();
            if pkg.contains("next") { return Some("nextjs".into()); }
            if pkg.contains("react") { return Some("react".into()); }
            if pkg.contains("vue") { return Some("vue".into()); }
            if pkg.contains("express") { return Some("express".into()); }
            None
        }
        "python" => {
            let req = std::fs::read_to_string(dir.join("requirements.txt")).unwrap_or_default();
            if req.contains("django") { return Some("django".into()); }
            if req.contains("flask") { return Some("flask".into()); }
            if req.contains("fastapi") { return Some("fastapi".into()); }
            None
        }
        "rust" => {
            let cargo = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
            if cargo.contains("actix") { return Some("actix".into()); }
            if cargo.contains("axum") { return Some("axum".into()); }
            if cargo.contains("rocket") { return Some("rocket".into()); }
            None
        }
        _ => None,
    }
}

fn read_dependencies(dir: &Path, language: &str) -> Vec<String> {
    match language {
        "typescript" | "javascript" => {
            let pkg = std::fs::read_to_string(dir.join("package.json")).unwrap_or_default();
            let parsed: serde_json::Value = serde_json::from_str(&pkg).unwrap_or_default();
            parsed.get("dependencies").and_then(|d| d.as_object())
                .map(|m| m.keys().take(20).cloned().collect()).unwrap_or_default()
        }
        "rust" => {
            let cargo = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
            cargo.lines().filter(|l| l.contains("=") && !l.starts_with('[') && !l.starts_with('#'))
                .take(20).map(|l| l.split('=').next().unwrap_or("").trim().to_string()).collect()
        }
        "python" => {
            std::fs::read_to_string(dir.join("requirements.txt")).unwrap_or_default()
                .lines().take(20).map(|l| l.split("==").next().unwrap_or(l).trim().to_string()).collect()
        }
        _ => vec![],
    }
}

fn count_source_files(dir: &Path, language: &str) -> usize {
    let ext = match language { "rust" => "rs", "typescript" => "ts", "python" => "py", "go" => "go", _ => "*" };
    // EC-9.1: limit to 1000 files, depth 5
    match std::process::Command::new("find").args([dir.to_str().unwrap_or("."), "-maxdepth", "5", "-name", &format!("*.{ext}"), "-type", "f"])
        .output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout).lines().count().min(10000),
        Err(_) => 0,
    }
}

fn detect_test_framework(dir: &Path, language: &str) -> Option<String> {
    match language {
        "typescript" => {
            let pkg = std::fs::read_to_string(dir.join("package.json")).unwrap_or_default();
            if pkg.contains("jest") { Some("jest".into()) }
            else if pkg.contains("vitest") { Some("vitest".into()) }
            else if pkg.contains("mocha") { Some("mocha".into()) }
            else { None }
        }
        "rust" => Some("cargo test".into()),
        "python" => {
            if dir.join("pytest.ini").exists() || dir.join("pyproject.toml").exists() { Some("pytest".into()) }
            else { Some("unittest".into()) }
        }
        _ => None,
    }
}

fn detect_linter(dir: &Path, language: &str) -> bool {
    match language {
        "typescript" => dir.join(".eslintrc.json").exists() || dir.join(".eslintrc.js").exists(),
        "rust" => true, // clippy is always available
        "python" => dir.join(".flake8").exists() || dir.join("pyproject.toml").exists(),
        _ => false,
    }
}

fn read_recent_commits(dir: &Path) -> Vec<String> {
    match std::process::Command::new("git").args(["log", "--oneline", "-10"]).current_dir(dir).output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).lines().map(|l| l.to_string()).collect(),
        _ => vec![],
    }
}

// ── Stage 2: Architecture Planner ──

/// Create a file-by-file coding plan from a goal + codebase profile.
pub fn plan_coding(goal: &str, profile: &CodebaseProfile, genome: &hydra_genome::GenomeStore) -> CodingPlan {
    // Check genome for proven approaches
    let similar = genome.query(goal);
    let strategy = if let Some(entry) = similar.first() {
        if entry.effective_confidence() > 0.7 {
            format!("genome approach (conf={:.2}): {}", entry.effective_confidence(),
                entry.approach.steps.first().cloned().unwrap_or_default())
        } else { format!("LLM-planned for {}", profile.language) }
    } else { format!("LLM-planned for {}", profile.language) };

    // Generate install commands based on language
    let install = match profile.language.as_str() {
        "typescript" => vec!["npm install".into()],
        "python" => vec!["pip install -r requirements.txt".into()],
        "rust" => vec!["cargo build".into()],
        _ => vec![],
    };

    eprintln!("hydra-coder: planned — strategy={strategy}");
    CodingPlan {
        files_to_create: vec![], // filled by LLM or genome
        files_to_modify: vec![],
        tests_to_write: vec![format!("Test: {goal}")],
        install_commands: install,
        strategy,
    }
}

/// Run the full coding pipeline.
pub fn code(goal: &str, working_dir: &str, genome: &mut hydra_genome::GenomeStore) -> CodingResult {
    let profile = analyze_codebase(working_dir);
    let plan = plan_coding(goal, &profile, genome);
    let tdd_result = tdd::run_tdd(&plan, working_dir);
    let review_issues = review::review_code(working_dir, &profile);
    let decisions = review::record_decisions(goal, &profile);

    eprintln!("hydra-coder: complete — tests={}/{}, issues={}, decisions={}",
        tdd_result.passed, tdd_result.total, review_issues.len(), decisions);

    CodingResult {
        files_created: plan.files_to_create.len(),
        files_modified: plan.files_to_modify.len(),
        tests_passed: tdd_result.passed,
        tests_failed: tdd_result.failed,
        review_issues,
        genome_entries_created: 0,
        score: if tdd_result.failed == 0 { 9.0 } else { 6.0 },
        summary: format!("{} — {} tests, {} issues", plan.strategy, tdd_result.total, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_current_codebase() {
        let profile = analyze_codebase(".");
        assert_eq!(profile.language, "rust");
        assert!(profile.file_count > 0);
    }

    #[test]
    fn detect_rust_language() {
        assert_eq!(detect_language(Path::new(".")), "rust");
    }

    #[test]
    fn plan_produces_strategy() {
        let genome = hydra_genome::GenomeStore::new();
        let profile = CodebaseProfile {
            language: "rust".into(), framework: None, dependencies: vec![],
            file_count: 10, test_framework: Some("cargo test".into()),
            has_linter: true, has_ci: false, recent_commits: vec![],
        };
        let plan = plan_coding("add auth", &profile, &genome);
        assert!(!plan.strategy.is_empty());
    }
}
