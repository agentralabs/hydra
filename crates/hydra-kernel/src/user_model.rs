//! O21 Deep User Model — learns user patterns from every conversation.
//! Tracks productive hours, domain preferences, stress signals, communication style.
//! Proactive suggestions based on observed patterns over time.
//! Privacy: stored locally in ~/.hydra/user_model.json, never sent to cloud.

use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Types ──

/// Preferred response length (learned from user feedback).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseLength { Concise, Moderate, Detailed, Adaptive }

/// An inferred goal based on repeated topic patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredGoal {
    pub description: String,
    pub confidence: f64,
    pub first_detected: DateTime<Utc>,
}

/// The deep user model — learns passively from every conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepUserModel {
    pub activity_hours: [u64; 24],
    pub domains_frequency: HashMap<String, u64>,
    pub session_count: u64,
    pub preferred_length: ResponseLength,
    pub satisfaction_count: u64,
    pub frustration_count: u64,
    pub correction_count: u64,
    pub inferred_goals: Vec<InferredGoal>,
    pub last_updated: DateTime<Utc>,
}

impl DeepUserModel {
    pub fn new() -> Self {
        Self {
            activity_hours: [0; 24],
            domains_frequency: HashMap::new(),
            session_count: 0, preferred_length: ResponseLength::Adaptive,
            satisfaction_count: 0, frustration_count: 0, correction_count: 0,
            inferred_goals: Vec::new(), last_updated: Utc::now(),
        }
    }

    // ── Observation ──

    /// Learn from one conversation exchange.
    pub fn observe_exchange(&mut self, input: &str, _response: &str, domain: &str) {
        let now = Utc::now();
        self.last_updated = now;
        self.session_count += 1;

        // Record activity hour
        let hour = now.hour() as usize;
        if hour < 24 { self.activity_hours[hour] += 1; }

        // Record domain
        if !domain.is_empty() {
            *self.domains_frequency.entry(domain.to_string()).or_insert(0) += 1;
        }

        // Detect satisfaction/frustration from input
        let lower = input.to_lowercase();
        if lower.contains("perfect") || lower.contains("exactly") || lower.contains("great") {
            self.satisfaction_count += 1;
        }
        if lower.contains("no not") || lower.contains("wrong") || lower.contains("that's not") {
            self.frustration_count += 1;
            self.correction_count += 1;
        }
        if lower.contains("too long") || lower.contains("shorter") {
            self.preferred_length = ResponseLength::Concise;
        }
        if lower.contains("more detail") || lower.contains("elaborate") {
            self.preferred_length = ResponseLength::Detailed;
        }
    }

    // ── Queries ──

    /// Top N domains by frequency.
    pub fn top_domains(&self, n: usize) -> Vec<(&str, u64)> {
        let mut sorted: Vec<_> = self.domains_frequency.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    /// Hours with highest activity.
    pub fn peak_hours(&self) -> Vec<u8> {
        if self.activity_hours.iter().all(|h| *h == 0) { return vec![]; }
        let max = *self.activity_hours.iter().max().unwrap_or(&0);
        if max == 0 { return vec![]; }
        let threshold = max / 2;
        (0..24).filter(|h| self.activity_hours[*h as usize] > threshold).collect()
    }

    // ── Proactive Suggestions ──

    /// Generate suggestions based on observed patterns.
    pub fn proactive_suggestions(&self) -> Vec<String> {
        let mut suggestions = Vec::new();
        // Suggest peak hours
        let peaks = self.peak_hours();
        if peaks.len() >= 2 {
            suggestions.push(format!("Peak productive hours: {}:00-{}:00",
                peaks.first().unwrap(), peaks.last().unwrap()));
        }
        // High frustration rate
        if self.session_count > 20 && self.frustration_count > self.session_count / 5 {
            suggestions.push("Correction rate is high — I'm adjusting my approach".into());
        }
        // Domain trend
        for (domain, count) in self.top_domains(1) {
            if count > 10 {
                suggestions.push(format!("You focus heavily on '{domain}' — consider /immerse {domain}"));
            }
        }
        suggestions
    }

    /// Compact summary for TUI display.
    pub fn summary(&self) -> String {
        let peaks = self.peak_hours();
        let peak_str = if peaks.is_empty() { "unknown".into() }
            else { format!("{}:00-{}:00", peaks.first().unwrap(), peaks.last().unwrap()) };
        let top = self.top_domains(3);
        let top_str = if top.is_empty() { "none".into() }
            else { top.iter().map(|(d, c)| format!("{d}({c})")).collect::<Vec<_>>().join(", ") };
        format!("sessions={}, peak={}, top=[{}], pref={:?}",
            self.session_count, peak_str, top_str, self.preferred_length)
    }

    // ── Persistence ──

    /// Load from ~/.hydra/user_model.json.
    pub fn load() -> Self {
        let path = model_path();
        std::fs::read_to_string(&path).ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(Self::new)
    }

    /// Save to ~/.hydra/user_model.json.
    pub fn save(&self) {
        let path = model_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    eprintln!("hydra-user-model: save failed: {e}");
                }
            }
            Err(e) => eprintln!("hydra-user-model: serialize failed: {e}"),
        }
    }
}

impl Default for DeepUserModel {
    fn default() -> Self { Self::new() }
}

fn model_path() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/user_model.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_empty() {
        let m = DeepUserModel::new();
        assert_eq!(m.session_count, 0);
        assert!(m.domains_frequency.is_empty());
    }

    #[test]
    fn observe_records_hour() {
        let mut m = DeepUserModel::new();
        m.observe_exchange("hello", "hi", "engineering");
        let hour = Utc::now().hour() as usize;
        assert!(m.activity_hours[hour] > 0);
    }

    #[test]
    fn observe_records_domain() {
        let mut m = DeepUserModel::new();
        m.observe_exchange("test", "ok", "finance");
        assert_eq!(*m.domains_frequency.get("finance").unwrap(), 1);
    }

    #[test]
    fn top_domains_sorted() {
        let mut m = DeepUserModel::new();
        for _ in 0..5 { m.observe_exchange("a", "b", "engineering"); }
        for _ in 0..3 { m.observe_exchange("a", "b", "finance"); }
        let top = m.top_domains(2);
        assert_eq!(top[0].0, "engineering");
        assert_eq!(top[1].0, "finance");
    }

    #[test]
    fn satisfaction_detected() {
        let mut m = DeepUserModel::new();
        m.observe_exchange("perfect, exactly what I needed", "ok", "test");
        assert_eq!(m.satisfaction_count, 1);
    }
}
