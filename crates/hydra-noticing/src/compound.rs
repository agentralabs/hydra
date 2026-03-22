//! CompoundRiskDetector — N small issues together = 1 large risk.
//! Each issue is fine alone. Together: a cascade forming.

use crate::{
    constants::COMPOUND_RISK_THRESHOLD,
    signal::{NoticingKind, NoticingSignal},
};
use serde::{Deserialize, Serialize};

/// A small issue that on its own is not significant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmallIssue {
    pub description: String,
    pub domain:      String,
    pub severity:    f64,
    pub observed_at: chrono::DateTime<chrono::Utc>,
}

impl SmallIssue {
    pub fn new(
        description: impl Into<String>,
        domain:      impl Into<String>,
        severity:    f64,
    ) -> Self {
        Self {
            description: description.into(),
            domain:      domain.into(),
            severity:    severity.clamp(0.0, 1.0),
            observed_at: chrono::Utc::now(),
        }
    }
}

/// Detects when small issues compound into a larger risk.
#[derive(Debug, Default)]
pub struct CompoundRiskDetector {
    issues: Vec<SmallIssue>,
}

impl CompoundRiskDetector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_issue(&mut self, issue: SmallIssue) {
        self.issues.push(issue);
        // Keep only recent issues (last 100)
        if self.issues.len() > 100 {
            self.issues.remove(0);
        }
    }

    /// Check if issues are compounding into a significant risk.
    pub fn check_compound(&self) -> Option<NoticingSignal> {
        if self.issues.len() < COMPOUND_RISK_THRESHOLD {
            return None;
        }

        // Find the most common domain
        let mut domain_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for issue in &self.issues {
            *domain_counts.entry(&issue.domain).or_insert(0) += 1;
        }

        let (top_domain, top_count) = domain_counts
            .iter()
            .max_by_key(|(_, count)| *count)?;

        if *top_count < COMPOUND_RISK_THRESHOLD {
            return None;
        }

        let avg_severity = self
            .issues
            .iter()
            .filter(|i| i.domain.as_str() == *top_domain)
            .map(|i| i.severity)
            .sum::<f64>()
            / *top_count as f64;

        let significance = (avg_severity * *top_count as f64 / 5.0).min(1.0);
        if significance < crate::constants::SIGNAL_SIGNIFICANCE_FLOOR {
            return None;
        }

        let descriptions: Vec<&str> = self
            .issues
            .iter()
            .filter(|i| i.domain.as_str() == *top_domain)
            .take(3)
            .map(|i| i.description.as_str())
            .collect();

        Some(NoticingSignal::new(
            NoticingKind::CompoundRisk {
                issue_count:  *top_count,
                shared_theme: top_domain.to_string(),
            },
            significance,
            format!(
                "Noticed: {} small issues in '{}' are compounding. \
                 Individually minor, together significant. \
                 Themes: {}.",
                top_count,
                top_domain,
                descriptions.join("; "),
            ),
            Some(format!(
                "Investigate {} domain holistically — {} small issues may indicate systemic problem.",
                top_domain, top_count
            )),
        ))
    }

    pub fn issue_count(&self) -> usize {
        self.issues.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_threshold_no_compound_signal() {
        let mut d = CompoundRiskDetector::new();
        d.add_issue(SmallIssue::new("minor issue 1", "engineering", 0.3));
        d.add_issue(SmallIssue::new("minor issue 2", "engineering", 0.3));
        assert!(d.check_compound().is_none());
    }

    #[test]
    fn at_threshold_compound_signal() {
        let mut d = CompoundRiskDetector::new();
        for i in 0..COMPOUND_RISK_THRESHOLD {
            d.add_issue(SmallIssue::new(
                format!("issue {}", i),
                "engineering",
                0.7,
            ));
        }
        let signal = d.check_compound();
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert!(matches!(s.kind, NoticingKind::CompoundRisk { .. }));
    }

    #[test]
    fn mixed_domains_finds_top_domain() {
        let mut d = CompoundRiskDetector::new();
        d.add_issue(SmallIssue::new("finance issue", "finance", 0.3));
        for i in 0..3 {
            d.add_issue(SmallIssue::new(
                format!("eng {}", i),
                "engineering",
                0.7,
            ));
        }
        let signal = d.check_compound();
        if let Some(s) = signal {
            assert!(matches!(
                s.kind,
                NoticingKind::CompoundRisk {
                    ref shared_theme, ..
                } if shared_theme == "engineering"
            ));
        }
    }
}
