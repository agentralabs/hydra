//! Autonomous learning loop — fetches curated sources, extracts knowledge,
//! validates against genome, and adds new entries. Zero human input required.
//! Configured via ~/.hydra/learning/sources.toml.

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use hydra_genome::{GenomeStore, ApproachSignature};

use crate::learning_validator::{self, ValidationResult};

/// Result of a learning tick.
#[derive(Debug, Default)]
pub struct LearningResult {
    pub entries_added: usize,
    pub entries_rejected: usize,
    pub conflicts: usize,
    pub sources_checked: usize,
}

/// Extract mode for a source.
#[derive(Debug, Clone, serde::Deserialize)]
pub enum ExtractMode {
    #[serde(rename = "titles_and_links")]
    TitlesAndLinks,
    #[serde(rename = "descriptions")]
    Descriptions,
    #[serde(rename = "abstracts")]
    Abstracts,
    #[serde(rename = "full_text")]
    FullText,
}

/// A learning source from config.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LearningSource {
    pub url: String,
    pub frequency: String,  // "4h", "6h", "12h", "24h"
    pub domain: String,
    pub extract: ExtractMode,
}

impl LearningSource {
    fn frequency_secs(&self) -> u64 {
        let s = self.frequency.trim();
        if let Some(h) = s.strip_suffix('h') {
            h.parse::<u64>().unwrap_or(24) * 3600
        } else if let Some(m) = s.strip_suffix('m') {
            m.parse::<u64>().unwrap_or(60) * 60
        } else { 86400 }
    }
}

/// Config file structure.
#[derive(Debug, serde::Deserialize)]
struct LearningConfig {
    #[serde(default)]
    learning: LearningSettings,
    #[serde(default)]
    source: Vec<LearningSource>,
}

#[derive(Debug, serde::Deserialize)]
struct LearningSettings {
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default = "default_max_per_day")]
    max_entries_per_day: usize,
    #[serde(default = "default_min_conf")]
    min_confidence: f64,
}

impl Default for LearningSettings {
    fn default() -> Self {
        Self { enabled: true, max_entries_per_day: 50, min_confidence: 0.6 }
    }
}

fn default_enabled() -> bool { true }
fn default_max_per_day() -> usize { 50 }
fn default_min_conf() -> f64 { 0.6 }

/// The autonomous learning loop.
pub struct LearningLoop {
    sources: Vec<LearningSource>,
    last_harvest: HashMap<String, DateTime<Utc>>,
    entries_today: usize,
    max_per_day: usize,
    min_confidence: f64,
    enabled: bool,
}

impl LearningLoop {
    /// Load config from ~/.hydra/learning/sources.toml or use defaults.
    pub fn new() -> Self {
        let (sources, settings) = load_config();
        Self {
            sources,
            last_harvest: HashMap::new(),
            entries_today: 0,
            max_per_day: settings.max_entries_per_day,
            min_confidence: settings.min_confidence,
            enabled: settings.enabled,
        }
    }

