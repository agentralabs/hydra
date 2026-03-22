//! Enrichment bridge — converts kernel enrichments to StreamItems + Alerts.
//!
//! This is the data pipeline from kernel intelligence to TUI surface.
//! Every enrichment key maps to a specific visual representation.

use std::collections::HashMap;

use crate::alert::{self, Alert, AlertLevel};
use crate::dot::DotKind;
use crate::stream_types::StreamItem;

/// Result of processing enrichments from a cognitive cycle.
pub struct EnrichmentSurface {
    /// Stream items to push (tool dots, notifications).
    pub items: Vec<StreamItem>,
    /// Alerts to trigger (frame or emergency).
    pub alerts: Vec<Alert>,
}

/// Process kernel enrichments into TUI-visible elements.
pub fn surface_enrichments(
    enrichments: &HashMap<String, String>,
    tokens_used: usize,
) -> EnrichmentSurface {
    let mut items = Vec::new();
    let mut alerts = Vec::new();

    // Memory recall — show what was recalled
    if let Some(memory) = enrichments.get("memory.context") {
        let facts: Vec<&str> = memory
            .lines()
            .filter(|l| l.trim_start().starts_with('•'))
            .collect();
        let fact_count = facts.len();
        if fact_count > 0 {
            items.push(tool_dot(
                DotKind::Read,
                "Memory",
                &format!("recalled {fact_count} prior exchanges"),
            ));
            // Show up to 3 recalled facts as connectors
            for fact in facts.iter().take(3) {
                let clean = fact.trim_start_matches('•').trim();
                let short: String = clean.chars().take(80).collect();
                items.push(tool_connector(&short));
            }
            if fact_count > 3 {
                items.push(tool_connector(&format!("... +{} more", fact_count - 3)));
            }
        }
    }

    // Genome match — show which approaches matched
    if let Some(genome) = enrichments.get("genome") {
        let approach_count = genome.matches("conf=").count().max(1);
        items.push(tool_dot(
            DotKind::Cognitive,
            "Genome",
            &format!("matched {approach_count} proven approaches"),
        ));
        // Show up to 2 matched approach names as connectors
        for line in genome.lines().take(2) {
            let short: String = line.trim().chars().take(80).collect();
            if !short.is_empty() {
                items.push(tool_connector(&short));
            }
        }

        // Extract confidence for belief citation box
        if let Some(conf_start) = genome.find("conf=") {
            let conf_str = &genome[conf_start + 5..];
            let conf_end = conf_str.find('%').unwrap_or(conf_str.len().min(4));
            if let Ok(conf_pct) = conf_str[..conf_end].parse::<f64>() {
                let confidence = conf_pct / 100.0;
                let belief_text: String = genome
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(60)
                    .collect();
                if !belief_text.is_empty() {
                    items.push(StreamItem::BeliefCitation {
                        id: uuid::Uuid::new_v4(),
                        belief: belief_text,
                        confidence,
                    });
                }
            }
        }
    }

    // Calibration
    if let Some(cal) = enrichments.get("calibration") {
        if !cal.contains("no adjustment") {
            items.push(tool_dot(
                DotKind::Narration,
                "Calibration",
                "confidence adjusted",
            ));
        }
    }

    // Red team analysis
    if let Some(redteam) = enrichments.get("redteam") {
        let alert_level = alert::classify_enrichment("redteam", redteam);
        if alert_level >= AlertLevel::Frame {
            alerts.push(Alert::new(alert_level, format!("RedTeam: {redteam}")));
        }
        let label = if redteam.contains("NO-GO") {
            "NO-GO — threats detected"
        } else if redteam.contains("0 threats") || redteam.contains("GO") {
            "0 threats (GO)"
        } else {
            redteam.as_str()
        };
        items.push(tool_dot(DotKind::Active, "RedTeam", label));
    }

    // Oracle projections
    if let Some(oracle) = enrichments.get("oracle") {
        if oracle.contains("adverse") {
            items.push(tool_dot(
                DotKind::Narration,
                "Oracle",
                "adverse scenarios projected",
            ));
        }
    }

    // Security threats — EMERGENCY
    if let Some(threat) = enrichments.get("security.threat") {
        alerts.push(Alert::new(
            AlertLevel::Emergency,
            format!("SECURITY: {threat}"),
        ));
        items.push(tool_dot(DotKind::Error, "Security", threat));
    }

    // Surprise detection
    if let Some(surprise) = enrichments.get("surprise") {
        items.push(sys_notification(&format!("Surprise: {surprise}")));
    }

    // Hydra has questions for the user
    if let Some(questions) = enrichments.get("hydra.questions") {
        items.push(sys_notification(questions));
    }

    // Knowledge gap
    if let Some(gap) = enrichments.get("omniscience.gap") {
        items.push(sys_notification(&format!("Knowledge gap: {gap}")));
    }

    // Compiled pattern match (0 tokens — genome answered without LLM)
    if tokens_used == 0 {
        if let Some(genome) = enrichments.get("genome") {
            let short = genome
                .lines()
                .next()
                .unwrap_or("pattern match")
                .chars()
                .take(60)
                .collect::<String>();
            items.push(tool_dot(
                DotKind::Success,
                "[Compiled]",
                &format!("{short} (0 tokens)"),
            ));
        }
    }

    // Truncation — check if any enrichment value is excessively long
    for (key, value) in enrichments {
        let line_count = value.lines().count();
        if line_count > 50 {
            let shown: String = value.lines().take(5).collect::<Vec<&str>>().join("\n");
            items.push(tool_dot(DotKind::Narration, key, &format!("{} lines", line_count)));
            items.push(tool_connector(&shown));
            items.push(StreamItem::Truncation {
                id: uuid::Uuid::new_v4(),
                chars_truncated: value.len(),
            });
        }
    }

    EnrichmentSurface { items, alerts }
}

