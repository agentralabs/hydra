//! O34: Deliberation Engine — adaptive thinking before acting.
//!
//! 5 cognitive modes: ASSESS → RESEARCH → PLAN → CRITIQUE → EXECUTE
//! Depth function: complexity × (1 - confidence) × novelty
//! Adapts thinking depth to task difficulty. Simple tasks skip thinking.
//! Complex novel tasks get deep research + multiple plan revision cycles.

use serde::{Deserialize, Serialize};

/// The 5 cognitive modes of deliberation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CognitiveMode {
    Assess,
    Research,
    Plan,
    Critique,
    Execute,
}

impl CognitiveMode {
    pub fn label(&self) -> &str {
        match self {
            Self::Assess => "ASSESS",
            Self::Research => "RESEARCH",
            Self::Plan => "PLAN",
            Self::Critique => "CRITIQUE",
            Self::Execute => "EXECUTE",
        }
    }
}

/// A single visible thinking step (rendered in TUI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingStep {
    pub mode: CognitiveMode,
    pub thought: String,
    pub result: ThinkingResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThinkingResult {
    Confident { reason: String },
    NeedResearch { gaps: Vec<String> },
    PlanReady { steps: usize },
    PlanFlawed { issues: Vec<String> },
    Proceed,
}

/// A plan step with confidence.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub description: String,
    pub confidence: f64,
}

/// Full deliberation state across the thinking cycle.
pub struct DeliberationState {
    pub mode: CognitiveMode,
    pub task: String,
    pub domain: String,
    pub depth: f64,
    pub knowledge_gaps: Vec<String>,
    pub research_findings: Vec<String>,
    pub plan: Vec<PlanStep>,
    pub critiques: Vec<String>,
    pub iterations: u32,
    pub max_iterations: u32,
    pub thinking_log: Vec<ThinkingStep>,
}

impl DeliberationState {
    pub fn new(task: &str, domain: &str) -> Self {
        Self {
            mode: CognitiveMode::Assess, task: task.into(), domain: domain.into(),
            depth: 0.0, knowledge_gaps: Vec::new(), research_findings: Vec::new(),
            plan: Vec::new(), critiques: Vec::new(),
            iterations: 0, max_iterations: 6, thinking_log: Vec::new(),
        }
    }
}

/// Compute deliberation depth: how much thinking does this task need?
/// depth = complexity × (1 - confidence) × novelty
pub fn compute_depth(
    task: &str,
    genome: &hydra_genome::GenomeStore,
) -> f64 {
    // Complexity: estimate sub-steps from task length and keywords
    let words = task.split_whitespace().count();
    let has_multi = task.contains(" and ") || task.contains(" then ") || task.contains(",");
    let complexity = if words < 5 && !has_multi { 0.2 }
        else if words < 15 { 0.5 }
        else { 0.8 };

    // Confidence: genome match for this domain
    let matches = genome.query(task);
    let confidence = matches.first()
        .map(|e| e.effective_confidence()).unwrap_or(0.1);

    // Novelty: has Hydra done similar tasks?
    let novelty = if matches.is_empty() { 1.0 }
        else if matches.first().map(|e| e.use_count).unwrap_or(0) > 5 { 0.2 }
        else { 0.6 };

    let depth = complexity * (1.0 - confidence) * novelty;
    eprintln!("hydra-deliberation: depth={depth:.2} (complexity={complexity:.2} confidence={confidence:.2} novelty={novelty:.2})");
    depth.clamp(0.0, 1.0)
}

/// Should we deliberate at all, or is this a simple query?
pub fn should_deliberate(task: &str, depth: f64) -> bool {
    // Skip deliberation for simple conversational queries
    let lower = task.to_lowercase();
    if lower.starts_with("what ") || lower.starts_with("who ") || lower.starts_with("how ")
        || lower.starts_with("why ") || lower.starts_with("when ") || lower.starts_with("where ")
        || lower.len() < 20 {
        return false;
    }
    depth > 0.2
}

/// Run the full deliberation state machine.
pub fn deliberate(
    task: &str,
    domain: &str,
    genome: &hydra_genome::GenomeStore,
) -> DeliberationState {
    let depth = compute_depth(task, genome);
    let mut state = DeliberationState::new(task, domain);
    state.depth = depth;
    state.max_iterations = if depth > 0.8 { 6 } else if depth > 0.5 { 4 } else { 2 };

    if !should_deliberate(task, depth) {
        state.mode = CognitiveMode::Execute;
        state.thinking_log.push(ThinkingStep {
            mode: CognitiveMode::Assess,
            thought: format!("Simple task (depth={depth:.2}) — proceeding directly"),
            result: ThinkingResult::Proceed,
        });
        return state;
    }

    for _ in 0..state.max_iterations {
        state.iterations += 1;
        match state.mode {
            CognitiveMode::Assess => assess(&mut state, genome),
            CognitiveMode::Research => research(&mut state, genome),
            CognitiveMode::Plan => plan(&mut state, genome),
            CognitiveMode::Critique => critique(&mut state, genome),
            CognitiveMode::Execute => break,
        }
    }

    // If we ran out of iterations without reaching Execute, force it
    if state.mode != CognitiveMode::Execute {
        state.thinking_log.push(ThinkingStep {
            mode: CognitiveMode::Assess,
            thought: format!("Max iterations ({}) reached — proceeding with current plan", state.max_iterations),
            result: ThinkingResult::Proceed,
        });
        state.mode = CognitiveMode::Execute;
    }

    state
}

