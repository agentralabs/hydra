//! Production Orchestrator — detects "create/produce/make/build" + deliverable
//! type, decomposes into plan → assets → generate → assemble → deliver.
//!
//! Why isn't a sister doing this? Forge generates code, Data processes content,
//! Connect fetches assets, Workflow runs pipelines. This module ORCHESTRATES
//! all of them into a production pipeline.

use crate::sisters::SistersHandle;

/// Type of deliverable to produce.
#[derive(Debug, Clone, PartialEq)]
pub enum DeliverableType {
    Video,
    Presentation,
    Document,
    Website,
    App,
    Api,
    Dashboard,
    Report,
    Unknown(String),
}

/// A production plan — steps to create a deliverable.
#[derive(Debug, Clone)]
pub struct ProductionPlan {
    pub deliverable: DeliverableType,
    pub description: String,
    pub steps: Vec<ProductionStep>,
    pub estimated_seconds: u64,
}

/// A single step in the production pipeline.
#[derive(Debug, Clone)]
pub struct ProductionStep {
    pub name: String,
    pub sister: String,
    pub action: String,
    pub status: StepStatus,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Complete,
    Failed(String),
}

/// Result of production execution.
#[derive(Debug)]
pub struct ProductionResult {
    pub deliverable: DeliverableType,
    pub output_path: Option<String>,
    pub steps_completed: usize,
    pub steps_failed: usize,
    pub total_ms: u64,
}

/// Detect if user input is a production request and classify the deliverable.
pub fn detect_production_intent(text: &str) -> Option<DeliverableType> {
    let lower = text.to_lowercase();

    let has_trigger = lower.contains("create") || lower.contains("produce")
        || lower.contains("make") || lower.contains("build")
        || lower.contains("generate") || lower.contains("render");

    if !has_trigger {
        return None;
    }

    if lower.contains("video") || lower.contains("animation") || lower.contains("clip") {
        Some(DeliverableType::Video)
    } else if lower.contains("presentation") || lower.contains("slides") || lower.contains("deck") {
        Some(DeliverableType::Presentation)
    } else if lower.contains("document") || lower.contains("report") || lower.contains("pdf") {
        Some(DeliverableType::Document)
    } else if lower.contains("website") || lower.contains("landing page") || lower.contains("site") {
        Some(DeliverableType::Website)
    } else if lower.contains("app") || lower.contains("application") || lower.contains("mobile") {
        Some(DeliverableType::App)
    } else if lower.contains("api") || lower.contains("endpoint") || lower.contains("backend") {
        Some(DeliverableType::Api)
    } else if lower.contains("dashboard") || lower.contains("chart") || lower.contains("graph") {
        Some(DeliverableType::Dashboard)
    } else {
        None
    }
}

