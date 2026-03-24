//! Task Conductor — types, decomposer, and DAG validation.
//! The DAG executor and step router are in conductor_exec.rs.

use std::collections::HashMap;
use std::path::PathBuf;

// ── Types ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Step {
    pub id: usize,
    pub step_type: StepType,
    pub description: String,
    pub depends_on: Vec<usize>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StepType {
    Shell { command: String, long_running: bool },
    CodeGen { description: String, target_path: String, language: String },
    BrowserNavigate { url: String },
    BrowserInteract { goal: String },
    DesktopAction { goal: String },
    FileWrite { path: String, content: String },
    FileRead { path: String },
    ApiCall { method: String, url: String, body: Option<String> },
    Wait { condition: WaitCondition },
    Verify { method: VerifyMethod },
    /// Execute a command on a remote machine via SSH (Session 24).
    Remote { machine: String, command: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WaitCondition {
    ProcessReady { port: u16 },
    FileExists { path: String },
    Duration { ms: u64 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum VerifyMethod {
    HttpStatus { url: String, expect: u16 },
    FileContains { path: String, pattern: String },
    CommandSuccess { command: String },
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_id: usize,
    pub success: bool,
    pub output: String,
    pub artifacts: Vec<String>,
    pub duration_ms: u64,
}

pub struct TaskContext {
    pub goal: String,
    pub steps: Vec<Step>,
    pub results: Vec<StepResult>,
    pub working_dir: PathBuf,
    pub env_vars: HashMap<String, String>,
    pub decomposition_depth: u8,
    pub cancelled: bool,
}

#[derive(Debug)]
pub enum ConductorResult {
    Complete { results: Vec<StepResult> },
    StepFailed { step_id: usize, error: String },
    EmptyPlan,
    CyclicDag,
    Cancelled,
    Error(String),
}

pub const SHELL_TIMEOUT_MS: u64 = 60_000;

// ── Decomposer ──

pub fn decompose(goal: &str, genome: &hydra_genome::GenomeStore) -> Vec<Step> {
    // O4: Check operational skills FIRST (zero LLM tokens, TOML-defined action plans)
    let all_ops = hydra_skills::operations::load_all_operations();
    if let Some(op) = hydra_skills::operations::match_operation(goal, &all_ops) {
        match hydra_skills::operations::extract_params(goal, &op.params) {
            Ok(params) => {
                eprintln!("hydra-conductor: operational skill '{}' matched (conf={:.2})",
                    op.name, op.confidence);
                return from_operation(op, &params);
            }
            Err(missing) => {
                eprintln!("hydra-conductor: skill '{}' matched but missing params: {:?}",
                    op.name, missing);
            }
        }
    }
    // O6: Check workflow templates (multi-app patterns)
    if let Some(steps) = crate::worker::expand_workflow(goal) {
        eprintln!("hydra-conductor: workflow template matched ({} steps)", steps.len());
        return steps;
    }
    // Genome approach (semantic similarity)
    let similar = genome.query(goal);
    if let Some(entry) = similar.first() {
        if entry.effective_confidence() > 0.7 && !entry.approach.steps.is_empty() {
            eprintln!("hydra-conductor: genome approach (conf={:.2})", entry.effective_confidence());
            return steps_from_genome(entry);
        }
    }
    // LLM micro-call fallback
    if let Some(steps) = try_llm_decompose(goal) { return steps; }
    // Never execute raw user input as a shell command — return empty so the LLM handles it
    eprintln!("hydra-conductor: no genome match and LLM decompose failed, deferring to LLM");
    vec![]
}

fn steps_from_genome(entry: &hydra_genome::GenomeEntry) -> Vec<Step> {
    entry.approach.steps.iter().enumerate().map(|(i, desc)| {
        Step { id: i, step_type: infer_step_type(desc), description: desc.clone(),
            depends_on: if i > 0 { vec![i - 1] } else { vec![] }, timeout_ms: SHELL_TIMEOUT_MS }
    }).collect()
}

fn infer_step_type(desc: &str) -> StepType {
    let lower = desc.to_lowercase();
    if lower.starts_with("npm ") || lower.starts_with("npx ") || lower.starts_with("pip ")
        || lower.starts_with("cargo ") || lower.starts_with("git ") || lower.contains("&&") {
        StepType::Shell { command: desc.into(), long_running: lower.contains("install") || lower.contains("build") }
    } else if lower.contains("navigate") || lower.contains("open http") {
        StepType::BrowserNavigate { url: desc.into() }
    } else if lower.contains("write ") || lower.contains("create file") {
        StepType::FileWrite { path: String::new(), content: String::new() }
    } else {
        // Natural language descriptions must NOT become shell commands.
        // Return a FileRead no-op so the step is skipped safely.
        eprintln!("hydra-conductor: skipping non-executable step: {desc}");
        StepType::FileRead { path: String::new() }
    }
}

fn try_llm_decompose(goal: &str) -> Option<Vec<Step>> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001", "max_tokens": 512,
        "messages": [{"role": "user", "content": format!(
            "Decompose this task into executable steps. Return ONLY a JSON array.\n\
             Each step: {{\"type\": \"shell|code_gen|browser|file_write|wait|verify\", \"command\": \"...\", \"desc\": \"...\"}}\n\
             Task: {goal}"
        )}]
    });
    let resp = client.post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key).header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json").json(&body).send().ok()?;
    let parsed: serde_json::Value = resp.json().ok()?;
    let text = parsed.get("content")?.as_array()?.first()?.get("text")?.as_str()?;
    parse_llm_steps(text)
}

