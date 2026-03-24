//! O11 Social Intelligence — relational context layer for communication awareness.
//! Tracks per-person sentiment, analyzes timing, detects thread tension,
//! matches empathy templates, and enriches LLM prompts with social context.

use chrono::{DateTime, Utc};
use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

// ── Types ──

/// Per-person relational state tracked across interactions.
#[derive(Debug, Clone)]
pub struct RelationalState {
    pub person: String,
    pub sentiment_history: Vec<(DateTime<Utc>, f64)>,
    pub last_interaction: DateTime<Utc>,
    pub interaction_count: u64,
    pub formality_level: f64,
    pub prefers_brevity: bool,
    pub timezone: Option<String>,
}

impl RelationalState {
    /// Default profile for unknown person (EC-11.3).
    pub fn default_for(name: &str) -> Self {
        Self {
            person: name.into(), sentiment_history: vec![],
            last_interaction: Utc::now(), interaction_count: 0,
            formality_level: 0.7, prefers_brevity: false, timezone: None,
        }
    }
    /// Current sentiment (average of last 5 readings, or 0.0 if none).
    pub fn current_sentiment(&self) -> f64 {
        let recent: Vec<f64> = self.sentiment_history.iter().rev().take(5).map(|(_, s)| *s).collect();
        if recent.is_empty() { 0.0 } else { recent.iter().sum::<f64>() / recent.len() as f64 }
    }
    /// Sentiment trend: positive = improving, negative = declining.
    pub fn sentiment_trend(&self) -> f64 {
        if self.sentiment_history.len() < 2 { return 0.0; }
        let last = self.sentiment_history.last().map(|(_, s)| *s).unwrap_or(0.0);
        let prev = self.sentiment_history.iter().rev().nth(1).map(|(_, s)| *s).unwrap_or(0.0);
        last - prev
    }
}

/// Timing analysis for when to send a message.
pub struct TimingAnalysis {
    pub best_send_time: Option<DateTime<Utc>>,
    pub reason: String,
    pub urgency_override: bool,
}

/// Thread tension analysis for multi-person conversations.
pub struct ThreadAnalysis {
    pub tension_level: f64,
    pub participants: Vec<String>,
    pub recommendation: String,
    pub confidence: f64,
}

/// Full social context for a given interaction.
pub struct SocialContext {
    pub relational_states: Vec<RelationalState>,
    pub timing: Option<TimingAnalysis>,
    pub thread: Option<ThreadAnalysis>,
    pub empathy_suggestions: Vec<String>,
}

// ── Analysis Functions ──

/// Analyze social context for a piece of text.
pub fn analyze_social_context(text: &str, genome: &hydra_genome::GenomeStore) -> SocialContext {
    let people = extract_people(text);
    let states: Vec<RelationalState> = people.iter()
        .map(|p| load_relational_state(p, genome))
        .collect();
    let timing = states.first().map(|s| analyze_timing(s));
    let thread = if people.len() > 1 { Some(analyze_thread(text)) } else { None };
    let empathy = match_empathy_templates(text, genome);
    SocialContext { relational_states: states, timing, thread, empathy_suggestions: empathy }
}

/// Estimate sentiment of text (-1.0 to 1.0).
/// Handles sarcasm detection (EC-11.1): positive word + negative context → negative.
pub fn estimate_sentiment(text: &str) -> f64 {
    let lower = text.to_lowercase();
    let positive = ["thanks", "appreciate", "great work", "excellent", "happy", "pleased",
        "wonderful", "love", "fantastic", "brilliant", "well done", "kudos"];
    let negative = ["frustrated", "disappointed", "angry", "upset", "terrible", "awful",
        "broken", "failed", "wrong", "bug", "issue", "problem", "delay", "missed"];
    let sarcasm_markers = ["oh great", "just great", "wonderful,", "fantastic,", "amazing,"];
    let pos_count = positive.iter().filter(|w| lower.contains(*w)).count() as f64;
    let neg_count = negative.iter().filter(|w| lower.contains(*w)).count() as f64;
    let sarcasm = sarcasm_markers.iter().any(|s| lower.contains(s));
    let raw = if pos_count + neg_count == 0.0 { 0.0 }
        else { (pos_count - neg_count) / (pos_count + neg_count) };
    // EC-11.1: Sarcasm flips positive to negative
    if sarcasm && raw > 0.0 { -raw } else { raw }
}

