//! Enrichment bridge — converts kernel enrichments into StreamItems.
//! No string parsing. Uses typed enrichment keys from the kernel.

use crate::dot::DotKind;
use crate::stream_types::StreamItem;
use std::collections::HashMap;

/// Convert kernel enrichments into stream items for display.
pub fn surface_enrichments(enrichments: &HashMap<String, String>) -> Vec<StreamItem> {
    let start = std::time::Instant::now();
    let mut items = Vec::new();

    // Memory context
    if let Some(memory) = enrichments.get("memory.context") {
        let evidence_lines: Vec<&str> = memory
            .lines()
            .filter(|l| l.trim().starts_with('[') && l.contains(']'))
            .collect();
        if !evidence_lines.is_empty() {
            items.push(tool_dot("Memory", DotKind::Read, &format!("{} exchanges recalled", evidence_lines.len())));
            // Show clean summaries, skip raw JSON/hashes
            for line in evidence_lines.iter().take(3) {
                let content = line
                    .trim()
                    .trim_start_matches(|c: char| c == '[' || c.is_ascii_digit() || c == ']' || c == ' ')
                    .trim_start_matches('•')
                    .trim();
                // Skip raw JSON objects and hash strings
                if content.starts_with('{') || content.starts_with("\"") || content.len() > 80 {
                    continue;
                }
                if content.len() >= 5 {
                    items.push(tool_connector(&truncate(content, 65)));
                }
            }
        }
    }

    // Genome enrichment
    if let Some(genome) = enrichments.get("genome") {
        let approach_count = genome.lines().filter(|l| l.contains("APPROACH")).count().max(1);
        items.push(tool_dot("Genome", DotKind::Cognitive, &format!("{approach_count} approaches matched")));
        // Extract approach names
        for line in genome.lines().filter(|l| l.contains("situation:")).take(3) {
            let content = line.trim().trim_start_matches("situation:").trim();
            if !content.is_empty() {
                items.push(tool_connector(&truncate(content, 70)));
            }
        }
    }

    // Genome identity (self-knowledge)
    if enrichments.contains_key("genome.identity") {
        items.push(tool_dot("Self-Knowledge", DotKind::Cognitive, "genome identity applied"));
    }

    // Calibration
    if let Some(cal) = enrichments.get("calibration") {
        items.push(tool_dot("Calibration", DotKind::Narration, "confidence adjusted"));
        if let Some(adj) = cal.lines().find(|l| l.contains("adjusted")) {
            items.push(tool_connector(adj.trim()));
        }
    }

    // Red team
    if let Some(rt) = enrichments.get("redteam") {
        let decision = if rt.contains("NoGo") {
            "NO-GO"
        } else if rt.contains("Mitigate") {
            "MITIGATE"
        } else {
            "GO"
        };
        items.push(tool_dot("RedTeam", DotKind::Error, &format!("threat analysis: {decision}")));
    }

    // Oracle
    if enrichments.contains_key("oracle") {
        items.push(tool_dot("Oracle", DotKind::Narration, "scenarios projected"));
    }

    // Omniscience gap
    if let Some(gap) = enrichments.get("omniscience.gap") {
        items.push(tool_dot("Knowledge", DotKind::Cognitive, &format!("gap: {}", truncate(gap, 60))));
    }

    // Surprise detection
    if let Some(surprise) = enrichments.get("surprise") {
        items.push(tool_dot("Surprise", DotKind::Active, &truncate(surprise, 60)));
    }

    // Browser relevance
    if enrichments.contains_key("browser_relevant") {
        items.push(tool_dot("Browser", DotKind::Active, "web context detected"));
    }

    // ── MISSING ENRICHMENTS (Sprint 2) ──

    // Judgment gate — only show REFUSED (approvals are noise)
    if let Some(judgment) = enrichments.get("judgment") {
        if judgment.contains("REFUSED") {
            items.push(tool_dot("Refused", DotKind::Error, &truncate(judgment, 35)));
        }
    }

    // Security threat — BLOCKED
    if let Some(threat) = enrichments.get("security.threat") {
        items.push(StreamItem::SystemNotification {
            id: uuid::Uuid::new_v4(),
            content: format!("\x07  ▲ SECURITY: {} — BLOCKED", truncate(threat, 50)),
            timestamp: chrono::Utc::now(),
        });
    }

    // Wisdom uncertainty — compact
    if let Some(wisdom) = enrichments.get("wisdom") {
        if wisdom.contains("uncertain") {
            items.push(tool_dot("Uncertain", DotKind::Narration, &truncate(wisdom, 30)));
        }
    }

    // Session weight — skip (too noisy for stream)
    // Scheduler fired — compact
    if let Some(sched) = enrichments.get("scheduler.fired") {
        items.push(tool_dot("Scheduler", DotKind::Active, &truncate(sched, 30)));
    }

    // Gift 9: Ambient noticing alerts
    if let Some(notice) = enrichments.get("noticing.surprise") {
        items.push(StreamItem::SystemNotification {
            id: uuid::Uuid::new_v4(),
            content: format!("  Noticed: {}", truncate(notice, 60)),
            timestamp: chrono::Utc::now(),
        });
    }

    if let Some(questions) = enrichments.get("hydra.questions") {
        items.push(StreamItem::SystemNotification {
            id: uuid::Uuid::new_v4(),
            content: format!("  ℹ Hydra's questions: {}", truncate(questions, 50)),
            timestamp: chrono::Utc::now(),
        });
    }

    // Zero-token resolution (compiled pattern)
    if let Some(compiled) = enrichments.get("zero_token") {
        items.push(StreamItem::SystemNotification {
            id: uuid::Uuid::new_v4(),
            content: format!("✓ [Compiled] {compiled} (0 tokens)"),
            timestamp: chrono::Utc::now(),
        });
    }

    // Gift 6: Enrichment timing
    if !items.is_empty() {
        let elapsed = start.elapsed();
        items.push(StreamItem::SystemNotification {
            id: uuid::Uuid::new_v4(),
            content: format!("  enriched in {:.0}ms", elapsed.as_secs_f64() * 1000.0),
            timestamp: chrono::Utc::now(),
        });
    }

    items
}