fn parse_llm_steps(text: &str) -> Option<Vec<Step>> {
    let start = text.find('[')?;
    let end = text.rfind(']')? + 1;
    let arr: Vec<serde_json::Value> = serde_json::from_str(&text[start..end]).ok()?;
    let steps: Vec<Step> = arr.iter().enumerate().filter_map(|(i, v)| {
        let desc = v.get("desc").or(v.get("description")).and_then(|d| d.as_str())?.to_string();
        let cmd = v.get("command").and_then(|c| c.as_str()).unwrap_or("").to_string();
        let step_type = match v.get("type").and_then(|t| t.as_str()).unwrap_or("shell") {
            "shell" => StepType::Shell { command: cmd, long_running: false },
            "code_gen" => StepType::CodeGen { description: desc.clone(), target_path: cmd, language: "typescript".into() },
            "browser" => StepType::BrowserNavigate { url: cmd },
            "file_write" => StepType::FileWrite { path: cmd, content: String::new() },
            "wait" => StepType::Wait { condition: WaitCondition::Duration { ms: 2000 } },
            "verify" => StepType::Verify { method: VerifyMethod::CommandSuccess { command: cmd } },
            _ => StepType::Shell { command: cmd, long_running: false },
        };
        Some(Step { id: i, step_type, description: desc,
            depends_on: if i > 0 { vec![i - 1] } else { vec![] }, timeout_ms: SHELL_TIMEOUT_MS })
    }).collect();
    if steps.is_empty() { None } else { Some(steps) }
}

// ── Operational Skills Bridge ──

/// Convert a skill Operation into conductor Steps with parameters substituted.
pub fn from_operation(
    op: &hydra_skills::operations::Operation,
    params: &std::collections::HashMap<String, String>,
) -> Vec<Step> {
    op.steps.iter().enumerate().map(|(i, step)| {
        let sub = |s: &str| hydra_skills::operations::substitute(s, params, step.step_type == "shell");
        let desc = sub(step.description.as_deref().unwrap_or(&step.step_type));
        let step_type = match step.step_type.as_str() {
            "shell" => StepType::Shell { command: sub(step.command.as_deref().unwrap_or("")), long_running: step.long_running },
            "code_gen" => StepType::CodeGen {
                description: sub(step.prompt.as_deref().unwrap_or(&desc)),
                target_path: sub(step.target.as_deref().unwrap_or("")),
                language: "typescript".into(),
            },
            "browser" => match &step.url {
                Some(url) => StepType::BrowserNavigate { url: sub(url) },
                None => StepType::BrowserInteract { goal: sub(step.goal.as_deref().unwrap_or(&desc)) },
            },
            "verify" => match step.method.as_deref().unwrap_or("command_success") {
                "http_status" => StepType::Verify { method: VerifyMethod::HttpStatus {
                    url: sub(step.url.as_deref().unwrap_or("")),
                    expect: step.expect.as_ref().and_then(|e| e.as_u64()).unwrap_or(200) as u16,
                }},
                "file_exists" | "file_contains" => StepType::Verify { method: VerifyMethod::FileContains {
                    path: sub(step.path.as_deref().unwrap_or("")), pattern: String::new(),
                }},
                _ => StepType::Verify { method: VerifyMethod::CommandSuccess {
                    command: sub(step.command.as_deref().unwrap_or("true")),
                }},
            },
            "wait" => match step.port {
                Some(port) => StepType::Wait { condition: WaitCondition::ProcessReady { port } },
                None => StepType::Wait { condition: WaitCondition::Duration { ms: step.timeout_ms.unwrap_or(2000) } },
            },
            "file" => StepType::FileWrite {
                path: sub(step.path.as_deref().unwrap_or("")),
                content: sub(step.template.as_deref().unwrap_or("")),
            },
            "desktop" => StepType::DesktopAction { goal: sub(step.goal.as_deref().unwrap_or(&desc)) },
            _ => StepType::Shell { command: sub(step.command.as_deref().unwrap_or("echo unknown")), long_running: false },
        };
        Step { id: i, step_type, description: desc,
            depends_on: if i > 0 { vec![i - 1] } else { vec![] },
            timeout_ms: step.timeout_ms.unwrap_or(SHELL_TIMEOUT_MS) }
    }).collect()
}