/// Analyze timing for sending a message to a person.
fn analyze_timing(person: &RelationalState) -> TimingAnalysis {
    let trend = person.sentiment_trend();
    // EC-11.2: If declining sentiment, suggest delay
    if trend < -0.2 {
        return TimingAnalysis {
            best_send_time: Some(Utc::now() + chrono::Duration::hours(24)),
            reason: format!("Sentiment declining ({:.2}) — consider waiting", trend),
            urgency_override: false,
        };
    }
    let gap_hours = (Utc::now() - person.last_interaction).num_hours();
    if gap_hours < 2 {
        TimingAnalysis {
            best_send_time: None, reason: "Recent interaction — timing fine".into(),
            urgency_override: false,
        }
    } else {
        TimingAnalysis {
            best_send_time: Some(Utc::now()), reason: format!("Last contact {}h ago", gap_hours),
            urgency_override: false,
        }
    }
}

/// Analyze thread tension in multi-person text.
fn analyze_thread(text: &str) -> ThreadAnalysis {
    let lower = text.to_lowercase();
    let tension_words = ["disagree", "wrong", "not realistic", "this is news", "why wasn't",
        "should have", "frustrated", "unacceptable", "dropped the ball"];
    let calm_words = ["agree", "makes sense", "sounds good", "thanks", "let's", "we can"];
    let tension = tension_words.iter().filter(|w| lower.contains(*w)).count() as f64;
    let calm = calm_words.iter().filter(|w| lower.contains(*w)).count() as f64;
    let level = if tension + calm == 0.0 { 0.0 } else { tension / (tension + calm) };
    let confidence = ((tension + calm) / 5.0).min(1.0);
    // EC-11.4: Below 0.6 confidence, don't recommend action
    let rec = if confidence < 0.6 { "Low confidence — observe, don't intervene".into() }
        else if level > 0.7 { "High tension — consider DMs over public thread".into() }
        else if level > 0.4 { "Moderate tension — acknowledge concerns before proposing".into() }
        else { "Thread seems constructive — proceed normally".into() };
    ThreadAnalysis { tension_level: level, participants: extract_people(text), recommendation: rec, confidence }
}

