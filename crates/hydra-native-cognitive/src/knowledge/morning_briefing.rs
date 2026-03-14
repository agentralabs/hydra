//! Morning Briefing — aggregates all watcher results since last session,
//! generates prioritized summary, shows BEFORE user types anything.
//!
//! Why isn't a sister doing this? This aggregates across all sisters and
//! the awareness mesh. No single sister has the full picture.

use super::awareness_mesh::{self, AlertSeverity, AwarenessAlert};
use crate::cognitive::temporal_fabric;
use crate::cognitive::ecosystem_monitor;
use hydra_native_state::operational_profile::ProfileBelief;

/// A complete morning briefing.
#[derive(Debug, Clone)]
pub struct MorningBriefing {
    pub greeting: String,
    pub urgent_items: Vec<BriefingItem>,
    pub important_items: Vec<BriefingItem>,
    pub info_items: Vec<BriefingItem>,
    pub belief_insights: Vec<String>,
    pub all_quiet: bool,
}

/// A single item in the briefing.
#[derive(Debug, Clone)]
pub struct BriefingItem {
    pub title: String,
    pub detail: String,
    pub source: String,
}

/// Generate the morning briefing from accumulated awareness data + beliefs.
pub fn generate(
    user_name: &str,
    beliefs: &[ProfileBelief],
    profile_name: &str,
) -> MorningBriefing {
    // Drain alerts from awareness mesh
    let alerts = if let Ok(mut state) = awareness_mesh::awareness_state().lock() {
        state.drain_alerts()
    } else {
        Vec::new()
    };

    // Categorize alerts by severity
    let mut urgent = Vec::new();
    let mut important = Vec::new();
    let mut info = Vec::new();

    for alert in &alerts {
        let item = BriefingItem {
            title: alert.title.clone(),
            detail: alert.detail.clone(),
            source: alert.watcher_name.clone(),
        };
        match alert.severity {
            AlertSeverity::Urgent => urgent.push(item),
            AlertSeverity::Important => important.push(item),
            AlertSeverity::Info => info.push(item),
        }
    }

    // Belief ecosystem insights
    let mut belief_insights = Vec::new();
    if !beliefs.is_empty() {
        let health = ecosystem_monitor::assess_health(beliefs);
        for w in &health.warnings {
            belief_insights.push(w.clone());
        }
        for s in &health.blind_spots {
            belief_insights.push(s.clone());
        }
    }

    // Temporal fabric insights — declining beliefs
    if let Ok(store) = temporal_fabric::temporal_store().lock() {
        let declining = store.declining_beliefs();
        for tl in declining.iter().take(3) {
            let latest = tl.snapshots.last().map(|s| s.confidence).unwrap_or(0.0);
            belief_insights.push(format!(
                "Belief '{}' confidence declining — now {:.0}%",
                tl.topic, latest * 100.0,
            ));
        }
    }

    let all_quiet = urgent.is_empty() && important.is_empty()
        && info.is_empty() && belief_insights.is_empty();

    let greeting = build_greeting(user_name, profile_name, &alerts);

    MorningBriefing {
        greeting,
        urgent_items: urgent,
        important_items: important,
        info_items: info,
        belief_insights,
        all_quiet,
    }
}

/// Format the briefing as a displayable string.
pub fn format_briefing(briefing: &MorningBriefing) -> String {
    let mut output = format!("{}\n\n", briefing.greeting);

    if briefing.all_quiet {
        output.push_str("All quiet. No issues detected since your last session.\n");
        return output;
    }

    let total = briefing.urgent_items.len()
        + briefing.important_items.len()
        + briefing.info_items.len();

    output.push_str(&format!("{} items since your last session:\n\n", total));

    if !briefing.urgent_items.is_empty() {
        output.push_str("URGENT:\n");
        for item in &briefing.urgent_items {
            output.push_str(&format!("  {} — {}\n", item.title, item.detail));
        }
        output.push('\n');
    }

    if !briefing.important_items.is_empty() {
        output.push_str("IMPORTANT:\n");
        for item in &briefing.important_items {
            output.push_str(&format!("  {} — {}\n", item.title, item.detail));
        }
        output.push('\n');
    }

    if !briefing.info_items.is_empty() {
        output.push_str("INFO:\n");
        for item in &briefing.info_items {
            output.push_str(&format!("  {} — {}\n", item.title, item.detail));
        }
        output.push('\n');
    }

    if !briefing.belief_insights.is_empty() {
        output.push_str("KNOWLEDGE INSIGHTS:\n");
        for insight in &briefing.belief_insights {
            output.push_str(&format!("  {}\n", insight));
        }
    }

    output
}

/// Build a contextual greeting based on time of day.
fn build_greeting(user_name: &str, profile_name: &str, alerts: &[AwarenessAlert]) -> String {
    let hour = chrono::Local::now().hour();
    let time_greeting = match hour {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=21 => "Good evening",
        _ => "Hello",
    };

    let name_part = if user_name.is_empty() {
        String::new()
    } else {
        format!(", {}", user_name)
    };

    let urgency = if alerts.iter().any(|a| a.severity == AlertSeverity::Urgent) {
        " Urgent items need attention."
    } else {
        ""
    };

    format!(
        "{}{}.{} Profile: {}.",
        time_greeting, name_part, urgency, profile_name,
    )
}

use chrono::Timelike;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_briefing() {
        let briefing = generate("test_user", &[], "dev");
        assert!(briefing.all_quiet);
        let formatted = format_briefing(&briefing);
        assert!(formatted.contains("All quiet"));
    }

    #[test]
    fn test_format_briefing_with_items() {
        let briefing = MorningBriefing {
            greeting: "Good morning.".into(),
            urgent_items: vec![BriefingItem {
                title: "Server down".into(),
                detail: "prod-1 not responding".into(),
                source: "endpoint_health".into(),
            }],
            important_items: vec![],
            info_items: vec![BriefingItem {
                title: "New commits".into(),
                detail: "3 commits pushed".into(),
                source: "git_changes".into(),
            }],
            belief_insights: vec![],
            all_quiet: false,
        };
        let formatted = format_briefing(&briefing);
        assert!(formatted.contains("URGENT"));
        assert!(formatted.contains("Server down"));
        assert!(formatted.contains("INFO"));
    }

    #[test]
    fn test_greeting_includes_profile() {
        let alerts = vec![];
        let greeting = build_greeting("Alice", "finance", &alerts);
        assert!(greeting.contains("Alice"));
        assert!(greeting.contains("finance"));
    }
}
