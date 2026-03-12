//! Story-based progress and celebration component data.

use serde::{Deserialize, Serialize};

/// A single step in a progress journey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressStep {
    pub emoji: String,
    pub label: String,
    pub completed: bool,
    pub active: bool,
}

/// A multi-step progress journey with a running narrative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressJourney {
    pub steps: Vec<ProgressStep>,
    pub current_index: usize,
    pub narrative: String,
}

/// Celebration intensity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CelebrationSize {
    Small,
    Medium,
    Big,
}

/// A celebration displayed when work completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Celebration {
    pub size: CelebrationSize,
    pub message: String,
    pub emoji: String,
    pub stats: Option<CelebrationStats>,
}

/// Optional stats shown in medium/big celebrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CelebrationStats {
    pub duration: String,
    pub steps_completed: usize,
    pub tokens_used: u64,
}

impl ProgressJourney {
    /// Create a journey from (emoji, label) pairs. The first step is active.
    pub fn new(steps: Vec<(&str, &str)>) -> Self {
        let steps: Vec<ProgressStep> = steps
            .into_iter()
            .enumerate()
            .map(|(i, (emoji, label))| ProgressStep {
                emoji: emoji.to_owned(),
                label: label.to_owned(),
                completed: false,
                active: i == 0,
            })
            .collect();
        Self {
            steps,
            current_index: 0,
            narrative: String::new(),
        }
    }

    /// Advance to the next step, marking the current one as completed.
    pub fn advance(&mut self, narrative: &str) {
        if self.current_index < self.steps.len() {
            self.steps[self.current_index].completed = true;
            self.steps[self.current_index].active = false;
        }
        self.current_index += 1;
        if self.current_index < self.steps.len() {
            self.steps[self.current_index].active = true;
        }
        self.narrative = narrative.to_owned();
    }

    /// Mark all remaining steps as completed.
    pub fn complete(&mut self) {
        for step in &mut self.steps {
            step.completed = true;
            step.active = false;
        }
        self.current_index = self.steps.len();
    }

    /// The currently active step, if any.
    pub fn current_step(&self) -> Option<&ProgressStep> {
        self.steps.get(self.current_index)
    }

    /// Visual trail of step status icons.
    pub fn emoji_trail(&self) -> String {
        self.steps
            .iter()
            .map(|s| {
                if s.completed {
                    "\u{2713}" // checkmark
                } else if s.active {
                    "\u{25C9}" // active circle
                } else {
                    "\u{25CB}" // empty circle
                }
            })
            .collect::<Vec<_>>()
            .join(" \u{2192} ") // arrow
    }

    /// Completion percentage (0.0..=100.0).
    pub fn percentage(&self) -> f32 {
        if self.steps.is_empty() {
            return 0.0;
        }
        let done = self.steps.iter().filter(|s| s.completed).count();
        (done as f32 / self.steps.len() as f32) * 100.0
    }
}

impl Celebration {
    /// Quick "Done!" celebration.
    pub fn small(message: &str) -> Self {
        Self {
            size: CelebrationSize::Small,
            message: message.to_owned(),
            emoji: "\u{2713}".into(), // check mark
            stats: None,
        }
    }

    /// Medium celebration with stats.
    pub fn medium(message: &str, stats: CelebrationStats) -> Self {
        Self {
            size: CelebrationSize::Medium,
            message: message.to_owned(),
            emoji: "\u{2605}".into(), // black star
            stats: Some(stats),
        }
    }

    /// Big celebration with star and stats.
    pub fn big(message: &str, stats: CelebrationStats) -> Self {
        Self {
            size: CelebrationSize::Big,
            message: message.to_owned(),
            emoji: "\u{2736}".into(), // six-pointed star
            stats: Some(stats),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journey_creation() {
        let j = ProgressJourney::new(vec![("E", "Find"), ("W", "Write"), ("S", "Send")]);
        assert_eq!(j.steps.len(), 3);
        assert!(j.steps[0].active);
        assert!(!j.steps[1].active);
        assert_eq!(j.current_index, 0);
    }

    #[test]
    fn test_advance() {
        let mut j = ProgressJourney::new(vec![("E", "Find"), ("W", "Write")]);
        j.advance("Found the email");
        assert!(j.steps[0].completed);
        assert!(j.steps[1].active);
        assert_eq!(j.current_index, 1);
        assert_eq!(j.narrative, "Found the email");
    }

    #[test]
    fn test_complete() {
        let mut j = ProgressJourney::new(vec![("A", "X"), ("B", "Y")]);
        j.complete();
        assert!(j.steps.iter().all(|s| s.completed));
        assert!(j.steps.iter().all(|s| !s.active));
        assert!(j.current_step().is_none());
    }

    #[test]
    fn test_emoji_trail() {
        let mut j = ProgressJourney::new(vec![("A", "X"), ("B", "Y"), ("C", "Z")]);
        j.advance("done with X");
        let trail = j.emoji_trail();
        assert!(trail.contains("\u{2713}")); // completed
        assert!(trail.contains("\u{25C9}")); // active
        assert!(trail.contains("\u{25CB}")); // pending
    }

    #[test]
    fn test_percentage() {
        let mut j = ProgressJourney::new(vec![("A", "X"), ("B", "Y"), ("C", "Z"), ("D", "W")]);
        assert_eq!(j.percentage(), 0.0);
        j.advance("one");
        assert_eq!(j.percentage(), 25.0);
        j.complete();
        assert_eq!(j.percentage(), 100.0);
    }

    #[test]
    fn test_percentage_empty() {
        let j = ProgressJourney::new(vec![]);
        assert_eq!(j.percentage(), 0.0);
    }

    #[test]
    fn test_celebration_small() {
        let c = Celebration::small("Done!");
        assert_eq!(c.size, CelebrationSize::Small);
        assert!(c.stats.is_none());
    }

    #[test]
    fn test_celebration_medium() {
        let stats = CelebrationStats {
            duration: "2m 30s".into(),
            steps_completed: 5,
            tokens_used: 1200,
        };
        let c = Celebration::medium("Great work!", stats);
        assert_eq!(c.size, CelebrationSize::Medium);
        assert!(c.stats.is_some());
    }

    #[test]
    fn test_celebration_big() {
        let stats = CelebrationStats {
            duration: "10m".into(),
            steps_completed: 12,
            tokens_used: 5000,
        };
        let c = Celebration::big("Amazing!", stats);
        assert_eq!(c.size, CelebrationSize::Big);
        let s = c.stats.unwrap();
        assert_eq!(s.steps_completed, 12);
    }
}
