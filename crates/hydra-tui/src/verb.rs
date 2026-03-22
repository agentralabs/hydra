//! Thinking verb display — the 12 permanent verb contexts.
//!
//! Each verb context maps to a sister and has a permanent color
//! and 4 alternative verbs. From HYDRA-THINKING-VERBS.md.
//! These assignments NEVER change. Same verb = same color. Always.

use ratatui::style::Color;

use crate::constants;

/// The 12 verb contexts with permanent colors, mapped to sisters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerbContext {
    /// General LLM reasoning, no specific sister active.
    General,
    /// Forge sister — code/content generation.
    Forge,
    /// Codebase sister — reading/searching code.
    Codebase,
    /// Memory sister — recall/retrieval.
    Memory,
    /// Generative/Cognition sister — synthesis.
    Synthesis,
    /// Workflow sister / fleet coordination.
    Workflow,
    /// Veritas sister — truth/fact-checking.
    Veritas,
    /// Aegis sister / adversary system.
    Aegis,
    /// Dream thread — overnight consolidation.
    Dream,
    /// Persona switch / identity operations.
    Persona,
    /// Data sister — processing/analysis.
    Data,
    /// Cross-sister ring operations (Hydra-branded).
    HydraBranded,
}

impl VerbContext {
    /// Return the permanent RGB color for this verb context.
    pub fn rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::General => constants::VERB_COLOR_GENERAL,
            Self::Forge => constants::VERB_COLOR_FORGE,
            Self::Codebase => constants::VERB_COLOR_CODEBASE,
            Self::Memory => constants::VERB_COLOR_MEMORY,
            Self::Synthesis => constants::VERB_COLOR_SYNTHESIS,
            Self::Workflow => constants::VERB_COLOR_WORKFLOW,
            Self::Veritas => constants::VERB_COLOR_VERITAS,
            Self::Aegis => constants::VERB_COLOR_AEGIS,
            Self::Dream => constants::VERB_COLOR_DREAM,
            Self::Persona => constants::VERB_COLOR_PERSONA,
            Self::Data => constants::VERB_COLOR_DATA,
            Self::HydraBranded => constants::VERB_COLOR_HYDRA_BRANDED,
        }
    }

    /// Return the ratatui color for this verb context.
    pub fn color(&self) -> Color {
        let (r, g, b) = self.rgb();
        Color::Rgb(r, g, b)
    }

    /// Return the 4 alternative verbs for this context.
    pub fn alternatives(&self) -> &'static [&'static str; 4] {
        match self {
            Self::General => &["Cogitating", "Ruminating", "Deliberating", "Musing"],
            Self::Forge => &["Forging", "Smithing", "Blueprinting", "Crafting"],
            Self::Codebase => &["Scanning", "Parsing", "Traversing", "Indexing"],
            Self::Memory => &["Remembering", "Recollecting", "Excavating", "Surfacing"],
            Self::Synthesis => &["Synthesizing", "Ideating", "Contemplating", "Composing"],
            Self::Workflow => &["Orchestrating", "Sequencing", "Pipelining", "Routing"],
            Self::Veritas => &["Verifying", "Truthing", "Validating", "Cross-checking"],
            Self::Aegis => &["Shielding", "Fortifying", "Guarding", "Sentineling"],
            Self::Dream => &["Dreaming", "Drifting", "Night-thinking", "Star-gazing"],
            Self::Persona => &["Channeling", "Voicing", "Shifting", "Embodying"],
            Self::Data => &["Crunching", "Munging", "Tabulating", "Correlating"],
            Self::HydraBranded => &["Hydrating", "Multi-minding", "Ring-resonating", "Hydrating"],
        }
    }

    /// Return the primary (first) verb for this context.
    pub fn primary_verb(&self) -> &'static str {
        self.alternatives()[0]
    }

    /// Return the past-tense completion verb for status display.
    pub fn completion_verb(&self) -> &'static str {
        match self {
            Self::General => "Cogitated",
            Self::Forge => "Forged",
            Self::Codebase => "Scanned",
            Self::Memory => "Remembered",
            Self::Synthesis => "Synthesized",
            Self::Workflow => "Orchestrated",
            Self::Veritas => "Verified",
            Self::Aegis => "Shielded",
            Self::Dream => "Dreamed",
            Self::Persona => "Channeled",
            Self::Data => "Crunched",
            Self::HydraBranded => "Hydrated",
        }
    }

    /// Map a sister name to the appropriate verb context.
    pub fn from_sister(sister: &str) -> Self {
        match sister.to_lowercase().as_str() {
            "forge" | "agentic-forge" => Self::Forge,
            "codebase" | "agentic-codebase" => Self::Codebase,
            "memory" | "agentic-memory" => Self::Memory,
            "cognition" | "agentic-cognition" => Self::Synthesis,
            "veritas" | "agentic-veritas" => Self::Veritas,
            "aegis" | "agentic-aegis" => Self::Aegis,
            "workflow" | "agentic-workflow" => Self::Workflow,
            "data" | "agentic-data" => Self::Data,
            "identity" | "agentic-identity" => Self::Persona,
            "reality" | "agentic-reality" => Self::General,
            "time" | "agentic-time" => Self::General,
            _ => Self::General,
        }
    }

    /// Return all 12 verb contexts.
    pub fn all() -> &'static [VerbContext] {
        &[
            Self::General,
            Self::Forge,
            Self::Codebase,
            Self::Memory,
            Self::Synthesis,
            Self::Workflow,
            Self::Veritas,
            Self::Aegis,
            Self::Dream,
            Self::Persona,
            Self::Data,
            Self::HydraBranded,
        ]
    }
}

