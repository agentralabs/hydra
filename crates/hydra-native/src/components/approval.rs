//! Approval card component data for risk-based action confirmation.

use serde::{Deserialize, Serialize};

/// Risk classification for an action requiring approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// View model for an approval card at a given risk level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalCard {
    pub risk_level: RiskLevel,
    pub title: String,
    pub description: String,
    pub preview: Option<String>,
    pub primary_action: String,
    pub secondary_action: String,
    pub challenge_phrase: Option<String>,
    pub icon: String,
    pub color: String,
    pub auto_approved: bool,
}

impl ApprovalCard {
    /// Auto-approved toast for low-risk actions.
    pub fn low(title: &str, description: &str) -> Self {
        Self {
            risk_level: RiskLevel::Low,
            title: title.to_owned(),
            description: description.to_owned(),
            preview: None,
            primary_action: "OK".into(),
            secondary_action: "Undo".into(),
            challenge_phrase: None,
            icon: "i".into(),
            color: "#4CAF50".into(),
            auto_approved: true,
        }
    }

    /// Quick confirmation for medium-risk actions.
    pub fn medium(title: &str, description: &str) -> Self {
        Self {
            risk_level: RiskLevel::Medium,
            title: title.to_owned(),
            description: description.to_owned(),
            preview: None,
            primary_action: "Approve".into(),
            secondary_action: "Cancel".into(),
            challenge_phrase: None,
            icon: "?".into(),
            color: "#FF9800".into(),
            auto_approved: false,
        }
    }

    /// Full preview card for high-risk actions.
    pub fn high(title: &str, description: &str, preview: &str) -> Self {
        Self {
            risk_level: RiskLevel::High,
            title: title.to_owned(),
            description: description.to_owned(),
            preview: Some(preview.to_owned()),
            primary_action: "Confirm".into(),
            secondary_action: "Reject".into(),
            challenge_phrase: None,
            icon: "!".into(),
            color: "#F44336".into(),
            auto_approved: false,
        }
    }

    /// Challenge-phrase card for critical-risk actions.
    pub fn critical(title: &str, description: &str, challenge: &str) -> Self {
        Self {
            risk_level: RiskLevel::Critical,
            title: title.to_owned(),
            description: description.to_owned(),
            preview: None,
            primary_action: "Execute".into(),
            secondary_action: "Abort".into(),
            challenge_phrase: Some(challenge.to_owned()),
            icon: "X".into(),
            color: "#B71C1C".into(),
            auto_approved: false,
        }
    }

    /// Verify a user's typed challenge phrase (case-insensitive).
    pub fn verify_challenge(&self, input: &str) -> bool {
        match &self.challenge_phrase {
            Some(phrase) => phrase.eq_ignore_ascii_case(input.trim()),
            None => true, // no challenge required
        }
    }

    /// Whether this card is auto-approved (no user interaction needed).
    pub fn is_auto_approved(&self) -> bool {
        self.auto_approved
    }

