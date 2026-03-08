//! CrystallizedSkill — a reusable skill extracted from observed patterns.

use serde::{Deserialize, Serialize};

/// Complexity of a crystallized skill
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillComplexity {
    Atomic,
    Composite,
    Complex,
}

/// What triggers a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillTrigger {
    /// Manually invoked by name
    Manual,
    /// Triggered by a pattern match in user input
    PatternMatch(String),
    /// Triggered by a context condition
    ContextCondition(String),
}

/// A crystallized skill: a repeatable, optimized action sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizedSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub actions: Vec<String>,
    pub trigger: SkillTrigger,
    pub complexity: SkillComplexity,
    pub source_pattern: String,
    pub confidence: f64,
    pub executions: u64,
    pub avg_duration_ms: f64,
    pub created_at: String,
}

impl CrystallizedSkill {
    /// Record an execution of this skill
    pub fn record_execution(&mut self, duration_ms: f64) {
        self.executions += 1;
        self.avg_duration_ms = (self.avg_duration_ms * (self.executions - 1) as f64 + duration_ms)
            / self.executions as f64;
    }

    /// Check if skill is mature enough for autonomous use
    pub fn is_mature(&self) -> bool {
        self.executions >= 5 && self.confidence >= 0.8
    }

    /// Get a summary of this skill
    pub fn summary(&self) -> String {
        format!(
            "{} ({} steps, {:.0}% confidence, {} executions)",
            self.name,
            self.actions.len(),
            self.confidence * 100.0,
            self.executions,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill() -> CrystallizedSkill {
        CrystallizedSkill {
            id: "skill-1".into(),
            name: "file_edit".into(),
            description: "Read, modify, and write a file".into(),
            actions: vec!["read".into(), "modify".into(), "write".into()],
            trigger: SkillTrigger::Manual,
            complexity: SkillComplexity::Composite,
            source_pattern: "pattern-1".into(),
            confidence: 0.9,
            executions: 0,
            avg_duration_ms: 0.0,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn test_skill_maturity() {
        let mut skill = make_skill();
        assert!(!skill.is_mature());

        for _ in 0..5 {
            skill.record_execution(100.0);
        }
        assert!(skill.is_mature());
    }

    #[test]
    fn test_skill_duration_tracking() {
        let mut skill = make_skill();
        skill.record_execution(100.0);
        skill.record_execution(200.0);
        assert!((skill.avg_duration_ms - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_skill_summary() {
        let skill = make_skill();
        let summary = skill.summary();
        assert!(summary.contains("file_edit"));
        assert!(summary.contains("3 steps"));
    }
}
