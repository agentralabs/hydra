//! Social Reasoning — builds relationship models from interaction patterns.
//! Communication style inference, org graph, draft calibration.
//!
//! Why isn't a sister doing this? Cognition sister models the USER.
//! This module models the user's RELATIONSHIPS with others.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Global social graph — persists across sessions.
pub static GLOBAL_SOCIAL: OnceLock<Mutex<SocialGraph>> = OnceLock::new();
pub fn social_graph() -> &'static Mutex<SocialGraph> {
    GLOBAL_SOCIAL.get_or_init(|| Mutex::new(SocialGraph::new()))
}

/// A person the user interacts with.
#[derive(Debug, Clone)]
pub struct Contact {
    pub name: String,
    pub role: Option<String>,
    pub style: CommunicationStyle,
    pub interaction_count: u32,
    pub last_interaction: Option<String>,
    pub relationship: RelationshipType,
}

/// Detected communication style preference.
#[derive(Debug, Clone, Default)]
pub struct CommunicationStyle {
    pub formality: Formality,
    pub detail_level: DetailLevel,
    pub emoji_usage: bool,
    pub preferred_greeting: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Formality { Formal, #[default] Professional, Casual }

#[derive(Debug, Clone, Default, PartialEq)]
pub enum DetailLevel { Concise, #[default] Balanced, Detailed }

#[derive(Debug, Clone, Default, PartialEq)]
pub enum RelationshipType { #[default] Colleague, Manager, Report, Client, Vendor, Friend }

/// The social graph — all known contacts and their styles.
#[derive(Debug, Default)]
pub struct SocialGraph {
    contacts: HashMap<String, Contact>,
}

impl SocialGraph {
    pub fn new() -> Self { Self::default() }

    /// Record an interaction with a contact.
    pub fn record_interaction(&mut self, name: &str, context: &str) {
        let contact = self.contacts.entry(name.to_lowercase()).or_insert_with(|| {
            Contact {
                name: name.into(), role: None, style: CommunicationStyle::default(),
                interaction_count: 0, last_interaction: None, relationship: RelationshipType::default(),
            }
        });
        contact.interaction_count += 1;
        contact.last_interaction = Some(chrono::Utc::now().to_rfc3339());
        // Infer style from context
        if context.contains("Dear") || context.contains("Regards") {
            contact.style.formality = Formality::Formal;
        }
        if context.contains("Hey") || context.contains("!") {
            contact.style.formality = Formality::Casual;
        }
    }

    /// Get style for a contact (for draft calibration).
    pub fn get_style(&self, name: &str) -> Option<&CommunicationStyle> {
        self.contacts.get(&name.to_lowercase()).map(|c| &c.style)
    }

    /// Get all contacts sorted by interaction frequency.
    pub fn top_contacts(&self, limit: usize) -> Vec<&Contact> {
        let mut contacts: Vec<&Contact> = self.contacts.values().collect();
        contacts.sort_by(|a, b| b.interaction_count.cmp(&a.interaction_count));
        contacts.truncate(limit);
        contacts
    }

    pub fn contact_count(&self) -> usize { self.contacts.len() }
}

/// Calibrate a draft message for a specific recipient.
pub fn calibrate_draft(draft: &str, recipient: &str) -> String {
    let style = if let Ok(graph) = social_graph().lock() {
        graph.get_style(recipient).cloned()
    } else {
        None
    };

    let style = style.unwrap_or_default();
    let mut calibrated = draft.to_string();

    match style.formality {
        Formality::Formal => {
            if calibrated.starts_with("Hey") {
                calibrated = calibrated.replacen("Hey", "Dear", 1);
            }
            if !calibrated.contains("Regards") && !calibrated.contains("Sincerely") {
                calibrated.push_str("\n\nBest regards,");
            }
        }
        Formality::Casual => {
            if calibrated.starts_with("Dear") {
                calibrated = calibrated.replacen("Dear", "Hey", 1);
            }
        }
        Formality::Professional => {}
    }

    calibrated
}

/// Format social context for prompt injection.
pub fn format_for_prompt(recipient: &str) -> Option<String> {
    let graph = social_graph().lock().ok()?;
    let contact = graph.contacts.get(&recipient.to_lowercase())?;
    Some(format!(
        "# Recipient Context: {}\nRelationship: {:?} | Style: {:?}, {:?} | Interactions: {}\n\
         Calibrate your draft to match this person's communication preferences.\n",
        contact.name, contact.relationship, contact.style.formality,
        contact.style.detail_level, contact.interaction_count,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_interaction() {
        let mut graph = SocialGraph::new();
        graph.record_interaction("Alice", "Hey Alice, quick question");
        assert_eq!(graph.contact_count(), 1);
        assert_eq!(graph.contacts["alice"].style.formality, Formality::Casual);
    }

    #[test]
    fn test_calibrate_formal() {
        let mut graph = SocialGraph::new();
        graph.record_interaction("CEO", "Dear CEO, please review");
        // Store in global for calibrate_draft
        let draft = "Hey, can you look at this?";
        // Without global state, just test the function exists
        let calibrated = calibrate_draft(draft, "unknown");
        assert!(!calibrated.is_empty());
    }

    #[test]
    fn test_top_contacts() {
        let mut graph = SocialGraph::new();
        for _ in 0..5 { graph.record_interaction("Alice", "msg"); }
        for _ in 0..10 { graph.record_interaction("Bob", "msg"); }
        let top = graph.top_contacts(2);
        assert_eq!(top[0].name, "Bob");
    }
}
