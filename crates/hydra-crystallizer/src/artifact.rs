//! CrystallizedArtifact — the output of crystallization.
//! Sourced from real operational data. Never from templates.

use serde::{Deserialize, Serialize};

/// The type of crystallized artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// Step-by-step guide derived from successful executions.
    Playbook,
    /// Technical specification derived from operational patterns.
    Specification,
    /// Proven standard derived from repeated successful approaches.
    Standard,
    /// Root cause analysis from failure patterns.
    PostMortem,
    /// Domain knowledge base from omniscience acquisitions.
    KnowledgeBase,
}

impl ArtifactKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Playbook => "playbook",
            Self::Specification => "specification",
            Self::Standard => "standard",
            Self::PostMortem => "post-mortem",
            Self::KnowledgeBase => "knowledge-base",
        }
    }
}

/// One crystallized artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizedArtifact {
    pub id: String,
    pub kind: ArtifactKind,
    pub title: String,
    pub domain: String,
    pub content: String,
    pub source_count: usize, // how many records contributed
    pub confidence: f64,
    pub version: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl CrystallizedArtifact {
    pub fn new(
        kind: ArtifactKind,
        title: impl Into<String>,
        domain: impl Into<String>,
        content: impl Into<String>,
        source_count: usize,
        confidence: f64,
    ) -> Self {
        let now = chrono::Utc::now();
        let content = content.into();
        let content = if content.len() > crate::constants::MAX_ARTIFACT_CONTENT_CHARS {
            let max_bytes = crate::constants::MAX_ARTIFACT_CONTENT_CHARS - 14;
            let safe_end = content
                .char_indices()
                .take_while(|(i, _)| *i <= max_bytes)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(max_bytes.min(content.len()));
            format!("{}...[truncated]", &content[..safe_end])
        } else {
            content
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            title: title.into(),
            domain: domain.into(),
            content,
            source_count,
            confidence: confidence.clamp(0.0, 1.0),
            version: 1,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_content(&mut self, new_content: impl Into<String>, new_confidence: f64) {
        self.content = new_content.into();
        self.confidence = new_confidence.clamp(0.0, 1.0);
        self.version += 1;
        self.updated_at = chrono::Utc::now();
    }

    pub fn summary_line(&self) -> String {
        format!(
            "[{}] {} v{} — {} records, confidence={:.2}",
            self.kind.label(),
            self.title,
            self.version,
            self.source_count,
            self.confidence,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_created_correctly() {
        let a = CrystallizedArtifact::new(
            ArtifactKind::Playbook,
            "Deploy Auth Service",
            "engineering",
            "Step 1: rotate credentials\nStep 2: deploy canary\nStep 3: promote",
            12,
            0.88,
        );
        assert_eq!(a.version, 1);
        assert!(!a.content.is_empty());
    }

    #[test]
    fn update_increments_version() {
        let mut a = CrystallizedArtifact::new(
            ArtifactKind::Standard,
            "Test",
            "test",
            "v1 content",
            5,
            0.75,
        );
        a.update_content("v2 content", 0.85);
        assert_eq!(a.version, 2);
        assert_eq!(a.content, "v2 content");
    }
}