/// State for the currently displayed thinking verb.
#[derive(Debug, Clone)]
pub struct ThinkingVerbState {
    /// Current verb context.
    context: VerbContext,
    /// Current alternative index (0..3).
    alt_index: usize,
    /// Current spinner frame index.
    spinner_index: usize,
    /// Whether the verb is actively spinning.
    active: bool,
}

impl ThinkingVerbState {
    /// Create a new thinking verb state.
    pub fn new(context: VerbContext) -> Self {
        Self {
            context,
            alt_index: 0,
            spinner_index: 0,
            active: false,
        }
    }

    /// Start the thinking animation.
    pub fn start(&mut self) {
        self.active = true;
        self.spinner_index = 0;
    }

    /// Stop the thinking animation.
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Whether the verb is actively spinning.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Advance the spinner by one frame.
    pub fn tick_spinner(&mut self) {
        if self.active {
            self.spinner_index = (self.spinner_index + 1) % constants::SPINNER_FRAMES.len();
        }
    }

    /// Rotate to the next verb alternative.
    pub fn rotate_verb(&mut self) {
        self.alt_index = (self.alt_index + 1) % self.context.alternatives().len();
    }

    /// Change the verb context.
    pub fn set_context(&mut self, context: VerbContext) {
        self.context = context;
        self.alt_index = 0;
    }

    /// Return the current verb context.
    pub fn context(&self) -> VerbContext {
        self.context
    }

    /// Build the status display string: "Forging◑" (verb + spinner, same color).
    pub fn status_display(&self) -> String {
        if !self.active {
            return String::new();
        }
        let spinner = constants::SPINNER_FRAMES
            .get(self.spinner_index)
            .copied()
            .unwrap_or("◌");
        let verb = self
            .context
            .alternatives()
            .get(self.alt_index)
            .copied()
            .unwrap_or("Thinking");
        format!("{verb}{spinner}")
    }

    /// Build the completion line: "● Remembered for 0.1s" (with duration).
    pub fn completion_display(&self, duration_secs: f64) -> String {
        let verb = self.context.completion_verb();
        format!("● {verb} for {duration_secs:.1}s")
    }

    /// Build the completion line: "● Scanned" (without duration).
    pub fn completion_line(&self) -> String {
        let verb = self.context.completion_verb();
        format!("● {verb}")
    }

    /// Return the current spinner frame.
    pub fn spinner_frame(&self) -> &'static str {
        constants::SPINNER_FRAMES
            .get(self.spinner_index)
            .copied()
            .unwrap_or("◌")
    }
}

impl Default for ThinkingVerbState {
    fn default() -> Self {
        Self::new(VerbContext::General)
    }
}
