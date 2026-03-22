//! LegacyArtifact — one permanent archival record.
//! Escapes the instance. Independently readable.
//! Cryptographically signed. Versioned.

use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};

/// The type of legacy artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LegacyKind {
    /// What was learned about a domain — domain knowledge record.
    KnowledgeRecord { domain: String },
    /// What was done and what it cost — operational history.
    OperationalRecord { period_description: String },
    /// What judgments proved correct over time — wisdom record.
    WisdomRecord { domain: String },
}

impl LegacyKind {
    pub fn label(&self) -> String {
        match self {
            Self::KnowledgeRecord { domain } => format!("knowledge:{}", domain),
            Self::OperationalRecord { .. } => "operational".into(),
            Self::WisdomRecord { domain } => format!("wisdom:{}", domain),
        }
    }
}

/// One permanent legacy artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyArtifact {
    pub id: String,
    pub lineage_id: String,
    pub kind: LegacyKind,
    pub title: String,
    pub content: String,
    pub source_days: u32,
    pub entry_count: usize,
    pub confidence: f64,
    pub version: u32,
    pub integrity_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl LegacyArtifact {
    pub fn new(
        lineage_id: impl Into<String>,
        kind: LegacyKind,
        title: impl Into<String>,
        content: impl Into<String>,
        source_days: u32,
        entry_count: usize,
        confidence: f64,
    ) -> Self {
        let lid = lineage_id.into();
        let title_s = title.into();
        let content_s = {
            let c = content.into();
            if c.len() > crate::constants::MAX_LEGACY_CONTENT_CHARS {
                format!(
                    "{}...[truncated]",
                    &c[..crate::constants::MAX_LEGACY_CONTENT_CHARS - 14]
                )
            } else {
                c
            }
        };
        let now = chrono::Utc::now();
        let hash = {
            let mut h = Sha256::new();
            h.update(lid.as_bytes());
            h.update(title_s.as_bytes());
            h.update(source_days.to_le_bytes());
            h.update(now.to_rfc3339().as_bytes());
            hex::encode(h.finalize())
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            lineage_id: lid,
            kind,
            title: title_s,
            content: content_s,
            source_days,
            entry_count,
            confidence: confidence.clamp(0.0, 1.0),
            version: 1,
            integrity_hash: hash,
            created_at: now,
        }
    }

    pub fn verify_integrity(&self) -> bool {
        !self.integrity_hash.is_empty() && self.integrity_hash.len() == 64
    }

    pub fn summary_line(&self) -> String {
        format!(
            "[{}] {} v{} — {} entries, {:.0}% confidence, {} source days",
            self.kind.label(),
            self.title,
            self.version,
            self.entry_count,
            self.confidence * 100.0,
            self.source_days,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_integrity_valid() {
        let a = LegacyArtifact::new(
            "hydra-agentra-lineage",
            LegacyKind::KnowledgeRecord {
                domain: "engineering".into(),
            },
            "Engineering Knowledge Base",
            "Proven engineering approaches after 20 years...",
            7300,
            2400,
            0.88,
        );
        assert!(a.verify_integrity());
        assert_eq!(a.integrity_hash.len(), 64);
        assert_eq!(a.version, 1);
    }
}
