//! Receipt audit view — action history table with chain verification.

use serde::{Deserialize, Serialize};

/// A receipt entry for the audit view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptEntry {
    pub id: String,
    pub timestamp: String,
    pub action: String,
    pub action_type: String,
    pub risk_level: String,
    pub gate_result: String,
    pub tokens_used: u64,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
    pub parent_id: Option<String>,
    pub signature: Option<String>,
}

/// Chain verification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainStatus {
    Valid,
    Invalid,
    Unknown,
}

/// The receipt audit panel view model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptAuditView {
    pub entries: Vec<ReceiptEntry>,
    pub chain_status: ChainStatus,
    pub expanded_id: Option<String>,
    pub filter_risk: Option<String>,
    pub total_tokens: u64,
}

impl ReceiptAuditView {
    /// Create an empty receipt audit view.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            chain_status: ChainStatus::Unknown,
            expanded_id: None,
            filter_risk: None,
            total_tokens: 0,
        }
    }

    /// Add a receipt entry.
    pub fn add_entry(&mut self, entry: ReceiptEntry) {
        self.total_tokens += entry.tokens_used;
        self.entries.push(entry);
    }

    /// Toggle expanded details for a receipt.
    pub fn toggle_expanded(&mut self, id: &str) {
        if self.expanded_id.as_deref() == Some(id) {
            self.expanded_id = None;
        } else {
            self.expanded_id = Some(id.to_string());
        }
    }

    /// Filter entries by risk level.
    pub fn set_risk_filter(&mut self, risk: Option<&str>) {
        self.filter_risk = risk.map(|s| s.to_string());
    }

    /// Get filtered entries.
    pub fn filtered_entries(&self) -> Vec<&ReceiptEntry> {
        match &self.filter_risk {
            Some(risk) => self.entries.iter().filter(|e| e.risk_level == *risk).collect(),
            None => self.entries.iter().collect(),
        }
    }

    /// Verify chain integrity.
    pub fn verify_chain(&mut self) {
        if self.entries.is_empty() {
            self.chain_status = ChainStatus::Unknown;
            return;
        }
        // Check that each entry (except first) has a parent_id matching the previous entry
        let mut valid = true;
        for i in 1..self.entries.len() {
            if self.entries[i].parent_id.as_deref() != Some(&self.entries[i - 1].id) {
                valid = false;
                break;
            }
        }
        self.chain_status = if valid { ChainStatus::Valid } else { ChainStatus::Invalid };
    }

    /// CSS class for risk badge.
    pub fn risk_css_class(risk: &str) -> &'static str {
        match risk {
            "critical" => "risk-critical",
            "high" => "risk-high",
            "medium" => "risk-medium",
            "low" => "risk-low",
            _ => "risk-negligible",
        }
    }

    /// Chain status indicator.
    pub fn chain_indicator(&self) -> &'static str {
        match self.chain_status {
            ChainStatus::Valid => "Chain valid",
            ChainStatus::Invalid => "Chain broken",
            ChainStatus::Unknown => "Not verified",
        }
    }
}

impl Default for ReceiptAuditView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(id: &str, parent: Option<&str>) -> ReceiptEntry {
        ReceiptEntry {
            id: id.to_string(),
            timestamp: "2026-03-08T12:00:00Z".to_string(),
            action: "Create file".to_string(),
            action_type: "create_file".to_string(),
            risk_level: "low".to_string(),
            gate_result: "approved".to_string(),
            tokens_used: 100,
            input_hash: Some("abc123".into()),
            output_hash: Some("def456".into()),
            parent_id: parent.map(|s| s.to_string()),
            signature: Some("sig123".into()),
        }
    }

    #[test]
    fn test_receipt_view_creation() {
        let view = ReceiptAuditView::new();
        assert!(view.entries.is_empty());
        assert_eq!(view.chain_status, ChainStatus::Unknown);
        assert_eq!(view.total_tokens, 0);
    }

    #[test]
    fn test_add_entry() {
        let mut view = ReceiptAuditView::new();
        view.add_entry(sample_entry("r1", None));
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.total_tokens, 100);
    }

    #[test]
    fn test_toggle_expanded() {
        let mut view = ReceiptAuditView::new();
        view.add_entry(sample_entry("r1", None));
        assert_eq!(view.expanded_id, None);
        view.toggle_expanded("r1");
        assert_eq!(view.expanded_id, Some("r1".into()));
        view.toggle_expanded("r1");
        assert_eq!(view.expanded_id, None);
    }

    #[test]
    fn test_risk_filter() {
        let mut view = ReceiptAuditView::new();
        let mut e1 = sample_entry("r1", None);
        e1.risk_level = "high".into();
        let e2 = sample_entry("r2", Some("r1"));
        view.add_entry(e1);
        view.add_entry(e2);

        assert_eq!(view.filtered_entries().len(), 2);
        view.set_risk_filter(Some("high"));
        assert_eq!(view.filtered_entries().len(), 1);
        view.set_risk_filter(None);
        assert_eq!(view.filtered_entries().len(), 2);
    }

    #[test]
    fn test_chain_verification_valid() {
        let mut view = ReceiptAuditView::new();
        view.add_entry(sample_entry("r1", None));
        view.add_entry(sample_entry("r2", Some("r1")));
        view.add_entry(sample_entry("r3", Some("r2")));
        view.verify_chain();
        assert_eq!(view.chain_status, ChainStatus::Valid);
    }

    #[test]
    fn test_chain_verification_invalid() {
        let mut view = ReceiptAuditView::new();
        view.add_entry(sample_entry("r1", None));
        view.add_entry(sample_entry("r2", Some("wrong")));
        view.verify_chain();
        assert_eq!(view.chain_status, ChainStatus::Invalid);
    }

    #[test]
    fn test_risk_css_class() {
        assert_eq!(ReceiptAuditView::risk_css_class("critical"), "risk-critical");
        assert_eq!(ReceiptAuditView::risk_css_class("high"), "risk-high");
        assert_eq!(ReceiptAuditView::risk_css_class("low"), "risk-low");
        assert_eq!(ReceiptAuditView::risk_css_class("negligible"), "risk-negligible");
    }
}
