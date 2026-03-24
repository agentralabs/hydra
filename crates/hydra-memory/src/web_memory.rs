//! WebsiteMemory — procedural memory for website navigation.
//! Stores navigation paths, login flows, and form locations per domain.
//! First visit: discover everything. Second visit: replay known steps.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A recorded navigation step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationStep {
    pub action: String,
    pub selector: Option<String>,
    pub url: Option<String>,
    pub wait_ms: u64,
    pub notes: String,
}

/// A recorded login flow for a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginFlow {
    pub domain: String,
    pub steps: Vec<NavigationStep>,
    pub has_2fa: bool,
    pub last_success: DateTime<Utc>,
    pub success_count: u32,
}

/// A recorded form location on a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormLocation {
    pub domain: String,
    pub path: String,
    pub form_type: String,
    pub field_selectors: HashMap<String, String>,
}

/// Website visit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisitRecord {
    pub domain: String,
    pub path: String,
    pub page_type: String,
    pub elements_found: usize,
    pub visited_at: DateTime<Utc>,
}

/// Procedural memory for web navigation.
pub struct WebsiteMemory {
    visits: HashMap<String, Vec<VisitRecord>>,
    login_flows: HashMap<String, LoginFlow>,
    form_locations: HashMap<String, Vec<FormLocation>>,
    navigation_cache: HashMap<String, Vec<NavigationStep>>,
}

impl WebsiteMemory {
    pub fn new() -> Self {
        Self {
            visits: HashMap::new(),
            login_flows: HashMap::new(),
            form_locations: HashMap::new(),
            navigation_cache: HashMap::new(),
        }
    }

    /// Record a website visit.
    pub fn record_visit(&mut self, record: VisitRecord) {
        let domain = record.domain.clone();
        eprintln!(
            "hydra-memory: recorded visit to {} ({})",
            domain, record.page_type
        );
        self.visits.entry(domain).or_default().push(record);
    }

    /// Record a successful login flow.
    pub fn record_login_flow(&mut self, flow: LoginFlow) {
        eprintln!(
            "hydra-memory: recorded login flow for {} ({} steps)",
            flow.domain,
            flow.steps.len()
        );
        self.login_flows.insert(flow.domain.clone(), flow);
    }

    /// Record a form location.
    pub fn record_form(&mut self, form: FormLocation) {
        self.form_locations
            .entry(form.domain.clone())
            .or_default()
            .push(form);
    }

    /// Cache a navigation path for a goal.
    pub fn cache_navigation(&mut self, goal_key: &str, steps: Vec<NavigationStep>) {
        self.navigation_cache.insert(goal_key.into(), steps);
    }

    /// Recall navigation steps for a goal (returns None if not cached).
    pub fn recall_navigation(&self, goal_key: &str) -> Option<&Vec<NavigationStep>> {
        self.navigation_cache.get(goal_key)
    }

    /// Recall the login flow for a domain.
    pub fn recall_login_flow(&self, domain: &str) -> Option<&LoginFlow> {
        self.login_flows.get(domain)
    }

    /// Recall form locations for a domain.
    pub fn recall_forms(&self, domain: &str) -> Vec<&FormLocation> {
        self.form_locations
            .get(domain)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// How many times have we visited this domain?
    pub fn visit_count(&self, domain: &str) -> usize {
        self.visits.get(domain).map(|v| v.len()).unwrap_or(0)
    }

    /// Get all known domains.
    pub fn known_domains(&self) -> Vec<&str> {
        let mut domains: Vec<&str> = self.visits.keys().map(|s| s.as_str()).collect();
        domains.sort();
        domains.dedup();
        domains
    }

    pub fn total_visits(&self) -> usize {
        self.visits.values().map(|v| v.len()).sum()
    }

    pub fn total_login_flows(&self) -> usize {
        self.login_flows.len()
    }
}

impl Default for WebsiteMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_recall_visit() {
        let mut mem = WebsiteMemory::new();
        mem.record_visit(VisitRecord {
            domain: "example.com".into(),
            path: "/".into(),
            page_type: "article".into(),
            elements_found: 15,
            visited_at: Utc::now(),
        });
        assert_eq!(mem.visit_count("example.com"), 1);
        assert_eq!(mem.total_visits(), 1);
    }

    #[test]
    fn record_and_recall_login_flow() {
        let mut mem = WebsiteMemory::new();
        mem.record_login_flow(LoginFlow {
            domain: "twitter.com".into(),
            steps: vec![NavigationStep {
                action: "type".into(),
                selector: Some("#email".into()),
                url: None,
                wait_ms: 0,
                notes: "enter email".into(),
            }],
            has_2fa: true,
            last_success: Utc::now(),
            success_count: 1,
        });
        let flow = mem.recall_login_flow("twitter.com");
        assert!(flow.is_some());
        assert!(flow.unwrap().has_2fa);
    }

    #[test]
    fn cache_and_recall_navigation() {
        let mut mem = WebsiteMemory::new();
        mem.cache_navigation("post_tweet", vec![
            NavigationStep {
                action: "navigate".into(),
                selector: None,
                url: Some("https://twitter.com".into()),
                wait_ms: 2000,
                notes: "open twitter".into(),
            },
        ]);
        let steps = mem.recall_navigation("post_tweet");
        assert!(steps.is_some());
        assert_eq!(steps.unwrap().len(), 1);
    }

    #[test]
    fn unknown_domain_returns_zero() {
        let mem = WebsiteMemory::new();
        assert_eq!(mem.visit_count("unknown.com"), 0);
        assert!(mem.recall_login_flow("unknown.com").is_none());
    }

    #[test]
    fn known_domains_sorted() {
        let mut mem = WebsiteMemory::new();
        mem.record_visit(VisitRecord {
            domain: "z.com".into(),
            path: "/".into(),
            page_type: "feed".into(),
            elements_found: 5,
            visited_at: Utc::now(),
        });
        mem.record_visit(VisitRecord {
            domain: "a.com".into(),
            path: "/".into(),
            page_type: "article".into(),
            elements_found: 3,
            visited_at: Utc::now(),
        });
        let domains = mem.known_domains();
        assert_eq!(domains, vec!["a.com", "z.com"]);
    }
}