// ── Mode Implementations ──

fn assess(state: &mut DeliberationState, genome: &hydra_genome::GenomeStore) {
    let domain_entries = genome.query(&state.domain);
    let confidence = domain_entries.first()
        .map(|e| e.effective_confidence()).unwrap_or(0.0);
    let entry_count = domain_entries.len();

    let mut gaps = Vec::new();
    if confidence < 0.5 {
        gaps.push(format!("Low confidence in '{}' ({:.0}%)", state.domain, confidence * 100.0));
    }
    if entry_count < 3 {
        gaps.push(format!("Only {} genome entries for '{}'", entry_count, state.domain));
    }

    if gaps.is_empty() || state.depth < 0.3 {
        state.thinking_log.push(ThinkingStep {
            mode: CognitiveMode::Assess,
            thought: format!("Domain '{}': {:.0}% confidence, {} entries — sufficient",
                state.domain, confidence * 100.0, entry_count),
            result: ThinkingResult::Confident { reason: "domain knowledge sufficient".into() },
        });
        state.mode = CognitiveMode::Plan;
    } else {
        state.knowledge_gaps = gaps.clone();
        state.thinking_log.push(ThinkingStep {
            mode: CognitiveMode::Assess,
            thought: format!("Knowledge gaps: {}", gaps.join(", ")),
            result: ThinkingResult::NeedResearch { gaps },
        });
        state.mode = CognitiveMode::Research;
    }
}

fn research(state: &mut DeliberationState, genome: &hydra_genome::GenomeStore) {
    let mut findings = Vec::new();

    // Search web for each gap
    for gap in &state.knowledge_gaps {
        let query = format!("{} {}", state.task, gap);
        let mut web = hydra_web::SearchOrchestrator::new();
        if let Ok(results) = web.search_blocking(&query) {
            let preview: String = results.chars().take(150).collect();
            findings.push(preview);
        }
    }

    // Check genome for related patterns
    let related = genome.query(&state.task);
    for entry in related.iter().take(3) {
        if let Some(step) = entry.approach.steps.first() {
            findings.push(format!("Genome: {} (conf={:.0}%)",
                &step[..step.len().min(80)], entry.effective_confidence() * 100.0));
        }
    }

    state.thinking_log.push(ThinkingStep {
        mode: CognitiveMode::Research,
        thought: format!("Found {} insights for {} gaps", findings.len(), state.knowledge_gaps.len()),
        result: ThinkingResult::Confident { reason: format!("{} findings", findings.len()) },
    });
    state.research_findings.extend(findings);
    state.mode = CognitiveMode::Plan;
}

fn plan(state: &mut DeliberationState, genome: &hydra_genome::GenomeStore) {
    let conventions = crate::convention::ConventionEngine::new();
    let compiled = crate::intent_compiler::compile(&state.task, None, &conventions, genome);

    state.plan = compiled.instructions.iter().map(|inst| {
        PlanStep {
            description: format!("{:?}", inst),
            confidence: if state.research_findings.is_empty() { 0.5 } else { 0.7 },
        }
    }).collect();

    let avg_conf = if state.plan.is_empty() { 0.0 }
        else { state.plan.iter().map(|s| s.confidence).sum::<f64>() / state.plan.len() as f64 };

    state.thinking_log.push(ThinkingStep {
        mode: CognitiveMode::Plan,
        thought: format!("{} steps, avg confidence {:.0}%", state.plan.len(), avg_conf * 100.0),
        result: ThinkingResult::PlanReady { steps: state.plan.len() },
    });

    if state.depth > 0.5 && state.iterations < state.max_iterations - 1 {
        state.mode = CognitiveMode::Critique;
    } else {
        state.mode = CognitiveMode::Execute;
    }
}

fn critique(state: &mut DeliberationState, genome: &hydra_genome::GenomeStore) {
    let mut issues = Vec::new();

    // Low-confidence steps
    for (i, step) in state.plan.iter().enumerate() {
        if step.confidence < 0.4 {
            issues.push(format!("Step {}: low confidence ({:.0}%)", i+1, step.confidence * 100.0));
        }
    }

    // No research for complex task
    if state.research_findings.is_empty() && state.depth > 0.6 {
        issues.push("Complex task with no research — consider searching first".into());
    }

    // Known obstacles in genome
    let obstacles = genome.query(&format!("obstacle:{}", state.task));
    for entry in obstacles.iter().take(2) {
        if entry.effective_confidence() > 0.5 {
            if let Some(obs) = entry.approach.steps.first() {
                issues.push(format!("Known obstacle: {}", &obs[..obs.len().min(60)]));
            }
        }
    }

    state.critiques = issues.clone();
    state.thinking_log.push(ThinkingStep {
        mode: CognitiveMode::Critique,
        thought: if issues.is_empty() {
            "Plan review passed — ready to execute".into()
        } else {
            format!("{} issues: {}", issues.len(), issues.join("; "))
        },
        result: if issues.is_empty() {
            ThinkingResult::Proceed
        } else {
            ThinkingResult::PlanFlawed { issues: issues.clone() }
        },
    });

    if issues.is_empty() {
        state.mode = CognitiveMode::Execute;
    } else if issues.iter().any(|i| i.contains("research") || i.contains("searching")) {
        state.mode = CognitiveMode::Research;
    } else {
        state.mode = CognitiveMode::Plan; // revise
    }
}
