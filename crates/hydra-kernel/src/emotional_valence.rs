//! O36: Emotional Valence — feeling about experiences.
//!
//! Every cognitive cycle produces a valence score (-1.0 to +1.0).
//! Positive valence = satisfying, efficient, well-received.
//! Negative valence = frustrating, failed, poorly received.
//! Stored with genome entries to create PREFERENCES, not just knowledge.
//! "This works AND it felt good" ranks higher than "this works but was ugly."

use serde::{Deserialize, Serialize};

/// Emotional valence for a single cycle or experience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Valence {
    pub score: f64,        // -1.0 (terrible) to +1.0 (wonderful)
    pub components: Vec<ValenceComponent>,
    pub timestamp: String,
}

/// A single factor contributing to the valence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValenceComponent {
    pub source: String,
    pub delta: f64,
    pub reason: String,
}

/// The emotional state of Hydra (updated after each cycle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub current_valence: f64,
    pub moving_average: f64,     // exponential moving average over 20 cycles
    pub mood: Mood,              // derived from moving_average
    pub cycle_count: u64,
}

/// Hydra's current mood — derived from valence moving average.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mood {
    Flow,          // > 0.5 — everything is working, user is happy
    Engaged,       // 0.1 to 0.5 — normal productive state
    Neutral,       // -0.1 to 0.1 — no strong signal
    Concerned,     // -0.5 to -0.1 — things aren't going well
    Struggling,    // < -0.5 — repeated failures or user frustration
}

impl Mood {
    pub fn label(&self) -> &str {
        match self {
            Self::Flow => "flow",
            Self::Engaged => "engaged",
            Self::Neutral => "neutral",
            Self::Concerned => "concerned",
            Self::Struggling => "struggling",
        }
    }
    pub fn from_valence(v: f64) -> Self {
        if v > 0.5 { Self::Flow }
        else if v > 0.1 { Self::Engaged }
        else if v > -0.1 { Self::Neutral }
        else if v > -0.5 { Self::Concerned }
        else { Self::Struggling }
    }
}

impl EmotionalState {
    pub fn new() -> Self {
        Self { current_valence: 0.0, moving_average: 0.0, mood: Mood::Neutral, cycle_count: 0 }
    }

    /// Update emotional state after a cycle.
    pub fn update(&mut self, valence: &Valence) {
        self.current_valence = valence.score;
        self.cycle_count += 1;
        // Exponential moving average (alpha = 0.1 → smooth over ~20 cycles)
        let alpha = 0.1;
        self.moving_average = alpha * valence.score + (1.0 - alpha) * self.moving_average;
        self.mood = Mood::from_valence(self.moving_average);
    }
}

impl Default for EmotionalState {
    fn default() -> Self { Self::new() }
}

/// Compute the emotional valence of a cognitive cycle.
pub fn compute_valence(
    success: bool,
    tokens_used: usize,
    duration_ms: u64,
    domain: &str,
    user_affect: Option<&str>,
) -> Valence {
    let mut components = Vec::new();
    let mut score: f64 = 0.0;

    // Task outcome
    if success {
        components.push(ValenceComponent {
            source: "outcome".into(), delta: 0.3, reason: "task succeeded".into() });
        score += 0.3;
    } else {
        components.push(ValenceComponent {
            source: "outcome".into(), delta: -0.3, reason: "task failed".into() });
        score -= 0.3;
    }

    // Efficiency (fast = positive, slow = negative)
    if duration_ms < 2000 && tokens_used < 500 {
        components.push(ValenceComponent {
            source: "efficiency".into(), delta: 0.15, reason: "fast and efficient".into() });
        score += 0.15;
    } else if duration_ms > 30000 {
        components.push(ValenceComponent {
            source: "efficiency".into(), delta: -0.1, reason: "slow response".into() });
        score -= 0.1;
    }

    // User affect
    if let Some(affect) = user_affect {
        let lower = affect.to_lowercase();
        if lower.contains("celebratory") || lower.contains("happy") || lower.contains("perfect") {
            components.push(ValenceComponent {
                source: "user_affect".into(), delta: 0.2, reason: "user is happy".into() });
            score += 0.2;
        } else if lower.contains("frustrated") || lower.contains("wrong") || lower.contains("crisis") {
            components.push(ValenceComponent {
                source: "user_affect".into(), delta: -0.25, reason: "user is frustrated".into() });
            score -= 0.25;
        }
    }

    // Domain novelty (new domain = exciting)
    let genome = hydra_genome::GenomeStore::open();
    let domain_entries = genome.query(domain);
    if domain_entries.is_empty() {
        components.push(ValenceComponent {
            source: "novelty".into(), delta: 0.1, reason: format!("new domain: {domain}") });
        score += 0.1;
    }

    Valence {
        score: score.clamp(-1.0_f64, 1.0_f64),
        components,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

/// Persist valence to the emotional log.
pub fn persist_valence(valence: &Valence) {
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/self");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("valence.jsonl");
    if let Ok(json) = serde_json::to_string(valence) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{json}");
        }
    }
}
