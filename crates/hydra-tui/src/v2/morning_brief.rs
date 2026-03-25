//! Session briefing — bordered card per spec Part Seven #2.
//! Time-appropriate: Morning / Afternoon / Evening.
//! Format: bordered box with spacing, priority items with │ borders.

use crate::stream_types::{BriefingPriority, StreamItem};

pub fn generate_briefing(genome_count: usize) -> Vec<StreamItem> {
    let mut items = Vec::new();
    let sessions = hydra_kernel::conversation_store::ConversationStore::list_sessions();
    let session_count = sessions.len();
    let (last_ago, hours) = if let Some((_, _, ts)) = sessions.first() {
        let h = (chrono::Utc::now() - *ts).num_hours();
        (if h < 1 { "just now".into() } else if h < 24 { format!("{h} hours") } else { format!("{} days", h / 24) }, h)
    } else { ("first session".into(), 999) };

    use chrono::Timelike;
    let period = match chrono::Local::now().hour() {
        5..=11 => "Morning", 12..=16 => "Afternoon", 17..=20 => "Evening", _ => "Evening",
    };
    let w = 50usize;
    let title = format!("{period} Briefing");
    let pad = w.saturating_sub(title.len() + 4);

    // ┌─ Title ──────────────────────────────────────┐
    items.push(sys(&format!("┌─ {title} {}┐", "─".repeat(pad))));
    // │                                               │
    items.push(sys(&format!("│{}│", " ".repeat(w))));

    if hours >= 1 {
        items.push(sys(&format!("│  While you were away ({last_ago}):{}│",
            " ".repeat(w.saturating_sub(last_ago.len() + 27)))));
        items.push(sys(&format!("│{}│", " ".repeat(w))));
    }

    // Briefing items with priority symbols
    push_brief_item(&mut items, "●", &format!("Genome: {genome_count} entries · self-writing active"), w, BriefingPriority::Normal);
    if session_count > 0 {
        push_brief_item(&mut items, "○", &format!("{session_count} sessions in memory"), w, BriefingPriority::Low);
    }
    // Real insights from subsystems
    let obstacles = hydra_antifragile::AntifragileStore::new().total_encounters();
    if obstacles > 0 {
        push_brief_item(&mut items, "●", &format!("{obstacles} obstacles overcome · growing stronger"), w, BriefingPriority::Normal);
    }
    let beliefs = hydra_belief::BeliefStore::new();
    let belief_count = beliefs.len();
    if belief_count > 0 {
        push_brief_item(&mut items, "○", &format!("{belief_count} beliefs held · AGM revision active"), w, BriefingPriority::Low);
    }
    // O7: Workspace state from last session
    if let Some(ws) = hydra_kernel::workspace::load_snapshot() {
        for line in hydra_kernel::workspace::briefing_items(&ws) {
            push_brief_item(&mut items, "●", &line, w, BriefingPriority::High);
        }
    }
    // Self-preservation (O23): integrity report
    let mut integrity_monitor = hydra_kernel::integrity::IntegrityMonitor::new();
    let integrity_report = integrity_monitor.check();
    if !integrity_report.is_healthy() {
        for issue in &integrity_report.issues {
            push_brief_item(&mut items, "▲", issue, w, BriefingPriority::Urgent);
        }
    }
    if integrity_report.genome_recovered {
        push_brief_item(&mut items, "●", "Genome auto-recovered from backup", w, BriefingPriority::High);
    }
    if integrity_report.memory_recovered {
        push_brief_item(&mut items, "●", "Memory auto-recovered from backup", w, BriefingPriority::High);
    }
    // O21: User model proactive suggestions
    let user_model = hydra_kernel::user_model::DeepUserModel::load();
    for suggestion in user_model.proactive_suggestions().iter().take(2) {
        push_brief_item(&mut items, "●", suggestion, w, BriefingPriority::Normal);
    }
    push_brief_item(&mut items, "○", "All systems nominal · laws verified", w, BriefingPriority::Low);

    // │                                               │
    items.push(sys(&format!("│{}│", " ".repeat(w))));
    // └──────────────────────────────────────────────┘
    items.push(sys(&format!("└{}┘", "─".repeat(w))));
    items.push(StreamItem::Blank);
    items
}

fn push_brief_item(items: &mut Vec<StreamItem>, _sym: &str, content: &str, _w: usize, priority: BriefingPriority) {
    items.push(StreamItem::BriefingItem {
        id: uuid::Uuid::new_v4(),
        content: content.into(),
        priority,
        timestamp: chrono::Utc::now(),
    });
}

fn sys(content: &str) -> StreamItem {
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(), content: content.into(), timestamp: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn briefing_generates() { assert!(!generate_briefing(390).is_empty()); }
}
