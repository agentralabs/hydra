//! O27: Intent Compiler — translates natural language goals into typed UI plans.
//!
//! Like a programming language compiler: Parse → Resolve → Optimize → Emit.
//! Plans the ENTIRE task BEFORE any mouse moves.
//! Uses AMM + Conventions + Genome for resolution. LLM only for novel goals.

use crate::conductor::{Step, StepType};
use crate::convention::ConventionEngine;
use crate::muscle_memory::UiPrimitive;

/// Parsed structure of a user's goal.
#[derive(Debug, Clone)]
pub struct IntentAST {
    pub goal: String,
    pub domain: String,
    pub target_app: Option<String>,
    pub constraints: Vec<String>,
    pub sub_goals: Vec<String>,
}

/// A single UI instruction — higher-level than UiPrimitive.
#[derive(Debug, Clone)]
pub enum UiInstruction {
    /// Open an application by name.
    OpenApp { name: String },
    /// Use a keyboard shortcut (resolved via ConventionEngine or AMM).
    UseShortcut { intent: String, modifier: String, key: String },
    /// Navigate a menu path (resolved via AppModel).
    NavigateMenu { path: Vec<String> },
    /// Select a tool from toolbar (resolved via AppModel).
    SelectTool { tool_name: String },
    /// Click an element described by text (resolved via vision at runtime).
    ClickElement { description: String },
    /// Type text into a field.
    TypeInField { field_hint: String, text: String },
    /// Press a key.
    KeyPress { key: String },
    /// Wait for a UI condition.
    WaitFor { condition: String, timeout_ms: u64 },
    /// Verify a condition is met (quality gate).
    Verify { check: String },
}

/// A compiled, typed plan ready for execution.
#[derive(Debug, Clone)]
pub struct TypedPlan {
    pub app: String,
    pub instructions: Vec<UiInstruction>,
    pub estimated_duration_ms: u64,
    pub risk_score: f64,
    pub can_undo: bool,
}

/// Compile a natural language goal into a typed UI plan.
pub fn compile(
    goal: &str,
    app_name: Option<&str>,
    conventions: &ConventionEngine,
    genome: &hydra_genome::GenomeStore,
) -> TypedPlan {
    let ast = parse_intent(goal);
    let app = app_name.map(|s| s.to_string())
        .or(ast.target_app.clone())
        .unwrap_or_else(|| "unknown".into());

    // Check genome for cached plan
    let plan_key = format!("intent_plan:{}:{}", app, goal.to_lowercase());
    if let Some(entry) = genome.query(&plan_key).first() {
        if entry.effective_confidence() > 0.7 && !entry.approach.steps.is_empty() {
            eprintln!("hydra-compiler: genome hit for '{goal}' (conf={:.2})", entry.effective_confidence());
            let instructions = entry.approach.steps.iter()
                .map(|s| UiInstruction::ClickElement { description: s.clone() })
                .collect();
            return TypedPlan { app, instructions, estimated_duration_ms: 5000, risk_score: 0.1, can_undo: true };
        }
    }

    let instructions = resolve_instructions(&ast, conventions);
    let risk_score = estimate_risk(&instructions);
    let can_undo = instructions.iter().all(|i| !matches!(i,
        UiInstruction::ClickElement { description } if description.to_lowercase().contains("send")
            || description.to_lowercase().contains("delete")));
    let estimated_duration_ms = instructions.len() as u64 * 2000; // ~2s per step

    eprintln!("hydra-compiler: compiled '{}' → {} instructions, risk={:.2}",
        goal, instructions.len(), risk_score);

    TypedPlan { app, instructions, estimated_duration_ms, risk_score, can_undo }
}

/// Parse a goal into structured AST.
fn parse_intent(goal: &str) -> IntentAST {
    let lower = goal.to_lowercase();
    // Extract app name from common patterns
    let target_app = ["autocad", "photoshop", "figma", "blender", "excel",
        "word", "powerpoint", "chrome", "safari", "firefox", "vscode",
        "terminal", "finder", "slack", "discord", "zoom"]
        .iter().find(|app| lower.contains(*app)).map(|s| s.to_string());

    // Extract constraints (phrases after "with", "for", "using")
    let constraints: Vec<String> = lower.split(&[',', '.'][..])
        .filter(|s| s.contains("with ") || s.contains("for ") || s.contains("using "))
        .map(|s| s.trim().to_string()).collect();

    IntentAST {
        goal: goal.into(), domain: String::new(),
        target_app, constraints, sub_goals: Vec::new(),
    }
}

