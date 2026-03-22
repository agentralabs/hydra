//! Web Knowledge — Hydra's gateway to the internet.
//!
//! Three layers of omniscience:
//!
//! Layer 1: GENOME (instant, zero calls)
//!   If the answer is in the genome, return it. No web call needed.
//!   Over time, most questions route here.
//!
//! Layer 2: INDEX (one call, right source)
//!   A topic→source index tells Hydra WHERE to look.
//!   "rust ownership" → docs.rust-lang.org
//!   "circuit breaker" → martinfowler.com/bliki/CircuitBreaker.html
//!   One targeted call instead of a blind search.
//!
//! Layer 3: SEARCH (one call, broad)
//!   If the index doesn't have the topic, search the web.
//!   The result is indexed for next time.
//!   The answer is crystallized into a genome entry.
//!
//! The dream loop expands the index by exploring topics
//! from recent conversations. Hydra gets smarter overnight.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A knowledge source — where to find information about a topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSource {
    pub topic: String,
    pub url: String,
    pub source_type: SourceType,
    pub reliability: f64,
    pub last_accessed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Documentation,
    Wikipedia,
    GitHub,
    StackOverflow,
    Blog,
    Paper,
    Official,
    Community,
}

/// The knowledge index — maps topics to their best sources.
#[derive(Debug, Default)]
pub struct KnowledgeIndex {
    /// Topic → list of known sources, sorted by reliability.
    sources: HashMap<String, Vec<KnowledgeSource>>,
}

impl KnowledgeIndex {
    pub fn new() -> Self {
        let mut idx = Self::default();
        idx.seed_foundational();
        idx
    }