    /// Run one tick of the learning loop. Check due sources and harvest.
    pub fn tick(&mut self, genome: &mut GenomeStore) -> LearningResult {
        if !self.enabled { return LearningResult::default(); }

        let mut result = LearningResult::default();
        let now = Utc::now();

        for source in &self.sources {
            if self.entries_today >= self.max_per_day { break; }

            // Check if source is due
            let last = self.last_harvest.get(&source.url).copied().unwrap_or(DateTime::UNIX_EPOCH);
            let elapsed = (now - last).num_seconds() as u64;
            if elapsed < source.frequency_secs() { continue; }

            result.sources_checked += 1;
            eprintln!("hydra-learning: harvesting {}", source.url);

            // Fetch and extract
            match fetch_and_extract(&source.url) {
                Ok(text) => {
                    let chunks = split_into_chunks(&text, &source.extract);
                    for chunk in chunks {
                        if chunk.trim().len() < 50 { continue; }
                        if self.entries_today >= self.max_per_day { break; }

                        match learning_validator::validate(&chunk, &source.domain, genome) {
                            ValidationResult::Novel => {
                                let approach = ApproachSignature::new(
                                    "web-harvested",
                                    vec![chunk.chars().take(200).collect()],
                                    vec!["learning-loop".into()],
                                );
                                match genome.add_from_operation(&chunk, approach, self.min_confidence) {
                                    Ok(id) => {
                                        eprintln!("hydra-learning: added entry {id}");
                                        result.entries_added += 1;
                                        self.entries_today += 1;
                                    }
                                    Err(e) => eprintln!("hydra-learning: add failed: {e}"),
                                }
                            }
                            ValidationResult::Complementary { existing_id } => {
                                if let Err(e) = genome.record_use(&existing_id, true) {
                                    eprintln!("hydra-learning: record_use failed: {e}");
                                }
                                result.entries_rejected += 1;
                            }
                            ValidationResult::Conflict { existing_id } => {
                                learning_validator::save_conflict(&chunk, &existing_id, &source.domain);
                                result.conflicts += 1;
                            }
                            ValidationResult::Duplicate { .. } => {
                                result.entries_rejected += 1;
                            }
                        }
                    }
                }
                Err(e) => eprintln!("hydra-learning: fetch failed for {}: {e}", source.url),
            }

            self.last_harvest.insert(source.url.clone(), now);
        }

        if result.entries_added > 0 {
            eprintln!("hydra-learning: +{} entries, {} rejected, {} conflicts",
                result.entries_added, result.entries_rejected, result.conflicts);
        }
        result
    }
}

impl Default for LearningLoop {
    fn default() -> Self { Self::new() }
}

fn fetch_and_extract(url: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
        .timeout(std::time::Duration::from_secs(15))
        .build().map_err(|e| format!("{e}"))?;
    let resp = client.get(url).send().map_err(|e| format!("{e}"))?;
    let html = resp.text().map_err(|e| format!("{e}"))?;
    let extracted = hydra_web::extractor::extract(&html);
    Ok(extracted.main_text)
}

fn split_into_chunks(text: &str, mode: &ExtractMode) -> Vec<String> {
    match mode {
        ExtractMode::TitlesAndLinks | ExtractMode::Descriptions => {
            // Split by lines, each line is a chunk
            text.lines().filter(|l| l.trim().len() > 20).map(|l| l.trim().to_string()).take(20).collect()
        }
        ExtractMode::Abstracts => {
            // Split by double newlines (paragraph-level)
            text.split("\n\n").filter(|p| p.trim().len() > 50).map(|p| p.trim().to_string()).take(10).collect()
        }
        ExtractMode::FullText => {
            // Split by paragraphs, larger chunks
            text.split("\n\n").filter(|p| p.trim().len() > 100).map(|p| p.trim().to_string()).take(5).collect()
        }
    }
}

fn load_config() -> (Vec<LearningSource>, LearningSettings) {
    let path = dirs::home_dir().unwrap_or_default().join(".hydra/learning/sources.toml");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(config) = toml::from_str::<LearningConfig>(&content) {
            return (config.source, config.learning);
        }
    }
    // Default sources
    (default_sources(), LearningSettings::default())
}

fn default_sources() -> Vec<LearningSource> {
    vec![
        LearningSource { url: "https://news.ycombinator.com/best".into(), frequency: "6h".into(), domain: "engineering".into(), extract: ExtractMode::TitlesAndLinks },
        LearningSource { url: "https://github.com/trending".into(), frequency: "12h".into(), domain: "engineering".into(), extract: ExtractMode::Descriptions },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frequency() {
        let src = LearningSource { url: String::new(), frequency: "6h".into(), domain: String::new(), extract: ExtractMode::TitlesAndLinks };
        assert_eq!(src.frequency_secs(), 21600);
    }

    #[test]
    fn split_titles_and_links() {
        let text = "First headline about Rust\nSecond headline about Python\nShort\nThird headline about AI developments";
        let chunks = split_into_chunks(text, &ExtractMode::TitlesAndLinks);
        assert_eq!(chunks.len(), 3); // "Short" filtered out
    }

    #[test]
    fn learning_loop_creates() {
        let _loop = LearningLoop::new();
    }
}
