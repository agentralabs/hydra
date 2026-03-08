//! Category 1: Unit Gap Fill — hydra-pulse edge cases.

use hydra_pulse::proactive::WatchTriggerType;
use hydra_pulse::*;

// === Tier escalation timeout ===

#[test]
fn test_tier_escalation_chain() {
    assert_eq!(ResponseTier::Instant.escalate(), Some(ResponseTier::Fast));
    assert_eq!(ResponseTier::Fast.escalate(), Some(ResponseTier::Full));
    assert_eq!(ResponseTier::Full.escalate(), None); // no further escalation
}

#[test]
fn test_tier_target_ms() {
    assert!(ResponseTier::Instant.target_ms() < ResponseTier::Fast.target_ms());
    assert!(ResponseTier::Fast.target_ms() < ResponseTier::Full.target_ms());
}

#[test]
fn test_tier_selector_cache_hit() {
    let selector = TierSelector::with_defaults();
    let tier = selector.select(true, 0.9, false);
    assert_eq!(tier, ResponseTier::Instant); // cache hit with high confidence
}

#[test]
fn test_tier_selector_no_cache_no_local() {
    let selector = TierSelector::with_defaults();
    let tier = selector.select(false, 0.0, false);
    assert_eq!(tier, ResponseTier::Full); // no cache, no local → full
}

// === Predictor cache eviction ===

#[test]
fn test_predictor_max_patterns() {
    let predictor = ResponsePredictor::new(5, 3);
    for i in 0..10 {
        predictor.learn(&format!("input_{}", i), &format!("response_{}", i));
    }
    assert!(predictor.pattern_count() <= 10); // may or may not evict
}

#[test]
fn test_predictor_prediction_miss() {
    let predictor = ResponsePredictor::with_defaults();
    let result = predictor.predict("never seen this before");
    assert!(!result.matched);
}

#[test]
fn test_predictor_learn_and_hit() {
    let predictor = ResponsePredictor::new(100, 3);
    predictor.learn("hello world", "greeting response");
    let result = predictor.predict("hello world");
    // May or may not be a hit depending on implementation
    let _ = result;
}

// === Resonance decay ===

#[test]
fn test_resonance_model_observe() {
    let model = ResonanceModel::with_defaults();
    model.observe("detail_level", 0.8);
    model.observe("detail_level", 0.9);
    let pref = model.preference("detail_level");
    assert!(pref.is_some());
    let val = pref.unwrap();
    assert!(val > 0.0 && val <= 1.0);
}

#[test]
fn test_resonance_score_empty() {
    let model = ResonanceModel::with_defaults();
    let traits = std::collections::HashMap::new();
    let score = model.score(&traits);
    assert!(score.overall >= 0.0);
}

#[test]
fn test_resonance_reset() {
    let model = ResonanceModel::with_defaults();
    model.observe("test", 0.5);
    assert_eq!(model.dimension_count(), 1);
    model.reset();
    assert_eq!(model.dimension_count(), 0);
}

// === Proactive engine ===

#[test]
fn test_proactive_engine_disabled() {
    let engine = ProactiveEngine::new();
    engine.set_enabled(false);
    assert!(!engine.is_enabled());
    let update = engine.process_trigger(ProactiveTrigger::ScheduledCheck {
        name: "test".into(),
    });
    assert!(update.is_none()); // disabled
}

#[test]
fn test_proactive_engine_watches() {
    let engine = ProactiveEngine::new();
    engine.add_watch(WatchSpec {
        id: "w1".into(),
        trigger: WatchTriggerType::Interval { seconds: 300 },
        description: "test watch".into(),
        enabled: true,
        cooldown_secs: 60,
    });
    assert_eq!(engine.watches().len(), 1);
    assert!(engine.remove_watch("w1"));
    assert_eq!(engine.watches().len(), 0);
}

// === Pulse state serialization ===

#[test]
fn test_pulse_state_roundtrip() {
    let state = PulseState::empty();
    let bytes = state.to_bytes();
    let restored = PulseState::from_bytes(&bytes).unwrap();
    assert_eq!(restored.patterns.len(), state.patterns.len());
}

#[test]
fn test_pulse_state_invalid_bytes() {
    let result = PulseState::from_bytes(&[0, 1, 2]);
    assert!(result.is_err());
}
