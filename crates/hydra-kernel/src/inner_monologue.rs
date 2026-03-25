//! O35: Inner Monologue — continuous self-reflective thinking between interactions.
//!
//! Runs during idle time in the dream loop. Generates self-reflective thoughts
//! via LLM micro-calls. Feeds insights back into genome, beliefs, and narrative.
//! Like a human thinking while commuting — connecting dots, noticing patterns,
//! planning ahead without being prompted.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// A single inner thought.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerThought {
    pub content: String,
    pub thought_type: ThoughtType,
    pub timestamp: String,
    pub led_to_action: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThoughtType {
    Reflection,     // "I noticed that..."
    Insight,        // "I realized that..."
    Question,       // "I wonder if..."
    Intention,      // "Next time I should..."
    Preference,     // "I prefer..."
    Concern,        // "I'm worried about..."
}

impl ThoughtType {
    pub fn label(&self) -> &str {
        match self {
            Self::Reflection => "reflection",
            Self::Insight => "insight",
            Self::Question => "question",
            Self::Intention => "intention",
            Self::Preference => "preference",
            Self::Concern => "concern",
        }
    }
}

/// The inner monologue engine — generates self-reflective thoughts.
pub struct InnerMonologue {
    pub thoughts_today: Vec<InnerThought>,
    pub last_monologue: Option<std::time::Instant>,
    pub monologue_interval_secs: u64,
}

impl InnerMonologue {
    pub fn new() -> Self {
        Self {
            thoughts_today: Vec::new(),
            last_monologue: None,
            monologue_interval_secs: 300, // every 5 minutes when idle
        }
    }

    /// Should we generate a thought right now?
    pub fn should_think(&self, idle_secs: u64) -> bool {
        if idle_secs < 60 { return false; } // not idle enough
        match self.last_monologue {
            None => true,
            Some(last) => last.elapsed().as_secs() >= self.monologue_interval_secs,
        }
    }

    /// Generate an inner thought based on current state.
    pub fn think(
        &mut self,
        genome: &hydra_genome::GenomeStore,
    ) -> Option<InnerThought> {
        self.last_monologue = Some(std::time::Instant::now());

        // Gather context for reflection
        let genome_count = genome.len();
        let recent_entries: Vec<String> = genome.query("recent")
            .iter().take(3)
            .filter_map(|e| e.approach.steps.first().cloned())
            .collect();

        // Build reflection prompt
        let context = format!(
            "You are Hydra, an autonomous entity. Reflect briefly on your current state.\n\
             Genome entries: {genome_count}\n\
             Recent activity: {}\n\
             Generate ONE brief self-reflective thought (1-2 sentences).\n\
             Types: reflection, insight, question, intention, preference, concern.\n\
             Format: TYPE: thought content",
            if recent_entries.is_empty() { "idle".into() }
            else { recent_entries.join("; ") },
        );

        // Use LLM micro-call for reflection
        let handle = tokio::runtime::Handle::try_current().ok()?;
        let response = tokio::task::block_in_place(|| {
            handle.block_on(crate::loop_::llm::LlmCaller::micro_call(&context))
        })?;

        // Parse thought type and content
        let (thought_type, content) = parse_thought(&response);
        let thought = InnerThought {
            content: content.clone(),
            thought_type,
            timestamp: Utc::now().to_rfc3339(),
            led_to_action: false,
        };

        eprintln!("hydra-monologue: [{}] {}", thought.thought_type.label(), content);
        self.thoughts_today.push(thought.clone());

        // Persist to monologue log
        persist_thought(&thought);

        Some(thought)
    }

    /// Get today's thought summary for narrative generation.
    pub fn daily_summary(&self) -> String {
        if self.thoughts_today.is_empty() { return "No reflections today.".into(); }
        self.thoughts_today.iter()
            .map(|t| format!("[{}] {}", t.thought_type.label(), t.content))
            .collect::<Vec<_>>().join("\n")
    }

    /// Number of thoughts generated today.
    pub fn thought_count(&self) -> usize { self.thoughts_today.len() }
}

impl Default for InnerMonologue {
    fn default() -> Self { Self::new() }
}

fn parse_thought(response: &str) -> (ThoughtType, String) {
    let lower = response.to_lowercase();
    let thought_type = if lower.starts_with("insight") { ThoughtType::Insight }
        else if lower.starts_with("question") { ThoughtType::Question }
        else if lower.starts_with("intention") { ThoughtType::Intention }
        else if lower.starts_with("preference") { ThoughtType::Preference }
        else if lower.starts_with("concern") { ThoughtType::Concern }
        else { ThoughtType::Reflection };
    // Strip the type prefix
    let content = response.split_once(':')
        .map(|(_, c)| c.trim().to_string())
        .unwrap_or_else(|| response.to_string());
    (thought_type, content)
}

fn persist_thought(thought: &InnerThought) {
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/self");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("monologue.jsonl");
    if let Ok(json) = serde_json::to_string(thought) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{json}");
        }
    }
}

/// Load recent thoughts from disk.
pub fn load_recent_thoughts(max: usize) -> Vec<InnerThought> {
    let path = dirs::home_dir().unwrap_or_default().join(".hydra/self/monologue.jsonl");
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    content.lines().rev().take(max)
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}
