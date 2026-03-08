//! KnowledgeBase — shared knowledge repository for collective learning.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Category of knowledge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgeCategory {
    BestPractice,
    AntiPattern,
    ToolUsage,
    DomainFact,
    UserPreference,
}

/// A knowledge item in the shared base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: String,
    pub category: KnowledgeCategory,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub votes: i64,
    pub source_count: usize,
    pub created_at: String,
}

impl KnowledgeItem {
    pub fn new(category: KnowledgeCategory, title: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            category,
            title: title.into(),
            content: content.into(),
            tags: Vec::new(),
            votes: 0,
            source_count: 1,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(String::from).collect();
        self
    }

    pub fn upvote(&mut self) {
        self.votes += 1;
    }

    pub fn downvote(&mut self) {
        self.votes -= 1;
    }
}

/// Shared knowledge base across instances
pub struct KnowledgeBase {
    items: parking_lot::RwLock<HashMap<String, KnowledgeItem>>,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self {
            items: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Add a knowledge item
    pub fn add(&self, item: KnowledgeItem) -> String {
        let id = item.id.clone();
        self.items.write().insert(id.clone(), item);
        id
    }

    /// Search by tag
    pub fn search_by_tag(&self, tag: &str) -> Vec<KnowledgeItem> {
        self.items
            .read()
            .values()
            .filter(|item| item.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    /// Search by category
    pub fn search_by_category(&self, category: KnowledgeCategory) -> Vec<KnowledgeItem> {
        self.items
            .read()
            .values()
            .filter(|item| item.category == category)
            .cloned()
            .collect()
    }

    /// Upvote a knowledge item
    pub fn upvote(&self, id: &str) -> bool {
        if let Some(item) = self.items.write().get_mut(id) {
            item.upvote();
            true
        } else {
            false
        }
    }

    /// Get top-voted items
    pub fn top_items(&self, limit: usize) -> Vec<KnowledgeItem> {
        let mut items: Vec<_> = self.items.read().values().cloned().collect();
        items.sort_by(|a, b| b.votes.cmp(&a.votes));
        items.truncate(limit);
        items
    }

    pub fn count(&self) -> usize {
        self.items.read().len()
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_add_search() {
        let kb = KnowledgeBase::new();
        let item = KnowledgeItem::new(
            KnowledgeCategory::BestPractice,
            "Use batch writes",
            "Batch file writes improve throughput",
        )
        .with_tags(vec!["files", "performance"]);

        kb.add(item);
        assert_eq!(kb.count(), 1);

        let results = kb.search_by_tag("performance");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_knowledge_voting() {
        let kb = KnowledgeBase::new();
        let item = KnowledgeItem::new(KnowledgeCategory::ToolUsage, "Tip", "Use grep");
        let id = kb.add(item);

        kb.upvote(&id);
        kb.upvote(&id);

        let top = kb.top_items(1);
        assert_eq!(top[0].votes, 2);
    }

    #[test]
    fn test_category_search() {
        let kb = KnowledgeBase::new();
        kb.add(KnowledgeItem::new(KnowledgeCategory::AntiPattern, "Bad", "Don't do X"));
        kb.add(KnowledgeItem::new(KnowledgeCategory::BestPractice, "Good", "Do Y"));
        kb.add(KnowledgeItem::new(KnowledgeCategory::AntiPattern, "Also bad", "Don't do Z"));

        let anti = kb.search_by_category(KnowledgeCategory::AntiPattern);
        assert_eq!(anti.len(), 2);
    }
}
