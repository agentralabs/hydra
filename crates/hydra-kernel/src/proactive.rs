//! O31: Proactive Initiation — start working without being asked.
//!
//! Watches triggers (calendar, file changes, genome patterns, workspace tasks,
//! web alerts, schedules) and initiates action when conditions are met.
//! Uses O29 Autonomy Gradient to decide whether to act or ask first.
//! Connects to O27 Intent Compiler for plan generation.

use std::time::Instant;

/// A trigger source that may initiate autonomous action.
#[derive(Debug, Clone)]
pub struct TriggerSource {
    pub source_type: TriggerType,
    pub description: String,
    pub urgency: f64,
    pub confidence: f64,
    pub suggested_goal: String,
}

/// Types of triggers that can initiate proactive work.
#[derive(Debug, Clone)]
pub enum TriggerType {
    /// Calendar event approaching deadline.
    Calendar { event: String, due: String },
    /// Monitor detected an event (CI fail, API alert, etc.).
    Monitor { event_type: String, details: String },
    /// Genome pattern match: "when X happens, do Y".
    Genome { pattern: String, situation: String },
    /// Pending task in workspace.
    Workspace { task: String, progress: f64 },
    /// File system change matching a known pattern.
    FileSystem { path: String, change: String },
    /// Web monitoring alert.
    WebAlert { source: String, content: String },
    /// Scheduled cron job.
    Schedule { cron: String, task: String },
}

/// Decision to initiate autonomous work.
#[derive(Debug, Clone)]
pub struct ProactiveAction {
    pub trigger: TriggerSource,
    pub goal: String,
    pub autonomy_score: f64,
    pub should_notify: bool,
}

/// Engine that evaluates triggers and decides what to initiate.
pub struct ProactiveEngine {
    pub initiation_threshold: f64,
    pub cooldown_minutes: u64,
    recent: Vec<(String, Instant)>,
}

impl ProactiveEngine {
    pub fn new() -> Self {
        Self {
            initiation_threshold: 0.7,
            cooldown_minutes: 30,
            recent: Vec::new(),
        }
    }

    /// Evaluate all triggers and return actions to initiate.
    pub fn evaluate_triggers(
        &mut self,
        triggers: Vec<TriggerSource>,
        genome: &hydra_genome::GenomeStore,
        human_active: bool,
    ) -> Vec<ProactiveAction> {
        let mut actions = Vec::new();
        // Clean expired cooldowns
        self.recent.retain(|(_, t)| t.elapsed().as_secs() < self.cooldown_minutes * 60);

        for trigger in triggers {
            if self.should_initiate(&trigger, genome, human_active) {
                let goal = trigger.suggested_goal.clone();
                let blast = hydra_wisdom::BlastRadius::Contained; // proactive = conservative
                let autonomy = hydra_wisdom::autonomy::compute_autonomy(
                    &goal, trigger.confidence, &blast,
                    0, 0, // no prior history for proactive tasks
                    false,
                );
                if autonomy.value >= self.initiation_threshold {
                    eprintln!("hydra-proactive: initiating '{}' (autonomy={:.2}, trigger={:?})",
                        goal, autonomy.value, trigger.source_type);
                    actions.push(ProactiveAction {
                        trigger: trigger.clone(), goal: goal.clone(),
                        autonomy_score: autonomy.value,
                        should_notify: !matches!(autonomy.decision,
                            hydra_wisdom::autonomy::AutonomyDecision::ActSilently),
                    });
                    self.recent.push((goal, Instant::now()));
                }
            }
        }
        actions
    }

    /// Check if a trigger should be acted on.
    fn should_initiate(
        &self,
        trigger: &TriggerSource,
        _genome: &hydra_genome::GenomeStore,
        human_active: bool,
    ) -> bool {
        // Don't initiate if human is actively working (unless urgent)
        if human_active && trigger.urgency < 0.8 { return false; }
        // Don't initiate if same goal recently started (cooldown)
        if self.recent.iter().any(|(g, _)| *g == trigger.suggested_goal) { return false; }
        // Confidence threshold
        if trigger.confidence < 0.3 { return false; }
        true
    }

    /// Collect triggers from the current system state.
    pub fn collect_triggers(genome: &hydra_genome::GenomeStore) -> Vec<TriggerSource> {
        let mut triggers = Vec::new();

        // Workspace: pending tasks
        if let Some(snap) = crate::workspace::load_snapshot() {
            for task in &snap.pending_tasks {
                if task.progress < 1.0 {
                    triggers.push(TriggerSource {
                        source_type: TriggerType::Workspace {
                            task: task.description.clone(), progress: task.progress,
                        },
                        description: format!("Pending: {}", task.description),
                        urgency: 0.5 + (1.0 - task.progress) * 0.3,
                        confidence: 0.6,
                        suggested_goal: task.description.clone(),
                    });
                }
            }
        }

        // Genome: trigger patterns (entries tagged with "trigger:")
        let trigger_entries = genome.query("trigger:");
        for entry in trigger_entries.iter().take(5) {
            let situation: String = entry.situation.keywords.iter().cloned().collect::<Vec<_>>().join(" ");
            if entry.effective_confidence() > 0.6 {
                triggers.push(TriggerSource {
                    source_type: TriggerType::Genome {
                        pattern: situation.clone(),
                        situation: entry.approach.steps.first().cloned().unwrap_or_default(),
                    },
                    description: format!("Genome: {situation}"),
                    urgency: entry.effective_confidence(),
                    confidence: entry.effective_confidence(),
                    suggested_goal: entry.approach.steps.first().cloned().unwrap_or_default(),
                });
            }
        }

        triggers
    }
}

impl Default for ProactiveEngine {
    fn default() -> Self { Self::new() }
}
