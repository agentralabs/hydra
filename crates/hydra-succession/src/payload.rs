//! The four payloads that compose a succession package.
//! Each represents one of the four transferable wisdom stores.

use serde::{Deserialize, Serialize};

/// Soul orientation payload — the meaning graph in transferable form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulPayload {
    /// Serialized soul entries (topic, meaning_score, horizon, observations).
    pub entries: Vec<SoulEntry>,
    /// Total days of accumulation.
    pub days_accumulated: u32,
    /// The founding orientation statement (day-1 soul state).
    pub founding_statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulEntry {
    pub topic: String,
    pub meaning_score: f64,
    pub horizon: String,
    pub observation_count: usize,
}

impl SoulPayload {
    pub fn simulated(days: u32, entry_count: usize) -> Self {
        let entries = (0..entry_count)
            .map(|i| SoulEntry {
                topic: format!("soul-topic-{}", i),
                meaning_score: 0.5 + (i as f64 * 0.01).min(0.49),
                horizon: if i < 3 {
                    "foundational".into()
                } else {
                    "developmental".into()
                },
                observation_count: 10 + i,
            })
            .collect();
        Self {
            entries,
            days_accumulated: days,
            founding_statement: "To serve with integrity, judgment, and care.".into(),
        }
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Genome payload — the proven approach library in transferable form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomePayload {
    pub entries: Vec<GenomeEntry>,
    pub total_domains: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomeEntry {
    pub domain: String,
    pub situation: String,
    pub approach: String,
    pub confidence: f64,
    pub observations: usize,
}

impl GenomePayload {
    pub fn simulated(entry_count: usize) -> Self {
        let domains = ["engineering", "fintech", "security", "cobol"];
        let entries = (0..entry_count)
            .map(|i| GenomeEntry {
                domain: domains[i % domains.len()].into(),
                situation: format!("situation-pattern-{}", i),
                approach: format!("proven-approach-{}", i),
                confidence: 0.70 + (i as f64 * 0.005).min(0.25),
                observations: 5 + i,
            })
            .collect();
        let total_domains = domains.len().min(entry_count);
        Self {
            entries,
            total_domains,
        }
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Calibration payload — domain bias profiles in transferable form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationPayload {
    pub profiles: Vec<CalibrationProfile>,
    pub total_records_tracked: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationProfile {
    pub domain: String,
    pub judgment_type: String,
    pub mean_offset: f64,
    pub sample_size: usize,
    pub is_significant: bool,
}

impl CalibrationPayload {
    pub fn simulated(profile_count: usize) -> Self {
        let profiles = (0..profile_count)
            .map(|i| CalibrationProfile {
                domain: format!("domain-{}", i),
                judgment_type: "risk".into(),
                mean_offset: if i % 2 == 0 { -0.08 } else { 0.05 },
                sample_size: 50 + i * 10,
                is_significant: true,
            })
            .collect();
        Self {
            profiles,
            total_records_tracked: profile_count * 100,
        }
    }

    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }
}

/// Morphic signature payload — identity continuity proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphicPayload {
    /// The entity's morphic signature fingerprint (SHA256 of accumulated state).
    pub signature: String,
    /// How many days this signature represents.
    pub days_depth: u32,
    /// The entity's lineage identifier (stable across generations).
    pub lineage_id: String,
    /// Signature chain — hashes at each major milestone.
    pub signature_chain: Vec<String>,
}

impl MorphicPayload {
    pub fn simulated(days: u32, lineage_id: &str) -> Self {
        use sha2::{Digest, Sha256};
        let sig = {
            let mut h = Sha256::new();
            h.update(lineage_id.as_bytes());
            h.update(days.to_le_bytes());
            hex::encode(h.finalize())
        };
        let chain: Vec<String> = (0..5)
            .map(|i| {
                let mut h = Sha256::new();
                h.update(format!("{}-{}", lineage_id, i).as_bytes());
                hex::encode(h.finalize())
            })
            .collect();
        Self {
            signature: sig,
            days_depth: days,
            lineage_id: lineage_id.to_string(),
            signature_chain: chain,
        }
    }

    pub fn verify(&self) -> bool {
        !self.signature.is_empty()
            && self.signature.len() == 64
            && !self.lineage_id.is_empty()
            && !self.signature_chain.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soul_payload_entry_count() {
        let p = SoulPayload::simulated(7300, 50);
        assert_eq!(p.entry_count(), 50);
        assert_eq!(p.days_accumulated, 7300);
    }

    #[test]
    fn genome_payload_domains() {
        let p = GenomePayload::simulated(20);
        assert_eq!(p.entry_count(), 20);
        assert!(p.total_domains > 0);
    }

    #[test]
    fn morphic_payload_valid() {
        let p = MorphicPayload::simulated(7300, "hydra-agentra-lineage");
        assert!(p.verify());
        assert_eq!(p.days_depth, 7300);
    }
}
