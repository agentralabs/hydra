//! Domain detection via vocabulary matching.
//!
//! Maps input text to one or more knowledge domains using keyword
//! vocabularies. No LLM calls — pure vocabulary intersection.

use crate::constants::{DOMAIN_VOCAB_MATCH_THRESHOLD, MAX_DETECTED_DOMAINS};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A knowledge domain that an input may belong to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    /// Software engineering, infrastructure, development.
    Engineering,
    /// Finance, budgets, markets, accounting.
    Finance,
    /// Security, authentication, threats, vulnerabilities.
    Security,
    /// Personal tasks, reminders, life management.
    Personal,
    /// Data engineering, analytics, pipelines.
    Data,
    /// Research, experimentation, academic topics.
    Research,
    /// Operations, deployment, monitoring, SRE.
    Operations,
    /// Creative work — writing, design, art.
    Creative,
    /// Skill acquisition, learning, training.
    SkillDomain,
    /// No clear domain detected.
    Unknown,
}

impl Domain {
    /// Return a human-readable label for the domain.
    pub fn label(&self) -> &str {
        match self {
            Self::Engineering => "engineering",
            Self::Finance => "finance",
            Self::Security => "security",
            Self::Personal => "personal",
            Self::Data => "data",
            Self::Research => "research",
            Self::Operations => "operations",
            Self::Creative => "creative",
            Self::SkillDomain => "skill",
            Self::Unknown => "unknown",
        }
    }
}

/// Vocabulary-based domain detector.
pub struct DomainVocabulary {
    /// Engineering vocabulary.
    engineering: HashSet<&'static str>,
    /// Finance vocabulary.
    finance: HashSet<&'static str>,
    /// Security vocabulary.
    security: HashSet<&'static str>,
    /// Data vocabulary.
    data: HashSet<&'static str>,
    /// Operations vocabulary.
    operations: HashSet<&'static str>,
    /// Personal vocabulary.
    personal: HashSet<&'static str>,
}

impl DomainVocabulary {
    /// Create a new domain vocabulary with default keyword sets.
    pub fn new() -> Self {
        Self {
            engineering: [
                "api",
                "code",
                "compile",
                "debug",
                "deploy",
                "docker",
                "endpoint",
                "framework",
                "function",
                "git",
                "http",
                "interface",
                "library",
                "microservice",
                "module",
                "package",
                "pipeline",
                "refactor",
                "repository",
                "rest",
                "runtime",
                "sdk",
                "server",
                "service",
                "software",
                "stack",
                "test",
                "typescript",
                "variable",
                "version",
            ]
            .into_iter()
            .collect(),

            finance: [
                "account", "asset", "balance", "bank", "budget", "capital", "cash", "cost",
                "credit", "debt", "dividend", "equity", "expense", "fund", "income", "interest",
                "invest", "ledger", "margin", "market", "payment", "profit", "revenue", "tax",
            ]
            .into_iter()
            .collect(),

            security: [
                "access",
                "attack",
                "auth",
                "authenticate",
                "breach",
                "certificate",
                "credential",
                "crypto",
                "decrypt",
                "encrypt",
                "exploit",
                "firewall",
                "hash",
                "intrusion",
                "malware",
                "permission",
                "phishing",
                "token",
                "threat",
                "vulnerability",
            ]
            .into_iter()
            .collect(),

            data: [
                "aggregate",
                "analytics",
                "batch",
                "bigquery",
                "column",
                "dashboard",
                "database",
                "dataset",
                "etl",
                "export",
                "filter",
                "import",
                "index",
                "ingest",
                "join",
                "kafka",
                "metric",
                "pipeline",
                "query",
                "schema",
                "sql",
                "table",
            ]
            .into_iter()
            .collect(),

            operations: [
                "alert",
                "availability",
                "backup",
                "capacity",
                "cluster",
                "container",
                "cron",
                "deploy",
                "downtime",
                "failover",
                "healthcheck",
                "incident",
                "kubernetes",
                "latency",
                "monitor",
                "node",
                "orchestrate",
                "provision",
                "replica",
                "rollback",
                "scale",
                "uptime",
            ]
            .into_iter()
            .collect(),

            personal: [
                "appointment",
                "birthday",
                "calendar",
                "dinner",
                "errand",
                "family",
                "flight",
                "grocery",
                "health",
                "hobby",
                "home",
                "meeting",
                "reminder",
                "schedule",
                "todo",
                "vacation",
            ]
            .into_iter()
            .collect(),
        }
    }

    /// Detect domains from input text. Returns up to `MAX_DETECTED_DOMAINS`
    /// matches ordered by confidence (descending).
    pub fn detect(&self, input: &str) -> Vec<(Domain, f64)> {
        let words: HashSet<String> = input
            .split_whitespace()
            .map(|w| w.to_lowercase().replace(|c: char| !c.is_alphanumeric(), ""))
            .filter(|w| w.len() >= 2)
            .collect();

        if words.is_empty() {
            return vec![(Domain::Unknown, 0.0)];
        }

        let mut scores: Vec<(Domain, f64)> = vec![
            (
                Domain::Engineering,
                self.match_score(&words, &self.engineering),
            ),
            (Domain::Finance, self.match_score(&words, &self.finance)),
            (Domain::Security, self.match_score(&words, &self.security)),
            (Domain::Data, self.match_score(&words, &self.data)),
            (
                Domain::Operations,
                self.match_score(&words, &self.operations),
            ),
            (Domain::Personal, self.match_score(&words, &self.personal)),
        ];

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.retain(|&(_, conf)| conf >= DOMAIN_VOCAB_MATCH_THRESHOLD);
        scores.truncate(MAX_DETECTED_DOMAINS);

        if scores.is_empty() {
            vec![(Domain::Unknown, 0.0)]
        } else {
            scores
        }
    }

    /// Return the best-matching domain.
    pub fn primary(&self, input: &str) -> (Domain, f64) {
        self.detect(input)
            .into_iter()
            .next()
            .unwrap_or((Domain::Unknown, 0.0))
    }

    /// Compute match score: fraction of input words found in the vocabulary.
    fn match_score(&self, input_words: &HashSet<String>, vocab: &HashSet<&str>) -> f64 {
        if input_words.is_empty() {
            return 0.0;
        }
        let hits = input_words
            .iter()
            .filter(|w| vocab.contains(w.as_str()))
            .count();
        hits as f64 / input_words.len() as f64
    }
}

impl Default for DomainVocabulary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_engineering() {
        let vocab = DomainVocabulary::new();
        let (domain, conf) = vocab.primary("deploy the api service to docker");
        assert_eq!(domain, Domain::Engineering);
        assert!(conf > 0.0);
    }

    #[test]
    fn detects_finance() {
        let vocab = DomainVocabulary::new();
        let (domain, _) = vocab.primary("check the account balance and revenue");
        assert_eq!(domain, Domain::Finance);
    }

    #[test]
    fn detects_security() {
        let vocab = DomainVocabulary::new();
        let (domain, _) = vocab.primary("fix the auth vulnerability in credential store");
        assert_eq!(domain, Domain::Security);
    }

    #[test]
    fn empty_returns_unknown() {
        let vocab = DomainVocabulary::new();
        let (domain, _) = vocab.primary("");
        assert_eq!(domain, Domain::Unknown);
    }

    #[test]
    fn max_domains_capped() {
        let vocab = DomainVocabulary::new();
        let results = vocab.detect("deploy api budget auth pipeline cron");
        assert!(results.len() <= MAX_DETECTED_DOMAINS);
    }
}