// ── DAG Validation ──

pub fn validate_dag(steps: &[Step]) -> Result<(), ConductorResult> {
    if steps.is_empty() { return Err(ConductorResult::EmptyPlan); }
    let mut in_degree: HashMap<usize, usize> = steps.iter().map(|s| (s.id, 0)).collect();
    for s in steps { for &d in &s.depends_on { *in_degree.entry(s.id).or_insert(0) += 1; let _ = in_degree.entry(d).or_insert(0); } }
    let mut queue: Vec<usize> = in_degree.iter().filter(|(_, v)| **v == 0).map(|(k, _)| *k).collect();
    let mut visited = 0;
    while let Some(node) = queue.pop() {
        visited += 1;
        for s in steps { if s.depends_on.contains(&node) {
            if let Some(deg) = in_degree.get_mut(&s.id) { *deg -= 1; if *deg == 0 { queue.push(s.id); } }
        }}
    }
    if visited < steps.len() { Err(ConductorResult::CyclicDag) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_dag_returns_empty_plan() {
        assert!(matches!(validate_dag(&[]), Err(ConductorResult::EmptyPlan)));
    }

    #[test]
    fn cyclic_dag_detected() {
        let steps = vec![
            Step { id: 0, step_type: StepType::Shell { command: "a".into(), long_running: false },
                description: "A".into(), depends_on: vec![1], timeout_ms: 1000 },
            Step { id: 1, step_type: StepType::Shell { command: "b".into(), long_running: false },
                description: "B".into(), depends_on: vec![0], timeout_ms: 1000 },
        ];
        assert!(matches!(validate_dag(&steps), Err(ConductorResult::CyclicDag)));
    }

    #[test]
    fn valid_dag_passes() {
        let steps = vec![
            Step { id: 0, step_type: StepType::Shell { command: "a".into(), long_running: false },
                description: "A".into(), depends_on: vec![], timeout_ms: 1000 },
            Step { id: 1, step_type: StepType::Shell { command: "b".into(), long_running: false },
                description: "B".into(), depends_on: vec![0], timeout_ms: 1000 },
        ];
        assert!(validate_dag(&steps).is_ok());
    }

    #[test]
    fn decompose_fallback_returns_empty() {
        let genome = hydra_genome::GenomeStore::new();
        // Without LLM or genome match, decompose returns empty (never raw-shell user input)
        let steps = decompose("can you post on the internet?", &genome);
        assert!(steps.is_empty());
    }

    #[test]
    fn operations_match_and_convert() {
        // Test the full O4 pipeline: parse TOML → match trigger → convert to Steps
        let toml = r#"
        [[operation]]
        name = "test_deploy"
        trigger = "deploy|ship to prod"
        confidence = 0.9

        [[operation.steps]]
        type = "shell"
        command = "docker build -t app ."
        description = "Build Docker image"

        [[operation.steps]]
        type = "shell"
        command = "docker run -d -p 8080:8080 app"
        description = "Run container"

        [[operation.steps]]
        type = "verify"
        method = "http_status"
        url = "http://localhost:8080"
        [operation.steps.expect]
        status = 200
        "#;
        let ops = hydra_skills::operations::parse_operations(toml).unwrap();
        assert_eq!(ops.len(), 1);
        let op = &ops[0];
        // Match trigger
        assert!(hydra_skills::operations::match_operation("deploy to production", &ops).is_some());
        // Convert to conductor steps
        let params = std::collections::HashMap::new();
        let steps = from_operation(op, &params);
        assert_eq!(steps.len(), 3);
        assert!(matches!(steps[0].step_type, StepType::Shell { .. }));
        assert!(matches!(steps[1].step_type, StepType::Shell { .. }));
        assert!(matches!(steps[2].step_type, StepType::Verify { .. }));
        // Dependencies are sequential
        assert!(steps[0].depends_on.is_empty());
        assert_eq!(steps[1].depends_on, vec![0]);
        assert_eq!(steps[2].depends_on, vec![1]);
    }
}
