//! DreamInsight — insights generated during dream state.

use serde::{Deserialize, Serialize};

/// Category of dream insight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightCategory {
    PatternDiscovered,
    OptimizationFound,
    MemoryConsolidated,
    PredictionCached,
    CounterfactualExplored,
}

/// An insight generated during a dream session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamInsight {
    pub id: String,
    pub source_task: String,
    pub category: InsightCategory,
    pub description: String,
    pub confidence: f32,
    pub surfaced: bool,
    pub created_at: String,
}

impl DreamInsight {
    pub fn new(
        source: &str,
        category: InsightCategory,
        description: &str,
        confidence: f32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_task: source.into(),
            category,
            description: description.into(),
            confidence: confidence.clamp(0.0, 1.0),
            surfaced: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Store for dream insights with surfacing
pub struct InsightStore {
    insights: parking_lot::RwLock<Vec<DreamInsight>>,
    max_insights: usize,
}

impl InsightStore {
    pub fn new(max: usize) -> Self {
        Self {
            insights: parking_lot::RwLock::new(Vec::new()),
            max_insights: max,
        }
    }

    pub fn add(&self, insight: DreamInsight) {
        let mut store = self.insights.write();
        store.push(insight);
        if store.len() > self.max_insights {
            store.remove(0);
        }
    }

    /// Get unsurfaced insights above confidence threshold
    pub fn surface(&self, min_confidence: f32) -> Vec<DreamInsight> {
        let mut store = self.insights.write();
        let mut surfaced = Vec::new();

        for insight in store.iter_mut() {
            if !insight.surfaced && insight.confidence >= min_confidence {
                insight.surfaced = true;
                surfaced.push(insight.clone());
            }
        }

        surfaced
    }

    /// Get all insights
    pub fn all(&self) -> Vec<DreamInsight> {
        self.insights.read().clone()
    }

    /// Count total insights
    pub fn count(&self) -> usize {
        self.insights.read().len()
    }

    /// Get insights by category
    pub fn by_category(&self, category: InsightCategory) -> Vec<DreamInsight> {
        self.insights
            .read()
            .iter()
            .filter(|i| i.category == category)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insight_generation() {
        let insight = DreamInsight::new(
            "pattern_mining",
            InsightCategory::PatternDiscovered,
            "Found repeated git flow pattern",
            0.85,
        );
        assert_eq!(insight.category, InsightCategory::PatternDiscovered);
        assert!(!insight.surfaced);
    }

    #[test]
    fn test_dream_surfacing() {
        let store = InsightStore::new(100);
        store.add(DreamInsight::new(
            "a",
            InsightCategory::PatternDiscovered,
            "high",
            0.9,
        ));
        store.add(DreamInsight::new(
            "b",
            InsightCategory::OptimizationFound,
            "low",
            0.3,
        ));

        let surfaced = store.surface(0.8);
        assert_eq!(surfaced.len(), 1);
        assert_eq!(surfaced[0].description, "high");

        // Already surfaced, won't surface again
        let surfaced2 = store.surface(0.8);
        assert_eq!(surfaced2.len(), 0);
    }
}
