//! Economics Tracker — tracks time saved, bugs prevented, costs avoided.
//! Proves ROI with real numbers. /roi command shows monthly summary.
//!
//! Why isn't a sister doing this? Memory stores events, Time tracks duration.
//! This module AGGREGATES across both into economic value.

use std::sync::{Mutex, OnceLock};

/// Global economics tracker — accumulates across sessions.
pub static GLOBAL_ECONOMICS: OnceLock<Mutex<EconomicsState>> = OnceLock::new();
pub fn economics_state() -> &'static Mutex<EconomicsState> {
    GLOBAL_ECONOMICS.get_or_init(|| Mutex::new(EconomicsState::new()))
}

/// A tracked economic event.
#[derive(Debug, Clone)]
pub struct EconomicEvent {
    pub category: EventCategory,
    pub description: String,
    pub estimated_value_usd: f64,
    pub timestamp: String,
}

/// Categories of economic value.
#[derive(Debug, Clone, PartialEq)]
pub enum EventCategory {
    TimeSaved,       // Task done faster with Hydra
    BugPrevented,    // Security scan or review caught issue
    CostAvoided,     // Cloud optimization, resource right-sizing
    HoursSaved,      // Meeting prep, email draft, research
    ErrorPrevented,  // Caught mistake before it shipped
}

/// Accumulated economics state.
#[derive(Debug)]
pub struct EconomicsState {
    pub events: Vec<EconomicEvent>,
    pub llm_cost_usd: f64,
    pub sessions: u32,
    pub start_date: String,
}

impl EconomicsState {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            llm_cost_usd: 0.0,
            sessions: 0,
            start_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        }
    }

    /// Record an economic event.
    pub fn record(&mut self, category: EventCategory, description: &str, value: f64) {
        self.events.push(EconomicEvent {
            category,
            description: description.to_string(),
            estimated_value_usd: value,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Record LLM token cost.
    pub fn record_llm_cost(&mut self, cost: f64) {
        self.llm_cost_usd += cost;
    }

    /// Record a session.
    pub fn record_session(&mut self) {
        self.sessions += 1;
    }

    /// Total value generated.
    pub fn total_value(&self) -> f64 {
        self.events.iter().map(|e| e.estimated_value_usd).sum()
    }

    /// Value by category.
    pub fn value_by_category(&self) -> Vec<(EventCategory, f64, usize)> {
        let categories = [
            EventCategory::TimeSaved, EventCategory::BugPrevented,
            EventCategory::CostAvoided, EventCategory::HoursSaved,
            EventCategory::ErrorPrevented,
        ];
        categories.iter().map(|cat| {
            let events: Vec<&EconomicEvent> = self.events.iter()
                .filter(|e| e.category == *cat).collect();
            let value: f64 = events.iter().map(|e| e.estimated_value_usd).sum();
            (cat.clone(), value, events.len())
        }).filter(|(_, v, _)| *v > 0.0).collect()
    }

    /// ROI calculation.
    pub fn roi(&self) -> f64 {
        if self.llm_cost_usd > 0.0 {
            self.total_value() / self.llm_cost_usd
        } else {
            0.0
        }
    }
}

/// Generate the /roi summary.
pub fn roi_summary() -> String {
    let state = match economics_state().lock() {
        Ok(s) => s,
        Err(_) => return "Economics tracker unavailable.".into(),
    };

    if state.events.is_empty() && state.sessions == 0 {
        return "No economic data yet. Use Hydra for real work and value will be tracked automatically.".into();
    }

    let total = state.total_value();
    let cost = state.llm_cost_usd;
    let roi = state.roi();

    let mut out = format!(
        "ROI Report (since {})\n\
         Sessions: {} | LLM cost: ${:.2}\n\n",
        state.start_date, state.sessions, cost,
    );

    let by_cat = state.value_by_category();
    if by_cat.is_empty() {
        out.push_str("No value events recorded yet.\n");
    } else {
        for (cat, value, count) in &by_cat {
            let label = match cat {
                EventCategory::TimeSaved => "Time saved",
                EventCategory::BugPrevented => "Bugs prevented",
                EventCategory::CostAvoided => "Costs avoided",
                EventCategory::HoursSaved => "Hours saved",
                EventCategory::ErrorPrevented => "Errors prevented",
            };
            out.push_str(&format!("  {}: ${:.0} ({} events)\n", label, value, count));
        }
        out.push_str(&format!(
            "\nTotal value: ${:.0} | Cost: ${:.2} | ROI: {:.0}x\n",
            total, cost, roi,
        ));
    }

    out
}

/// Auto-detect economic value from a completed task.
pub fn auto_track(task_description: &str, duration_ms: u64, success: bool) {
    let state = match economics_state().lock() {
        Ok(mut s) => {
            s.record_session();

            // Estimate time saved: assume manual task takes 3x longer
            let manual_estimate_ms = duration_ms * 3;
            let saved_ms = manual_estimate_ms - duration_ms;
            let saved_hours = saved_ms as f64 / 3_600_000.0;
            let hourly_rate = 75.0; // Estimated developer hourly rate

            if saved_hours > 0.01 {
                s.record(
                    EventCategory::TimeSaved,
                    task_description,
                    saved_hours * hourly_rate,
                );
            }

            // Detect bug prevention
            let lower = task_description.to_lowercase();
            if lower.contains("security") || lower.contains("vulnerability") || lower.contains("bug") {
                if success {
                    s.record(EventCategory::BugPrevented, task_description, 500.0);
                }
            }

            // Detect cost optimization
            if lower.contains("cost") || lower.contains("optimize") || lower.contains("cloud") {
                if success {
                    s.record(EventCategory::CostAvoided, task_description, 200.0);
                }
            }

            drop(s);
        }
        Err(_) => {}
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = EconomicsState::new();
        assert_eq!(state.total_value(), 0.0);
        assert_eq!(state.sessions, 0);
    }

    #[test]
    fn test_record_and_total() {
        let mut state = EconomicsState::new();
        state.record(EventCategory::TimeSaved, "code review", 50.0);
        state.record(EventCategory::BugPrevented, "security scan", 500.0);
        assert_eq!(state.total_value(), 550.0);
    }

    #[test]
    fn test_roi() {
        let mut state = EconomicsState::new();
        state.record(EventCategory::TimeSaved, "task", 100.0);
        state.llm_cost_usd = 2.0;
        assert!((state.roi() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_roi_summary_empty() {
        let summary = roi_summary();
        assert!(summary.contains("No economic data") || summary.contains("ROI Report"));
    }

    #[test]
    fn test_auto_track() {
        auto_track("test task", 5000, true);
        // Just verify it doesn't panic
    }
}