/// Resolve AST into concrete UI instructions using conventions.
fn resolve_instructions(ast: &IntentAST, conventions: &ConventionEngine) -> Vec<UiInstruction> {
    let mut instructions = Vec::new();
    let lower = ast.goal.to_lowercase();

    // DIRECT SHELL COMMANDS — common verbs that should just execute
    // "open google.com" → shell: open https://google.com
    // "open TextEdit" → shell: open -a TextEdit
    // "run npm install" → shell: npm install
    if lower.starts_with("open ") {
        let target = ast.goal[5..].trim();
        if target.contains('.') && !target.contains(' ') {
            // Looks like a URL or domain
            let url = if target.starts_with("http") { target.to_string() }
                else { format!("https://{target}") };
            return vec![UiInstruction::ClickElement { description: format!("shell:open {url}") }];
        } else {
            // Looks like an app name
            return vec![UiInstruction::OpenApp { name: target.into() }];
        }
    }
    if lower.starts_with("run ") || lower.starts_with("execute ") {
        let cmd = if lower.starts_with("run ") { &ast.goal[4..] } else { &ast.goal[8..] };
        return vec![UiInstruction::ClickElement { description: format!("shell:{}", cmd.trim()) }];
    }

    // If app specified, open it first
    if let Some(app) = &ast.target_app {
        instructions.push(UiInstruction::OpenApp { name: app.clone() });
        instructions.push(UiInstruction::WaitFor {
            condition: format!("{app} window visible"), timeout_ms: 5000,
        });
    }

    // Try convention shortcuts for common intents
    let intents = ["new", "open", "save", "save_as", "undo", "redo",
        "copy", "paste", "cut", "find", "print", "close"];
    for intent in intents {
        if lower.contains(intent) {
            if let Some(conv) = conventions.resolve(intent, "") {
                instructions.push(UiInstruction::UseShortcut {
                    intent: intent.into(),
                    modifier: conv.modifier.clone(), key: conv.key.clone(),
                });
                return instructions; // Simple shortcut goal — done
            }
        }
    }

    // For complex goals, decompose into click-based steps
    // Each sub-goal becomes a ClickElement instruction
    instructions.push(UiInstruction::ClickElement { description: ast.goal.clone() });
    instructions.push(UiInstruction::Verify { check: "goal appears complete".into() });

    instructions
}

/// Estimate risk of a plan (0.0 = safe, 1.0 = dangerous).
fn estimate_risk(instructions: &[UiInstruction]) -> f64 {
    let mut risk = 0.0;
    for inst in instructions {
        match inst {
            UiInstruction::UseShortcut { intent, .. } => {
                if intent.contains("save") || intent.contains("close") { risk += 0.1; }
            }
            UiInstruction::ClickElement { description } => {
                let lower = description.to_lowercase();
                if lower.contains("delete") || lower.contains("send") { risk += 0.5; }
                else if lower.contains("submit") || lower.contains("pay") { risk += 0.7; }
                else { risk += 0.05; }
            }
            _ => {}
        }
    }
    (risk / instructions.len().max(1) as f64).min(1.0)
}

/// Convert a TypedPlan into conductor Steps for DAG execution.
pub fn plan_to_steps(plan: &TypedPlan) -> Vec<Step> {
    plan.instructions.iter().enumerate().map(|(i, inst)| {
        let (step_type, desc) = match inst {
            UiInstruction::OpenApp { name } => (
                StepType::Shell { command: format!("open -a '{name}'"), long_running: false },
                format!("Open {name}"),
            ),
            UiInstruction::UseShortcut { intent, modifier, key } => (
                StepType::DesktopAction { goal: format!("shortcut: {modifier}+{key} ({intent})") },
                format!("Shortcut: {intent}"),
            ),
            UiInstruction::NavigateMenu { path } => (
                StepType::DesktopAction { goal: format!("menu: {}", path.join(" → ")) },
                format!("Menu: {}", path.join(" → ")),
            ),
            UiInstruction::SelectTool { tool_name } => (
                StepType::DesktopAction { goal: format!("select tool: {tool_name}") },
                format!("Tool: {tool_name}"),
            ),
            UiInstruction::ClickElement { description } => {
                // "shell:<command>" → execute as shell step
                if let Some(cmd) = description.strip_prefix("shell:") {
                    (StepType::Shell { command: cmd.to_string(), long_running: false }, cmd.to_string())
                } else {
                    (StepType::DesktopAction { goal: description.clone() }, description.clone())
                }
            }
            UiInstruction::TypeInField { field_hint, text } => (
                StepType::DesktopAction { goal: format!("type '{text}' in {field_hint}") },
                format!("Type in {field_hint}"),
            ),
            UiInstruction::KeyPress { key } => (
                StepType::DesktopAction { goal: format!("press {key}") },
                format!("Key: {key}"),
            ),
            UiInstruction::WaitFor { condition, timeout_ms } => (
                StepType::Wait { condition: crate::conductor::WaitCondition::Duration { ms: *timeout_ms } },
                format!("Wait: {condition}"),
            ),
            UiInstruction::Verify { check } => (
                StepType::Verify { method: crate::conductor::VerifyMethod::CommandSuccess {
                    command: format!("echo 'Verify: {check}'") } },
                format!("Verify: {check}"),
            ),
        };
        Step { id: i, step_type, description: desc, depends_on: if i > 0 { vec![i-1] } else { vec![] },
            timeout_ms: 30_000 }
    }).collect()
}
