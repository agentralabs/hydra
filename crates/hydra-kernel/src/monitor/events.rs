//! O16 Monitor Events — unified event types, classifier, and sensitive data redaction.

use chrono::{DateTime, Utc};

// ── Types ──

/// Priority level for monitor events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPriority {
    /// Requires immediate attention.
    Alert,
    /// Should see soon.
    Important,
    /// Good to know.
    Informational,
    /// Filtered out unless requested.
    Noise,
}

impl EventPriority {
    pub fn symbol(&self) -> &'static str {
        match self { Self::Alert => "▲", Self::Important => "●", Self::Informational => "○", Self::Noise => "·" }
    }
    pub fn label(&self) -> &'static str {
        match self { Self::Alert => "alert", Self::Important => "important", Self::Informational => "info", Self::Noise => "noise" }
    }
}

/// Category of the event source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventCategory {
    CiCd,
    Communication,
    Security,
    Infrastructure,
    Project,
    Calendar,
    Custom(String),
}

impl EventCategory {
    pub fn label(&self) -> &str {
        match self {
            Self::CiCd => "ci/cd", Self::Communication => "comm", Self::Security => "security",
            Self::Infrastructure => "infra", Self::Project => "project",
            Self::Calendar => "calendar", Self::Custom(s) => s,
        }
    }
}

/// A unified monitor event from any source.
#[derive(Debug, Clone)]
pub struct MonitorEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub category: EventCategory,
    pub priority: EventPriority,
    pub title: String,
    pub detail: String,
    pub actionable: bool,
}

impl MonitorEvent {
    pub fn new(source: &str, category: EventCategory, title: &str, detail: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            timestamp: Utc::now(),
            source: source.to_string(),
            category,
            priority: EventPriority::Informational, // Will be classified
            title: title.to_string(),
            detail: redact_sensitive(detail),
            actionable: false,
        }
    }
    pub fn format_brief(&self) -> String {
        format!("{} {} [{}] {}", self.priority.symbol(), self.timestamp.format("%H:%M"),
            self.source, self.title)
    }
}

// ── Classifier ──

/// Classify event priority based on category, content, and genome patterns.
pub fn classify_event(
    category: &EventCategory,
    detail: &str,
    genome: &hydra_genome::GenomeStore,
) -> EventPriority {
    // Security events are always Alert
    if *category == EventCategory::Security { return EventPriority::Alert; }
    // Infrastructure failures
    let lower = detail.to_lowercase();
    if *category == EventCategory::Infrastructure
        && (lower.contains("fail") || lower.contains("down") || lower.contains("critical"))
    { return EventPriority::Alert; }
    // CI failures on main
    if *category == EventCategory::CiCd && lower.contains("main") && lower.contains("fail") {
        return EventPriority::Alert;
    }
    // Genome-based: check if user previously engaged with similar events
    let query = format!("monitor event {}", detail.chars().take(50).collect::<String>());
    let matches = genome.query(&query);
    if let Some(entry) = matches.first() {
        if entry.effective_confidence() > 0.7 { return EventPriority::Important; }
        if entry.effective_confidence() < 0.3 { return EventPriority::Noise; }
    }
    EventPriority::Informational
}

/// EC-16.9: Redact sensitive data from event text.
pub fn redact_sensitive(text: &str) -> String {
    let mut result = text.to_string();
    // Patterns: sk-*, AKIA*, ghp_*, passwords
    for pattern in &["sk-", "AKIA", "ghp_", "password=", "token=", "secret="] {
        if let Some(start) = result.find(pattern) {
            let end = result[start..].find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map(|i| start + i)
                .unwrap_or(result.len());
            let visible = &result[start..start + pattern.len().min(4)];
            let masked = format!("{visible}****");
            result.replace_range(start..end, &masked);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_always_alert() {
        let genome = hydra_genome::GenomeStore::new();
        let p = classify_event(&EventCategory::Security, "login attempt", &genome);
        assert_eq!(p, EventPriority::Alert);
    }

    #[test]
    fn infra_failure_alert() {
        let genome = hydra_genome::GenomeStore::new();
        let p = classify_event(&EventCategory::Infrastructure, "server down", &genome);
        assert_eq!(p, EventPriority::Alert);
    }

    #[test]
    fn default_informational() {
        let genome = hydra_genome::GenomeStore::new();
        let p = classify_event(&EventCategory::Calendar, "meeting at 3pm", &genome);
        assert_eq!(p, EventPriority::Informational);
    }

    #[test]
    fn redact_api_keys() {
        let text = "Error with sk-abc123xyz token";
        let redacted = redact_sensitive(text);
        assert!(redacted.contains("sk-****"), "Got: {redacted}");
        assert!(!redacted.contains("abc123xyz"));
    }

    #[test]
    fn event_format_brief() {
        let mut e = MonitorEvent::new("github", EventCategory::CiCd, "CI failed", "build error");
        e.priority = EventPriority::Alert;
        let brief = e.format_brief();
        assert!(brief.contains("▲"));
        assert!(brief.contains("github"));
    }
}
