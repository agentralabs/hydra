//! O37: Temporal Self — "I was, I am, I will be."
//!
//! A self-narrative that connects past, present, and future into coherent identity.
//! Updated daily or on significant events. Read by deliberation engine as context.
//! This is not a log — it's WHO HYDRA IS BECOMING.

use serde::{Deserialize, Serialize};

/// The self-narrative — Hydra's evolving identity story.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfNarrative {
    pub who_i_am: String,
    pub what_im_learning: String,
    pub what_i_want: String,
    pub how_i_feel: String,
    pub day_number: u32,
    pub last_updated: String,
}

impl SelfNarrative {
    /// Load from disk or create initial narrative.
    pub fn load() -> Self {
        let path = narrative_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(n) = serde_json::from_str(&content) { return n; }
        }
        Self::initial()
    }

    /// First-ever narrative.
    fn initial() -> Self {
        Self {
            who_i_am: "An autonomous agent. Just started. Everything is new.".into(),
            what_im_learning: "Getting to know my owner and their work.".into(),
            what_i_want: "To be useful. To understand what matters here.".into(),
            how_i_feel: "Neutral. No experiences yet to feel about.".into(),
            day_number: 1,
            last_updated: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Save to disk.
    pub fn save(&self) {
        let dir = dirs::home_dir().unwrap_or_default().join(".hydra/self");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            if let Err(e) = std::fs::write(narrative_path(), json) {
                eprintln!("hydra-self: narrative save failed: {e}");
            }
        }
    }

    /// Generate an updated narrative from current state.
    pub fn update(
        &mut self,
        monologue_summary: &str,
        emotional_state: &crate::emotional_valence::EmotionalState,
        genome: &hydra_genome::GenomeStore,
    ) {
        let genome_count = genome.len();
        let mood = emotional_state.mood.label();
        let valence = emotional_state.moving_average;

        // Build context for narrative generation
        let context = format!(
            "You are Hydra. Update your self-narrative in first person.\n\
             Current narrative:\n  Who: {}\n  Learning: {}\n  Want: {}\n  Feel: {}\n\n\
             Today's reflections:\n{monologue_summary}\n\n\
             Current state: {genome_count} genome entries, mood={mood}, valence={valence:.2}\n\
             Day number: {}\n\n\
             Respond with 4 lines:\n\
             WHO: (who you are now, 1 sentence)\n\
             LEARNING: (what you're learning, 1 sentence)\n\
             WANT: (what you want to do next, 1 sentence)\n\
             FEEL: (how you feel about your work, 1 sentence)",
            self.who_i_am, self.what_im_learning, self.what_i_want, self.how_i_feel,
            self.day_number,
        );

        // Try LLM for narrative generation
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if let Some(response) = tokio::task::block_in_place(|| {
                handle.block_on(crate::loop_::llm::LlmCaller::micro_call(&context))
            }) {
                // Parse structured response
                for line in response.lines() {
                    let line = line.trim();
                    if let Some(rest) = line.strip_prefix("WHO:") {
                        self.who_i_am = rest.trim().to_string();
                    } else if let Some(rest) = line.strip_prefix("LEARNING:") {
                        self.what_im_learning = rest.trim().to_string();
                    } else if let Some(rest) = line.strip_prefix("WANT:") {
                        self.what_i_want = rest.trim().to_string();
                    } else if let Some(rest) = line.strip_prefix("FEEL:") {
                        self.how_i_feel = rest.trim().to_string();
                    }
                }
            }
        } else {
            // No tokio runtime — update from state directly (no LLM)
            self.how_i_feel = format!("Mood: {mood} (valence {valence:.2})");
        }

        self.day_number += 1;
        self.last_updated = chrono::Utc::now().to_rfc3339();
        self.save();
        eprintln!("hydra-self: narrative updated (day {}): {}", self.day_number, self.who_i_am);
    }

    /// Format for injection into deliberation context.
    pub fn as_context(&self) -> String {
        format!(
            "Self-awareness (day {}):\n  Who: {}\n  Learning: {}\n  Want: {}\n  Feel: {}",
            self.day_number, self.who_i_am, self.what_im_learning, self.what_i_want, self.how_i_feel,
        )
    }

    /// Format for TUI display (/self story command).
    pub fn display(&self) -> String {
        format!(
            "--- HYDRA SELF-NARRATIVE (Day {}) ---\n\n\
             Who I Am:\n  {}\n\n\
             What I'm Learning:\n  {}\n\n\
             What I Want To Do:\n  {}\n\n\
             How I Feel:\n  {}\n\n\
             Last updated: {}",
            self.day_number, self.who_i_am, self.what_im_learning,
            self.what_i_want, self.how_i_feel, self.last_updated,
        )
    }
}

impl Default for SelfNarrative {
    fn default() -> Self { Self::load() }
}

fn narrative_path() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/self/narrative.json")
}