    /// Whether this card requires a challenge phrase.
    pub fn needs_challenge(&self) -> bool {
        self.challenge_phrase.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_risk_auto_approved() {
        let card = ApprovalCard::low("Read file", "Reading config.toml");
        assert_eq!(card.risk_level, RiskLevel::Low);
        assert!(card.is_auto_approved());
        assert!(!card.needs_challenge());
    }

    #[test]
    fn test_medium_risk() {
        let card = ApprovalCard::medium("Install package", "npm install lodash");
        assert_eq!(card.risk_level, RiskLevel::Medium);
        assert!(!card.is_auto_approved());
        assert!(!card.needs_challenge());
    }

    #[test]
    fn test_high_risk_with_preview() {
        let card = ApprovalCard::high("Delete branch", "Removing feature-x", "git branch -D feature-x");
        assert_eq!(card.risk_level, RiskLevel::High);
        assert!(card.preview.is_some());
        assert!(!card.is_auto_approved());
    }

    #[test]
    fn test_critical_risk_needs_challenge() {
        let card = ApprovalCard::critical("Force push", "Overwriting main", "force push main");
        assert_eq!(card.risk_level, RiskLevel::Critical);
        assert!(card.needs_challenge());
        assert!(!card.is_auto_approved());
    }

    #[test]
    fn test_challenge_verification_correct() {
        let card = ApprovalCard::critical("Drop DB", "Dropping production", "drop production");
        assert!(card.verify_challenge("drop production"));
        assert!(card.verify_challenge("DROP PRODUCTION"));
        assert!(card.verify_challenge("  drop production  "));
    }

    #[test]
    fn test_challenge_verification_incorrect() {
        let card = ApprovalCard::critical("Drop DB", "Dropping production", "drop production");
        assert!(!card.verify_challenge("delete production"));
        assert!(!card.verify_challenge(""));
    }

    #[test]
    fn test_no_challenge_always_verifies() {
        let card = ApprovalCard::low("Read", "Reading");
        assert!(card.verify_challenge("anything"));
    }

    #[test]
    fn test_low_card_fields() {
        let card = ApprovalCard::low("Read file", "Reading config.toml");
        assert_eq!(card.title, "Read file");
        assert_eq!(card.description, "Reading config.toml");
        assert_eq!(card.primary_action, "OK");
        assert_eq!(card.secondary_action, "Undo");
        assert_eq!(card.icon, "i");
        assert_eq!(card.color, "#4CAF50");
        assert!(card.preview.is_none());
    }

    #[test]
    fn test_medium_card_fields() {
        let card = ApprovalCard::medium("Install", "npm install lodash");
        assert_eq!(card.primary_action, "Approve");
        assert_eq!(card.secondary_action, "Cancel");
        assert_eq!(card.icon, "?");
        assert_eq!(card.color, "#FF9800");
    }

    #[test]
    fn test_high_card_fields() {
        let card = ApprovalCard::high("Delete", "Removing", "git branch -D feature");
        assert_eq!(card.primary_action, "Confirm");
        assert_eq!(card.secondary_action, "Reject");
        assert_eq!(card.icon, "!");
        assert_eq!(card.color, "#F44336");
        assert_eq!(card.preview.as_deref(), Some("git branch -D feature"));
    }

    #[test]
    fn test_critical_card_fields() {
        let card = ApprovalCard::critical("Force push", "Overwriting main", "force push main");
        assert_eq!(card.primary_action, "Execute");
        assert_eq!(card.secondary_action, "Abort");
        assert_eq!(card.icon, "X");
        assert_eq!(card.color, "#B71C1C");
        assert_eq!(card.challenge_phrase.as_deref(), Some("force push main"));
    }

    #[test]
    fn test_challenge_partial_match_fails() {
        let card = ApprovalCard::critical("Drop", "Dropping", "drop production");
        assert!(!card.verify_challenge("drop"));
        assert!(!card.verify_challenge("production"));
        assert!(!card.verify_challenge("drop prod"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let card = ApprovalCard::critical("Drop DB", "Dropping production", "drop production");
        let json = serde_json::to_string(&card).unwrap();
        let back: ApprovalCard = serde_json::from_str(&json).unwrap();
        assert_eq!(back.risk_level, RiskLevel::Critical);
        assert_eq!(back.title, "Drop DB");
        assert!(back.needs_challenge());
        assert!(back.verify_challenge("drop production"));
    }

    #[test]
    fn test_risk_level_serialization() {
        let levels = [RiskLevel::Low, RiskLevel::Medium, RiskLevel::High, RiskLevel::Critical];
        for level in &levels {
            let json = serde_json::to_string(level).unwrap();
            let back: RiskLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(*level, back);
        }
    }

    #[test]
    fn test_only_low_is_auto_approved() {
        assert!(ApprovalCard::low("a", "b").is_auto_approved());
        assert!(!ApprovalCard::medium("a", "b").is_auto_approved());
        assert!(!ApprovalCard::high("a", "b", "c").is_auto_approved());
        assert!(!ApprovalCard::critical("a", "b", "c").is_auto_approved());
    }

    #[test]
    fn test_only_critical_needs_challenge() {
        assert!(!ApprovalCard::low("a", "b").needs_challenge());
        assert!(!ApprovalCard::medium("a", "b").needs_challenge());
        assert!(!ApprovalCard::high("a", "b", "c").needs_challenge());
        assert!(ApprovalCard::critical("a", "b", "c").needs_challenge());
    }
}