/// Create a cycle metadata line (shown after response). No provider name — it's in the top frame.
pub fn cycle_metadata(tokens: usize, duration_ms: u64, _provider: &str) -> StreamItem {
    let duration_s = duration_ms as f64 / 1000.0;
    let content = format!("[{duration_s:.1}s | {tokens} tok]");
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(),
        content,
        timestamp: chrono::Utc::now(),
    }
}

fn tool_dot(name: &str, kind: DotKind, description: &str) -> StreamItem {
    StreamItem::ToolDot {
        id: uuid::Uuid::new_v4(),
        tool_name: format!("{name} ▸ {description}"),
        kind,
        timestamp: chrono::Utc::now(),
    }
}

fn tool_connector(label: &str) -> StreamItem {
    StreamItem::ToolConnector {
        id: uuid::Uuid::new_v4(),
        label: label.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.min(s.len())])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_enrichments_produces_nothing() {
        let items = surface_enrichments(&HashMap::new());
        assert!(items.is_empty());
    }

    #[test]
    fn memory_enrichment_produces_dot() {
        let mut e = HashMap::new();
        e.insert(
            "memory.context".into(),
            "EVIDENCE:\n  [1] circuit breaker\n  [2] retry logic\nEND".into(),
        );
        let items = surface_enrichments(&e);
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| matches!(i, StreamItem::ToolDot { .. })));
    }

    #[test]
    fn genome_enrichment_produces_dot() {
        let mut e = HashMap::new();
        e.insert("genome".into(), "APPROACH 1\nsituation: circuit breaker".into());
        let items = surface_enrichments(&e);
        assert!(!items.is_empty());
    }

    #[test]
    fn cycle_metadata_formats() {
        let item = cycle_metadata(847, 12400, "anthropic");
        if let StreamItem::SystemNotification { content, .. } = item {
            assert!(content.contains("12.4s"));
            assert!(content.contains("847"));
        } else {
            panic!("Expected SystemNotification");
        }
    }
}
