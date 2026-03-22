//! Active context window — built from the current comprehended input.

use crate::window::{ContextItem, ContextWindow};
use hydra_comprehension::ComprehendedInput;

/// Build the active context window from a comprehended input.
///
/// Adds the raw input, domain signals, and temporal urgency as items.
pub fn build_active(input: &ComprehendedInput) -> ContextWindow {
    let mut window = ContextWindow::new("active");

    // The raw input itself is always the highest-significance item.
    window.add(ContextItem::with_domain(
        &input.raw,
        input.confidence,
        input.primary_domain.label(),
    ));

    // Add domain signals for each detected domain.
    for (domain, conf) in &input.all_domains {
        let content = format!("domain signal: {}", domain.label());
        window.add(ContextItem::with_domain(content, *conf, domain.label()));
    }

    // Add temporal urgency as an item if significant.
    if input.is_urgent() {
        window.add(ContextItem::new(
            format!("temporal urgency: {:.2}", input.temporal.urgency),
            input.temporal.urgency,
        ));
    }

    // Add primitive signals.
    for prim in &input.primitives {
        window.add(ContextItem::new(
            format!("primitive: {}", prim.label()),
            0.4,
        ));
    }

    window
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_comprehension::{
        ConstraintStatus, Domain, Horizon, InputSource, ResonanceResult, TemporalContext,
    };

    fn make_input(raw: &str, urgency: f64) -> ComprehendedInput {
        ComprehendedInput {
            raw: raw.to_string(),
            primary_domain: Domain::Engineering,
            all_domains: vec![(Domain::Engineering, 0.6)],
            primitives: vec![],
            temporal: TemporalContext {
                urgency,
                horizon: Horizon::ShortTerm,
                constraint_status: ConstraintStatus::None,
            },
            resonance: ResonanceResult::empty(),
            source: InputSource::PrincipalText,
            confidence: 0.7,
            used_llm: false,
        }
    }

    #[test]
    fn active_window_has_items() {
        let input = make_input("deploy the api service now", 0.8);
        let window = build_active(&input);
        assert!(!window.is_empty());
    }

    #[test]
    fn urgent_input_adds_urgency_item() {
        let input = make_input("critical outage now", 0.9);
        let window = build_active(&input);
        let has_urgency = window.items.iter().any(|i| i.content.contains("urgency"));
        assert!(has_urgency);
    }

    #[test]
    fn non_urgent_skips_urgency_item() {
        let input = make_input("plan to migrate later", 0.3);
        let window = build_active(&input);
        let has_urgency = window.items.iter().any(|i| i.content.contains("urgency"));
        assert!(!has_urgency);
    }
}