    /// Seed with foundational knowledge sources that every Hydra should know.
    fn seed_foundational(&mut self) {
        let seeds = vec![
            // Programming languages
            ("rust", "https://doc.rust-lang.org/book/", SourceType::Documentation, 0.98),
            ("python", "https://docs.python.org/3/", SourceType::Documentation, 0.98),
            ("javascript", "https://developer.mozilla.org/en-US/docs/Web/JavaScript", SourceType::Documentation, 0.97),
            ("typescript", "https://www.typescriptlang.org/docs/", SourceType::Documentation, 0.97),
            ("go", "https://go.dev/doc/", SourceType::Documentation, 0.97),
            // Frameworks
            ("react", "https://react.dev/learn", SourceType::Documentation, 0.96),
            ("kubernetes", "https://kubernetes.io/docs/", SourceType::Documentation, 0.97),
            ("docker", "https://docs.docker.com/", SourceType::Documentation, 0.97),
            ("terraform", "https://developer.hashicorp.com/terraform/docs", SourceType::Documentation, 0.96),
            // Architecture patterns
            ("circuit breaker", "https://martinfowler.com/bliki/CircuitBreaker.html", SourceType::Blog, 0.95),
            ("microservices", "https://microservices.io/patterns/", SourceType::Blog, 0.93),
            ("design patterns", "https://refactoring.guru/design-patterns", SourceType::Community, 0.92),
            // Science
            ("physics", "https://en.wikipedia.org/wiki/Physics", SourceType::Wikipedia, 0.90),
            ("chemistry", "https://en.wikipedia.org/wiki/Chemistry", SourceType::Wikipedia, 0.90),
            ("biology", "https://en.wikipedia.org/wiki/Biology", SourceType::Wikipedia, 0.90),
            ("mathematics", "https://en.wikipedia.org/wiki/Mathematics", SourceType::Wikipedia, 0.90),
            // Finance
            ("stock market", "https://www.investopedia.com/", SourceType::Community, 0.88),
            ("cryptocurrency", "https://en.wikipedia.org/wiki/Cryptocurrency", SourceType::Wikipedia, 0.85),
            // AI/ML
            ("machine learning", "https://scikit-learn.org/stable/user_guide.html", SourceType::Documentation, 0.95),
            ("neural networks", "https://en.wikipedia.org/wiki/Neural_network_(machine_learning)", SourceType::Wikipedia, 0.90),
            ("transformers", "https://huggingface.co/docs/transformers/", SourceType::Documentation, 0.95),
            ("pytorch", "https://pytorch.org/docs/stable/", SourceType::Documentation, 0.96),
            ("openai api", "https://platform.openai.com/docs/", SourceType::Documentation, 0.96),
            ("langchain", "https://python.langchain.com/docs/", SourceType::Documentation, 0.93),
            ("embeddings", "https://platform.openai.com/docs/guides/embeddings", SourceType::Documentation, 0.94),
            // More programming
            ("java", "https://docs.oracle.com/en/java/", SourceType::Documentation, 0.96),
            ("c++", "https://en.cppreference.com/w/", SourceType::Documentation, 0.97),
            ("swift", "https://docs.swift.org/swift-book/", SourceType::Documentation, 0.96),
            ("kotlin", "https://kotlinlang.org/docs/", SourceType::Documentation, 0.96),
            ("ruby", "https://ruby-doc.org/", SourceType::Documentation, 0.95),
            ("php", "https://www.php.net/docs.php", SourceType::Documentation, 0.95),
            ("sql", "https://www.w3schools.com/sql/", SourceType::Community, 0.88),
            ("postgresql", "https://www.postgresql.org/docs/current/", SourceType::Documentation, 0.97),
            ("redis", "https://redis.io/docs/", SourceType::Documentation, 0.96),
            ("mongodb", "https://www.mongodb.com/docs/", SourceType::Documentation, 0.95),
            ("elasticsearch", "https://www.elastic.co/guide/en/elasticsearch/reference/current/", SourceType::Documentation, 0.95),
            // Web & frameworks
            ("nextjs", "https://nextjs.org/docs", SourceType::Documentation, 0.96),
            ("vue", "https://vuejs.org/guide/", SourceType::Documentation, 0.95),
            ("angular", "https://angular.io/docs", SourceType::Documentation, 0.95),
            ("django", "https://docs.djangoproject.com/", SourceType::Documentation, 0.96),
            ("flask", "https://flask.palletsprojects.com/", SourceType::Documentation, 0.95),
            ("express", "https://expressjs.com/en/guide/", SourceType::Documentation, 0.94),
            ("graphql", "https://graphql.org/learn/", SourceType::Documentation, 0.95),
            ("rest api", "https://restfulapi.net/", SourceType::Community, 0.90),
            // DevOps & cloud
            ("aws", "https://docs.aws.amazon.com/", SourceType::Documentation, 0.97),
            ("gcp", "https://cloud.google.com/docs", SourceType::Documentation, 0.97),
            ("azure", "https://learn.microsoft.com/en-us/azure/", SourceType::Documentation, 0.97),
            ("nginx", "https://nginx.org/en/docs/", SourceType::Documentation, 0.96),
            ("linux", "https://www.kernel.org/doc/html/latest/", SourceType::Documentation, 0.97),
            ("git", "https://git-scm.com/doc", SourceType::Documentation, 0.97),
            ("github actions", "https://docs.github.com/en/actions", SourceType::Documentation, 0.96),
            ("ansible", "https://docs.ansible.com/", SourceType::Documentation, 0.95),
            ("prometheus", "https://prometheus.io/docs/", SourceType::Documentation, 0.95),
            ("grafana", "https://grafana.com/docs/grafana/latest/", SourceType::Documentation, 0.95),
            // Security
            ("owasp", "https://owasp.org/www-project-top-ten/", SourceType::Official, 0.96),
            ("cryptography", "https://en.wikipedia.org/wiki/Cryptography", SourceType::Wikipedia, 0.90),
            ("tls ssl", "https://en.wikipedia.org/wiki/Transport_Layer_Security", SourceType::Wikipedia, 0.90),
            ("oauth", "https://oauth.net/2/", SourceType::Official, 0.94),
            ("jwt", "https://jwt.io/introduction", SourceType::Community, 0.92),
            // More science
            ("quantum computing", "https://en.wikipedia.org/wiki/Quantum_computing", SourceType::Wikipedia, 0.88),
            ("general relativity", "https://en.wikipedia.org/wiki/General_relativity", SourceType::Wikipedia, 0.90),
            ("organic chemistry", "https://en.wikipedia.org/wiki/Organic_chemistry", SourceType::Wikipedia, 0.89),
            ("genetics", "https://en.wikipedia.org/wiki/Genetics", SourceType::Wikipedia, 0.90),
            ("evolution", "https://en.wikipedia.org/wiki/Evolution", SourceType::Wikipedia, 0.91),
            ("calculus", "https://en.wikipedia.org/wiki/Calculus", SourceType::Wikipedia, 0.90),
            ("linear algebra", "https://en.wikipedia.org/wiki/Linear_algebra", SourceType::Wikipedia, 0.90),
            ("statistics", "https://en.wikipedia.org/wiki/Statistics", SourceType::Wikipedia, 0.89),
            ("thermodynamics", "https://en.wikipedia.org/wiki/Thermodynamics", SourceType::Wikipedia, 0.90),
            ("electromagnetism", "https://en.wikipedia.org/wiki/Electromagnetism", SourceType::Wikipedia, 0.90),
            // Finance & economics
            ("options trading", "https://www.investopedia.com/options-basics-tutorial-4583012", SourceType::Community, 0.87),
            ("federal reserve", "https://www.federalreserve.gov/", SourceType::Official, 0.97),
            ("economics", "https://en.wikipedia.org/wiki/Economics", SourceType::Wikipedia, 0.89),
            ("accounting", "https://en.wikipedia.org/wiki/Accounting", SourceType::Wikipedia, 0.88),
            ("venture capital", "https://en.wikipedia.org/wiki/Venture_capital", SourceType::Wikipedia, 0.87),
            // Humanities & general knowledge
            ("philosophy", "https://plato.stanford.edu/", SourceType::Official, 0.95),
            ("history", "https://en.wikipedia.org/wiki/History", SourceType::Wikipedia, 0.88),
            ("psychology", "https://en.wikipedia.org/wiki/Psychology", SourceType::Wikipedia, 0.88),
            ("law", "https://en.wikipedia.org/wiki/Law", SourceType::Wikipedia, 0.85),
            ("political science", "https://en.wikipedia.org/wiki/Political_science", SourceType::Wikipedia, 0.86),
            // Health
            ("nutrition", "https://en.wikipedia.org/wiki/Nutrition", SourceType::Wikipedia, 0.85),
            ("exercise science", "https://en.wikipedia.org/wiki/Exercise_physiology", SourceType::Wikipedia, 0.85),
            ("mental health", "https://en.wikipedia.org/wiki/Mental_health", SourceType::Wikipedia, 0.85),
        ];

        for (topic, url, source_type, reliability) in seeds {
            self.add(KnowledgeSource {
                topic: topic.to_string(),
                url: url.to_string(),
                source_type,
                reliability,
                last_accessed: None,
            });
        }
    }