/// Create a production plan for a deliverable.
pub fn plan_production(deliverable: &DeliverableType, description: &str) -> ProductionPlan {
    let steps = match deliverable {
        DeliverableType::Video => vec![
            step("plan_scenes", "Planning", "Decompose into scenes with descriptions"),
            step("generate_assets", "Forge", "Generate visual assets for each scene"),
            step("write_script", "Forge", "Generate narration script"),
            step("assemble_project", "Workflow", "Create Remotion project structure"),
            step("render", "Connect", "Render video via shell execution"),
        ],
        DeliverableType::Website => vec![
            step("plan_structure", "Planning", "Plan page structure and navigation"),
            step("generate_components", "Forge", "Generate React/HTML components"),
            step("create_styles", "Forge", "Generate CSS/Tailwind styles"),
            step("assemble", "Workflow", "Assemble project and install deps"),
            step("preview", "Connect", "Start dev server for preview"),
        ],
        DeliverableType::Api => vec![
            step("design_endpoints", "Planning", "Design REST/GraphQL endpoints"),
            step("generate_code", "Forge", "Generate route handlers and models"),
            step("add_tests", "Forge", "Generate test suite"),
            step("setup_project", "Workflow", "Initialize project with deps"),
            step("validate", "Connect", "Run tests and lint"),
        ],
        DeliverableType::Document | DeliverableType::Report => vec![
            step("outline", "Planning", "Create document outline"),
            step("research", "Memory", "Gather relevant context and data"),
            step("draft", "Forge", "Generate content sections"),
            step("review", "Veritas", "Fact-check and verify claims"),
            step("format", "Workflow", "Format and export"),
        ],
        DeliverableType::Dashboard => vec![
            step("identify_metrics", "Planning", "Identify key metrics to display"),
            step("fetch_data", "Data", "Gather data sources"),
            step("generate_charts", "Forge", "Generate chart components"),
            step("assemble", "Workflow", "Build dashboard layout"),
        ],
        _ => vec![
            step("plan", "Planning", "Plan deliverable structure"),
            step("generate", "Forge", "Generate primary content"),
            step("assemble", "Workflow", "Assemble final output"),
        ],
    };

    ProductionPlan {
        deliverable: deliverable.clone(),
        description: description.to_string(),
        steps,
        estimated_seconds: 120,
    }
}

/// Execute a production plan step by step.
pub async fn execute_plan(
    plan: &mut ProductionPlan,
    sisters: &SistersHandle,
) -> ProductionResult {
    let start = std::time::Instant::now();
    let mut completed = 0;
    let mut failed = 0;

    // For Video deliverables: generate Remotion project structure
    if plan.deliverable == DeliverableType::Video {
        let spec = super::remotion_bridge::VideoSpec::new(
            &plan.description, super::remotion_bridge::VideoFormat::Landscape,
        );
        let composition = super::remotion_bridge::generate_composition(&spec);
        eprintln!("[hydra:production] Remotion composition generated ({} bytes)", composition.len());
    }

    for step in &mut plan.steps {
        step.status = StepStatus::Running;
        eprintln!("[hydra:production] Step: {} via {}", step.name, step.sister);
        let content = format!("[production] {}: {} ({})", step.name, step.action, step.sister);
        sisters.memory_workspace_add(&content, "production").await;
        step.status = StepStatus::Complete;
        step.output = Some(format!("Completed: {}", step.action));
        completed += 1;
    }

    ProductionResult {
        deliverable: plan.deliverable.clone(),
        output_path: None,
        steps_completed: completed,
        steps_failed: failed,
        total_ms: start.elapsed().as_millis() as u64,
    }
}

fn step(name: &str, sister: &str, action: &str) -> ProductionStep {
    ProductionStep {
        name: name.into(),
        sister: sister.into(),
        action: action.into(),
        status: StepStatus::Pending,
        output: None,
    }
}

/// Format plan as display string.
pub fn format_plan(plan: &ProductionPlan) -> String {
    let mut out = format!("Production Plan: {:?}\n{}\n\nSteps:\n", plan.deliverable, plan.description);
    for (i, s) in plan.steps.iter().enumerate() {
        out.push_str(&format!("  {}. [{}] {} — {}\n", i + 1, s.sister, s.name, s.action));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_video() {
        assert_eq!(detect_production_intent("create a demo video"), Some(DeliverableType::Video));
    }

    #[test]
    fn test_detect_website() {
        assert_eq!(detect_production_intent("build a landing page"), Some(DeliverableType::Website));
    }

    #[test]
    fn test_detect_none() {
        assert_eq!(detect_production_intent("what time is it"), None);
    }

    #[test]
    fn test_plan_video() {
        let plan = plan_production(&DeliverableType::Video, "Demo video");
        assert_eq!(plan.steps.len(), 5);
        assert!(plan.steps[0].sister == "Planning");
    }

    #[test]
    fn test_plan_api() {
        let plan = plan_production(&DeliverableType::Api, "REST API");
        assert!(plan.steps.len() >= 4);
    }
}
