//! Skill browser — browse crystallized skills from Evolve sister.

use serde::{Deserialize, Serialize};

/// A skill entry in the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub usage_count: u32,
    pub last_used: Option<String>,
    pub source: SkillSource,
}

/// Where a skill originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillSource {
    Crystallized,
    Builtin,
    Community,
    Custom,
}

impl SkillSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Crystallized => "Crystallized",
            Self::Builtin => "Built-in",
            Self::Community => "Community",
            Self::Custom => "Custom",
        }
    }
}

/// The skill browser view model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBrowser {
    pub skills: Vec<SkillEntry>,
    pub search_query: String,
    pub selected_category: Option<String>,
    pub selected_skill_id: Option<String>,
}

impl SkillBrowser {
    /// Create an empty skill browser.
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
            search_query: String::new(),
            selected_category: None,
            selected_skill_id: None,
        }
    }

    /// Add a skill.
    pub fn add_skill(&mut self, skill: SkillEntry) {
        self.skills.push(skill);
    }

    /// Get unique categories.
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self.skills.iter().map(|s| s.category.clone()).collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// Filter skills by search query and category.
    pub fn filtered(&self) -> Vec<&SkillEntry> {
        let query_lower = self.search_query.to_lowercase();
        self.skills
            .iter()
            .filter(|s| {
                let matches_query = query_lower.is_empty()
                    || s.name.to_lowercase().contains(&query_lower)
                    || s.description.to_lowercase().contains(&query_lower);
                let matches_cat = self.selected_category.is_none()
                    || self.selected_category.as_deref() == Some(&s.category);
                matches_query && matches_cat
            })
            .collect()
    }

    /// Get the currently selected skill.
    pub fn selected_skill(&self) -> Option<&SkillEntry> {
        self.selected_skill_id
            .as_ref()
            .and_then(|id| self.skills.iter().find(|s| s.id == *id))
    }

    /// Total skill count.
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Skills sorted by usage (most used first).
    pub fn by_usage(&self) -> Vec<&SkillEntry> {
        let mut sorted: Vec<&SkillEntry> = self.skills.iter().collect();
        sorted.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        sorted
    }
}

impl Default for SkillBrowser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_skill(id: &str, name: &str, category: &str, usage: u32) -> SkillEntry {
        SkillEntry {
            id: id.into(),
            name: name.into(),
            description: format!("Description for {}", name),
            category: category.into(),
            usage_count: usage,
            last_used: None,
            source: SkillSource::Crystallized,
        }
    }

    #[test]
    fn test_skill_browser_creation() {
        let browser = SkillBrowser::new();
        assert!(browser.skills.is_empty());
        assert_eq!(browser.count(), 0);
    }

    #[test]
    fn test_add_and_filter() {
        let mut browser = SkillBrowser::new();
        browser.add_skill(sample_skill("1", "Code Review", "Development", 10));
        browser.add_skill(sample_skill("2", "File Backup", "Operations", 5));
        browser.add_skill(sample_skill("3", "Code Format", "Development", 8));

        assert_eq!(browser.count(), 3);
        assert_eq!(browser.filtered().len(), 3);

        browser.search_query = "code".into();
        assert_eq!(browser.filtered().len(), 2);

        browser.search_query.clear();
        browser.selected_category = Some("Operations".into());
        assert_eq!(browser.filtered().len(), 1);
    }

    #[test]
    fn test_categories() {
        let mut browser = SkillBrowser::new();
        browser.add_skill(sample_skill("1", "A", "Dev", 1));
        browser.add_skill(sample_skill("2", "B", "Ops", 1));
        browser.add_skill(sample_skill("3", "C", "Dev", 1));
        let cats = browser.categories();
        assert_eq!(cats, vec!["Dev", "Ops"]);
    }

    #[test]
    fn test_by_usage() {
        let mut browser = SkillBrowser::new();
        browser.add_skill(sample_skill("1", "Low", "A", 1));
        browser.add_skill(sample_skill("2", "High", "A", 100));
        browser.add_skill(sample_skill("3", "Mid", "A", 50));
        let sorted = browser.by_usage();
        assert_eq!(sorted[0].name, "High");
        assert_eq!(sorted[1].name, "Mid");
        assert_eq!(sorted[2].name, "Low");
    }

    #[test]
    fn test_selected_skill() {
        let mut browser = SkillBrowser::new();
        browser.add_skill(sample_skill("s1", "Test", "Cat", 1));
        assert!(browser.selected_skill().is_none());
        browser.selected_skill_id = Some("s1".into());
        assert_eq!(browser.selected_skill().unwrap().name, "Test");
    }
}