    /// Add a knowledge source to the index.
    pub fn add(&mut self, source: KnowledgeSource) {
        let entry = self.sources.entry(source.topic.clone()).or_default();
        // Don't add duplicates
        if !entry.iter().any(|s| s.url == source.url) {
            entry.push(source);
            // Sort by reliability descending
            entry.sort_by(|a, b| {
                b.reliability
                    .partial_cmp(&a.reliability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    /// Find the best source for a topic.
    pub fn find(&self, topic: &str) -> Option<&KnowledgeSource> {
        let topic_lower = topic.to_lowercase();
        // Exact match
        if let Some(sources) = self.sources.get(&topic_lower) {
            return sources.first();
        }
        // Partial match — find topics that contain the query
        for (key, sources) in &self.sources {
            if key.contains(&topic_lower) || topic_lower.contains(key) {
                return sources.first();
            }
        }
        None
    }

    /// Find all sources for a topic.
    pub fn find_all(&self, topic: &str) -> Vec<&KnowledgeSource> {
        let topic_lower = topic.to_lowercase();
        let mut results = Vec::new();
        for (key, sources) in &self.sources {
            if key.contains(&topic_lower) || topic_lower.contains(key) {
                results.extend(sources.iter());
            }
        }
        results.sort_by(|a, b| {
            b.reliability
                .partial_cmp(&a.reliability)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// How many topics are indexed.
    pub fn topic_count(&self) -> usize {
        self.sources.len()
    }

    /// How many total sources are indexed.
    pub fn source_count(&self) -> usize {
        self.sources.values().map(|v| v.len()).sum()
    }

    /// Describe the resolution strategy for a query.
    pub fn resolution_strategy(&self, query: &str) -> ResolutionStrategy {
        if let Some(source) = self.find(query) {
            ResolutionStrategy::Indexed {
                url: source.url.clone(),
                reliability: source.reliability,
            }
        } else {
            ResolutionStrategy::Search {
                query: query.to_string(),
            }
        }
    }
}

/// How Hydra will resolve a knowledge query.
#[derive(Debug, Clone)]
pub enum ResolutionStrategy {
    /// Answer is in the genome — zero web calls.
    Genome { entry_confidence: f64 },
    /// Topic is in the index — one targeted call.
    Indexed { url: String, reliability: f64 },
    /// Topic is unknown — one search call, then index the result.
    Search { query: String },
}

impl ResolutionStrategy {
    pub fn describe(&self) -> String {
        match self {
            Self::Genome { entry_confidence } => {
                format!(
                    "Answer from genome (conf={:.0}%). Zero web calls.",
                    entry_confidence * 100.0
                )
            }
            Self::Indexed { url, reliability } => {
                format!(
                    "Indexed source: {} (reliability={:.0}%). One targeted call.",
                    url,
                    reliability * 100.0
                )
            }
            Self::Search { query } => {
                format!(
                    "Unknown topic: '{}'. One search call. Result will be indexed.",
                    query
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_seeded() {
        let idx = KnowledgeIndex::new();
        assert!(idx.topic_count() > 15);
        assert!(idx.source_count() > 15);
    }

    #[test]
    fn find_exact() {
        let idx = KnowledgeIndex::new();
        let source = idx.find("rust");
        assert!(source.is_some());
        assert!(source.unwrap().url.contains("rust-lang"));
    }

    #[test]
    fn find_partial() {
        let idx = KnowledgeIndex::new();
        let source = idx.find("circuit");
        assert!(source.is_some());
        assert!(source.unwrap().url.contains("CircuitBreaker"));
    }

    #[test]
    fn resolution_indexed() {
        let idx = KnowledgeIndex::new();
        let strategy = idx.resolution_strategy("kubernetes");
        assert!(matches!(strategy, ResolutionStrategy::Indexed { .. }));
    }

    #[test]
    fn resolution_search() {
        let idx = KnowledgeIndex::new();
        let strategy = idx.resolution_strategy("quantum teleportation");
        assert!(matches!(strategy, ResolutionStrategy::Search { .. }));
    }

    #[test]
    fn add_and_find() {
        let mut idx = KnowledgeIndex::new();
        idx.add(KnowledgeSource {
            topic: "hydra architecture".into(),
            url: "https://github.com/agentralabs/hydra".into(),
            source_type: SourceType::GitHub,
            reliability: 0.99,
            last_accessed: None,
        });
        let source = idx.find("hydra architecture");
        assert!(source.is_some());
    }
}