fn tool_connector(label: &str) -> StreamItem {
    StreamItem::ToolConnector {
        id: uuid::Uuid::new_v4(),
        label: label.to_string(),
    }
}

fn tool_dot(kind: DotKind, tool_name: &str, label: &str) -> StreamItem {
    StreamItem::ToolDot {
        id: uuid::Uuid::new_v4(),
        tool_name: format!("{tool_name} ▸ {label}"),
        kind,
        timestamp: chrono::Utc::now(),
    }
}

fn sys_notification(content: &str) -> StreamItem {
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(),
        content: content.to_string(),
        timestamp: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_enrichment_creates_tool_dot_with_connectors() {
        let mut enrichments = HashMap::new();
        enrichments.insert(
            "memory.context".into(),
            "• fact one\n• fact two\n• fact three".into(),
        );
        let surface = surface_enrichments(&enrichments, 100);
        // 1 ToolDot + 3 ToolConnectors
        assert_eq!(surface.items.len(), 4);
        match &surface.items[0] {
            StreamItem::ToolDot { tool_name, .. } => {
                assert!(tool_name.contains("recalled 3"));
            }
            _ => panic!("expected ToolDot first"),
        }
    }

    #[test]
    fn security_threat_creates_emergency_alert() {
        let mut enrichments = HashMap::new();
        enrichments.insert(
            "security.threat".into(),
            "BLOCKED: prompt injection attempt".into(),
        );
        let surface = surface_enrichments(&enrichments, 100);
        assert!(!surface.alerts.is_empty());
        assert_eq!(surface.alerts[0].level, AlertLevel::Emergency);
    }

    #[test]
    fn zero_tokens_creates_compiled_pattern() {
        let mut enrichments = HashMap::new();
        enrichments.insert("genome".into(), "circuit-breaker conf=91%".into());
        let surface = surface_enrichments(&enrichments, 0);
        let compiled = surface.items.iter().any(|item| match item {
            StreamItem::ToolDot { tool_name, .. } => tool_name.contains("[Compiled]"),
            _ => false,
        });
        assert!(compiled);
    }
}