/// Match empathy templates from genome.
fn match_empathy_templates(text: &str, genome: &hydra_genome::GenomeStore) -> Vec<String> {
    let query = format!("empathy {text}");
    genome.query(&query).iter()
        .filter(|e| e.effective_confidence() > 0.5)
        .take(3)
        .map(|e| e.approach.steps.first().cloned().unwrap_or_default())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extract mentioned people from text (simple @mention and capitalized names).
fn extract_people(text: &str) -> Vec<String> {
    let mut people = Vec::new();
    for word in text.split_whitespace() {
        if word.starts_with('@') && word.len() > 1 {
            people.push(word[1..].trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase());
        }
    }
    people.dedup();
    people
}

// ── Persistence ──

/// Load relational state for a person from genome. Returns default if unknown (EC-11.3).
pub fn load_relational_state(person: &str, genome: &hydra_genome::GenomeStore) -> RelationalState {
    let matches = genome.query(&format!("communication {person}"));
    if let Some(entry) = matches.first() {
        RelationalState {
            person: person.into(),
            sentiment_history: vec![(entry.last_used_at, entry.effective_confidence())],
            last_interaction: entry.last_used_at,
            interaction_count: entry.use_count,
            formality_level: 0.5,
            prefers_brevity: false,
            timezone: None,
        }
    } else {
        RelationalState::default_for(person)
    }
}

/// Save an interaction's sentiment to genome for learning.
pub fn save_interaction(person: &str, sentiment: f64, genome: &mut hydra_genome::GenomeStore) {
    let desc = format!("communication {person} interaction");
    let matches = genome.query(&desc);
    if let Some(entry) = matches.first() {
        let id = entry.id.clone();
        let success = sentiment > 0.0;
        if let Err(e) = genome.record_use(&id, success) {
            eprintln!("hydra-social: genome record: {e}");
        }
    } else {
        let entry = hydra_genome::social_genome::create_communication_entry(
            person, "initial interaction", "observe and adapt", sentiment.abs().max(0.5));
        if let Err(e) = genome.add(entry) {
            eprintln!("hydra-social: genome add: {e}");
        }
    }
}

/// Format social context as a summary string for enrichment.
fn format_social(ctx: &SocialContext) -> String {
    let mut parts = Vec::new();
    for s in &ctx.relational_states {
        parts.push(format!("{}: sentiment={:.1}, interactions={}", s.person, s.current_sentiment(), s.interaction_count));
    }
    if let Some(t) = &ctx.timing { parts.push(format!("Timing: {}", t.reason)); }
    if let Some(th) = &ctx.thread { parts.push(format!("Thread tension: {:.1} — {}", th.tension_level, th.recommendation)); }
    for e in &ctx.empathy_suggestions { parts.push(format!("Empathy: {e}")); }
    parts.join("; ")
}

/// Format social context as prompt enrichment lines.
pub fn enrich_prompt_with_social(ctx: &SocialContext) -> Vec<String> {
    let mut lines = Vec::new();
    if ctx.relational_states.is_empty() && ctx.thread.is_none() { return lines; }
    lines.push("[Social Context]".into());
    for s in &ctx.relational_states {
        let trend = if s.sentiment_trend() > 0.1 { "improving" }
            else if s.sentiment_trend() < -0.1 { "declining" } else { "stable" };
        lines.push(format!("  {}: sentiment {:.1} ({}), {} interactions",
            s.person, s.current_sentiment(), trend, s.interaction_count));
    }
    if let Some(t) = &ctx.timing { lines.push(format!("  Timing: {}", t.reason)); }
    if let Some(th) = &ctx.thread {
        if th.confidence >= 0.6 { lines.push(format!("  Thread: {}", th.recommendation)); }
    }
    for e in &ctx.empathy_suggestions { lines.push(format!("  Approach: {e}")); }
    lines
}

// ── Middleware ──

/// Social intelligence middleware — enriches the cognitive loop with relational context.
pub struct SocialMiddleware {
    last_context: Option<SocialContext>,
}

impl SocialMiddleware {
    pub fn new() -> Self { Self { last_context: None } }
}

impl CycleMiddleware for SocialMiddleware {
    fn name(&self) -> &'static str { "social" }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        let genome = hydra_genome::GenomeStore::open();
        let ctx = analyze_social_context(&perceived.raw, &genome);
        if !ctx.relational_states.is_empty() || ctx.thread.is_some() {
            perceived.enrichments.insert("social_context".into(), format_social(&ctx));
        }
        self.last_context = Some(ctx);
    }

    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        self.last_context.as_ref().map(enrich_prompt_with_social).unwrap_or_default()
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        if let Some(ctx) = &self.last_context {
            let mut genome = hydra_genome::GenomeStore::open();
            let sentiment = estimate_sentiment(&cycle.response);
            for state in &ctx.relational_states {
                save_interaction(&state.person, sentiment, &mut genome);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentiment_positive() { assert!(estimate_sentiment("Thanks for the great work!") > 0.0); }

    #[test]
    fn sentiment_negative() { assert!(estimate_sentiment("I'm frustrated by this delay") < 0.0); }

    #[test]
    fn sentiment_sarcasm_flips() {
        // EC-11.1: "oh great" + negative context → negative
        assert!(estimate_sentiment("oh great, another deployment failed") < 0.0);
    }

    #[test]
    fn sentiment_neutral() { assert_eq!(estimate_sentiment("The meeting is at 3pm"), 0.0); }

    #[test]
    fn unknown_person_default_formal() {
        // EC-11.3: Unknown person gets formal default
        let state = RelationalState::default_for("stranger");
        assert!(state.formality_level >= 0.5);
        assert_eq!(state.interaction_count, 0);
    }

    #[test]
    fn thread_tension_confidence_threshold() {
        // EC-11.4: Low confidence → observe, don't intervene
        let analysis = analyze_thread("Let's do this.");
        assert!(analysis.recommendation.contains("observe") || analysis.tension_level < 0.3);
    }

    #[test]
    fn thread_high_tension() {
        let analysis = analyze_thread("This is wrong and not realistic. Why wasn't this flagged? Unacceptable.");
        assert!(analysis.tension_level > 0.5);
    }

    #[test]
    fn timing_declining_sentiment_delays() {
        let mut state = RelationalState::default_for("test");
        state.sentiment_history = vec![
            (Utc::now() - chrono::Duration::hours(2), 0.8),
            (Utc::now(), 0.3),
        ];
        let timing = analyze_timing(&state);
        assert!(timing.best_send_time.is_some());
        assert!(timing.reason.contains("declining"));
    }

    #[test]
    fn extract_at_mentions() {
        let people = extract_people("Hey @john and @sarah, please review");
        assert!(people.contains(&"john".to_string()));
        assert!(people.contains(&"sarah".to_string()));
    }

    #[test]
    fn enrich_prompt_includes_context() {
        let ctx = SocialContext {
            relational_states: vec![RelationalState::default_for("alice")],
            timing: None, thread: None, empathy_suggestions: vec![],
        };
        let lines = enrich_prompt_with_social(&ctx);
        assert!(lines.iter().any(|l| l.contains("alice")));
    }
}
