//! Mentor System — tracks concept understanding per domain, uses spaced
//! repetition, teaches instead of answering when user knows enough.
//!
//! Why isn't a sister doing this? Cognition sister models user behavior.
//! This module owns the TEACHING strategy — when to explain vs challenge.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Global knowledge tracker — persists across sessions.
pub static GLOBAL_MENTOR: OnceLock<Mutex<MentorState>> = OnceLock::new();
pub fn mentor_state() -> &'static Mutex<MentorState> {
    GLOBAL_MENTOR.get_or_init(|| Mutex::new(MentorState::new()))
}

/// Understanding level for a concept.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum UnderstandingLevel {
    Novice,       // 0-2 interactions
    Familiar,     // 3-7 interactions
    Competent,    // 8-15 interactions
    Proficient,   // 16-30 interactions
    Expert,       // 30+ interactions
}

/// A tracked concept the user is learning.
#[derive(Debug, Clone)]
pub struct TrackedConcept {
    pub name: String,
    pub domain: String,
    pub interaction_count: u32,
    pub correct_answers: u32,
    pub last_reviewed: Option<String>,
    pub next_review: Option<String>,
    pub level: UnderstandingLevel,
}

/// Teaching mode — what Hydra should do for this concept.
#[derive(Debug, Clone, PartialEq)]
pub enum TeachingMode {
    Explain,    // User is novice — full explanation
    Guide,      // User is familiar — guided discovery
    Challenge,  // User is competent — "what do you think?" first
    Verify,     // User is proficient — just verify their answer
    Defer,      // User is expert — they know more than the profile
}

/// The mentor's state — all tracked concepts.
#[derive(Debug, Default)]
pub struct MentorState {
    concepts: HashMap<String, TrackedConcept>,
}

impl MentorState {
    pub fn new() -> Self { Self::default() }

    /// Record that the user interacted with a concept.
    pub fn record_interaction(&mut self, concept: &str, domain: &str, correct: bool) {
        let entry = self.concepts.entry(concept.to_lowercase()).or_insert_with(|| {
            TrackedConcept {
                name: concept.into(), domain: domain.into(),
                interaction_count: 0, correct_answers: 0,
                last_reviewed: None, next_review: None,
                level: UnderstandingLevel::Novice,
            }
        });
        entry.interaction_count += 1;
        if correct { entry.correct_answers += 1; }
        entry.last_reviewed = Some(chrono::Utc::now().to_rfc3339());
        entry.level = compute_level(entry.interaction_count, entry.correct_answers);

        // Spaced repetition: schedule next review
        let interval_days = match entry.level {
            UnderstandingLevel::Novice => 1,
            UnderstandingLevel::Familiar => 3,
            UnderstandingLevel::Competent => 7,
            UnderstandingLevel::Proficient => 14,
            UnderstandingLevel::Expert => 30,
        };
        let next = chrono::Utc::now() + chrono::Duration::days(interval_days);
        entry.next_review = Some(next.to_rfc3339());
    }

    /// Get the teaching mode for a concept.
    pub fn teaching_mode(&self, concept: &str) -> TeachingMode {
        match self.concepts.get(&concept.to_lowercase()) {
            None => TeachingMode::Explain,
            Some(c) => match c.level {
                UnderstandingLevel::Novice => TeachingMode::Explain,
                UnderstandingLevel::Familiar => TeachingMode::Guide,
                UnderstandingLevel::Competent => TeachingMode::Challenge,
                UnderstandingLevel::Proficient => TeachingMode::Verify,
                UnderstandingLevel::Expert => TeachingMode::Defer,
            },
        }
    }

    /// Get concepts due for review (spaced repetition).
    pub fn due_for_review(&self) -> Vec<&TrackedConcept> {
        let now = chrono::Utc::now().to_rfc3339();
        self.concepts.values()
            .filter(|c| c.next_review.as_ref().map(|r| r.as_str() <= now.as_str()).unwrap_or(false))
            .collect()
    }

    /// Summary of knowledge progress.
    pub fn progress_summary(&self) -> String {
        if self.concepts.is_empty() {
            return "No concepts tracked yet.".into();
        }
        let mut by_level: HashMap<&str, usize> = HashMap::new();
        for c in self.concepts.values() {
            let label = match c.level {
                UnderstandingLevel::Novice => "Novice",
                UnderstandingLevel::Familiar => "Familiar",
                UnderstandingLevel::Competent => "Competent",
                UnderstandingLevel::Proficient => "Proficient",
                UnderstandingLevel::Expert => "Expert",
            };
            *by_level.entry(label).or_insert(0) += 1;
        }
        let due = self.due_for_review().len();
        let mut out = format!("{} concepts tracked", self.concepts.len());
        for (level, count) in &by_level {
            out.push_str(&format!(", {}: {}", level, count));
        }
        if due > 0 { out.push_str(&format!(". {} due for review", due)); }
        out
    }

    pub fn concept_count(&self) -> usize { self.concepts.len() }
}

/// Format teaching mode as a prompt injection.
pub fn format_for_prompt(concept: &str) -> Option<String> {
    let state = mentor_state().lock().ok()?;
    let mode = state.teaching_mode(concept);
    let instruction = match mode {
        TeachingMode::Explain => return None, // Default behavior
        TeachingMode::Guide => format!(
            "# Teaching Mode: Guided Discovery\n\
             The user has some familiarity with '{}'. \
             Don't give the full answer — ask guiding questions first.\n", concept),
        TeachingMode::Challenge => format!(
            "# Teaching Mode: Challenge\n\
             The user is competent with '{}'. \
             Ask 'What do you think?' BEFORE giving your answer. \
             Build on their response.\n", concept),
        TeachingMode::Verify => format!(
            "# Teaching Mode: Verification\n\
             The user is proficient with '{}'. \
             Let them lead — only correct if wrong.\n", concept),
        TeachingMode::Defer => format!(
            "# Teaching Mode: Expert Deference\n\
             The user is an expert on '{}'. \
             Ask for their perspective — they may teach YOU.\n", concept),
    };
    Some(instruction)
}

fn compute_level(interactions: u32, correct: u32) -> UnderstandingLevel {
    let accuracy = if interactions > 0 { correct as f64 / interactions as f64 } else { 0.0 };
    match interactions {
        0..=2 => UnderstandingLevel::Novice,
        3..=7 if accuracy >= 0.5 => UnderstandingLevel::Familiar,
        8..=15 if accuracy >= 0.6 => UnderstandingLevel::Competent,
        16..=30 if accuracy >= 0.7 => UnderstandingLevel::Proficient,
        _ if interactions > 30 && accuracy >= 0.8 => UnderstandingLevel::Expert,
        _ => UnderstandingLevel::Familiar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_novice_by_default() {
        let state = MentorState::new();
        assert_eq!(state.teaching_mode("rust"), TeachingMode::Explain);
    }

    #[test]
    fn test_level_progression() {
        let mut state = MentorState::new();
        for _ in 0..10 { state.record_interaction("ownership", "rust", true); }
        assert!(state.concepts["ownership"].level >= UnderstandingLevel::Competent);
        assert_eq!(state.teaching_mode("ownership"), TeachingMode::Challenge);
    }

    #[test]
    fn test_progress_summary() {
        let mut state = MentorState::new();
        state.record_interaction("rust", "dev", true);
        let summary = state.progress_summary();
        assert!(summary.contains("1 concepts tracked"));
    }

    #[test]
    fn test_spaced_repetition() {
        let mut state = MentorState::new();
        state.record_interaction("testing", "dev", true);
        assert!(state.concepts["testing"].next_review.is_some());
    }
}
