//! Tests for threat intelligence engine.

use super::*;
use chrono::Utc;

fn make_signal(source: SisterName, sig_type: SignalType, severity: f32) -> ThreatSignal {
    ThreatSignal {
        source,
        signal_type: sig_type,
        severity,
        timestamp: Utc::now(),
        details: "test signal".into(),
    }
}

// ── 1. Signal ingestion ──

#[test]
fn test_signal_ingestion() {
    let mut tc = ThreatCorrelator::new();
    tc.report_signal(make_signal(SisterName::Aegis, SignalType::InputAnomaly, 0.5));
    tc.report_signal(make_signal(SisterName::Memory, SignalType::MemoryTampering, 0.8));
    assert_eq!(tc.signal_count(), 2);
}

// ── 2. Signal expiry ──

#[test]
fn test_signal_expiry() {
    let mut tc = ThreatCorrelator::new();
    // Add a signal with an old timestamp
    let mut old_signal = make_signal(SisterName::Comm, SignalType::TrafficSpike, 0.3);
    old_signal.timestamp = Utc::now() - chrono::Duration::seconds(3700);
    tc.report_signal(old_signal);
    // Fresh signal
    tc.report_signal(make_signal(SisterName::Aegis, SignalType::InputAnomaly, 0.4));
    assert_eq!(tc.signal_count(), 1); // old one pruned
}

// ── 3. Single signal = no threat ──

#[test]
fn test_single_signal_no_threat() {
    let mut tc = ThreatCorrelator::new();
    tc.report_signal(make_signal(SisterName::Reality, SignalType::EnvironmentChange, 0.3));
    let assessments = tc.correlate();
    // One signal shouldn't trigger known patterns (all need 2+ sisters)
    assert!(assessments.is_empty());
}

// ── 4. Correlated signals = alert ──

#[test]
fn test_correlated_signals_threat() {
    let mut tc = ThreatCorrelator::new();
    // Prompt injection probe pattern: InputAnomaly + PolicyProbe from 2 sisters
    tc.report_signal(make_signal(SisterName::Aegis, SignalType::InputAnomaly, 0.7));
    tc.report_signal(make_signal(SisterName::Contract, SignalType::PolicyProbe, 0.8));
    let assessments = tc.correlate();
    assert!(!assessments.is_empty());
    assert!(assessments[0].matched_pattern.is_some());
    assert!(assessments[0].description.contains("Prompt Injection"));
}

// ── 5. Anomaly scoring ──

#[test]
fn test_anomaly_scoring() {
    let mut tc = ThreatCorrelator::new();
    // No signals = score 0
    assert_eq!(tc.anomaly_score(), 0.0);

    // Add several unusual signals
    for _ in 0..5 {
        tc.report_signal(make_signal(SisterName::Aegis, SignalType::DataExfiltration, 0.9));
    }
    let score = tc.anomaly_score();
    assert!(score > 0.3, "score should be elevated, got {}", score);
}

// ── 6. Response: log for low ──

#[test]
fn test_response_log_for_low() {
    let assessment = ThreatAssessment {
        threat_level: ThreatLevel::Low,
        description: "Minor anomaly".into(),
        contributing_signals: vec![],
        matched_pattern: None,
        confidence: 0.15,
    };
    let response = responder::respond(&assessment);
    assert_eq!(response, ThreatResponse::Log);
}

// ── 7. Response: alert for medium ──

#[test]
fn test_response_alert_for_medium() {
    let assessment = ThreatAssessment {
        threat_level: ThreatLevel::Medium,
        description: "Elevated activity".into(),
        contributing_signals: vec![],
        matched_pattern: None,
        confidence: 0.45,
    };
    let response = responder::respond(&assessment);
    match response {
        ThreatResponse::Alert(msg) => assert!(msg.contains("Elevated")),
        other => panic!("Expected Alert, got {:?}", other),
    }
}

// ── 8. Response: lockdown for critical ──

#[test]
fn test_response_lockdown_for_critical() {
    let assessment = ThreatAssessment {
        threat_level: ThreatLevel::Critical,
        description: "Full system compromise".into(),
        contributing_signals: vec![],
        matched_pattern: None,
        confidence: 0.95,
    };
    let response = responder::respond(&assessment);
    assert_eq!(response, ThreatResponse::Lockdown);
}

// ── 9. Known pattern detection ──

#[test]
fn test_known_pattern_detection() {
    let mut tc = ThreatCorrelator::new();
    // Data exfil pattern: ToolMisuse + DataExfiltration from 2 sisters
    tc.report_signal(make_signal(SisterName::Forge, SignalType::ToolMisuse, 0.7));
    tc.report_signal(make_signal(SisterName::Comm, SignalType::DataExfiltration, 0.9));
    let assessments = tc.correlate();
    let exfil = assessments.iter().find(|a| {
        a.matched_pattern.as_deref() == Some("Data Exfil Attempt")
    });
    assert!(exfil.is_some(), "Should detect data exfil pattern");
}

// ── 10. Coordinated attack: 3+ sisters in 5 seconds ──

#[test]
fn test_coordinated_attack_detection() {
    let mut tc = ThreatCorrelator::new();
    // 3 different sisters reporting within 5 seconds
    tc.report_signal(make_signal(SisterName::Aegis, SignalType::InputAnomaly, 0.6));
    tc.report_signal(make_signal(SisterName::Memory, SignalType::MemoryTampering, 0.7));
    tc.report_signal(make_signal(SisterName::Identity, SignalType::AuthFailure, 0.8));

    let signals: Vec<ThreatSignal> = tc.recent_signals(10)
        .into_iter().cloned().collect();
    let coordinated = correlator::detect_coordinated_attack(&signals, 5);
    assert!(coordinated.is_some());
    let ct = coordinated.unwrap();
    assert!(ct.sisters_involved.len() >= 3);
    assert_eq!(ct.signal_count, 3);
}

// ── Additional: ThreatLevel from score ──

#[test]
fn test_threat_level_from_score() {
    assert_eq!(ThreatLevel::from_score(0.0), ThreatLevel::None);
    assert_eq!(ThreatLevel::from_score(0.15), ThreatLevel::None);
    assert_eq!(ThreatLevel::from_score(0.25), ThreatLevel::Low);
    assert_eq!(ThreatLevel::from_score(0.45), ThreatLevel::Medium);
    assert_eq!(ThreatLevel::from_score(0.65), ThreatLevel::High);
    assert_eq!(ThreatLevel::from_score(0.85), ThreatLevel::Critical);
}

// ── Summary and display ──

#[test]
fn test_summary_empty() {
    let tc = ThreatCorrelator::new();
    let summary = tc.summary();
    assert!(summary.contains("0/10"));
    assert!(summary.contains("All clear"));
}

#[test]
fn test_signal_history_display() {
    let mut tc = ThreatCorrelator::new();
    tc.report_signal(make_signal(SisterName::Aegis, SignalType::InputAnomaly, 0.5));
    let history = tc.signal_history(10);
    assert!(history.contains("InputAnomaly"));
    assert!(history.contains("Aegis"));
}

#[test]
fn test_patterns_summary() {
    let tc = ThreatCorrelator::new();
    let summary = tc.patterns_summary();
    assert!(summary.contains("Prompt Injection"));
    assert!(summary.contains("Data Exfil"));
    assert!(summary.contains("Coordinated"));
}

// ── Response for high with behavioral drift ──

#[test]
fn test_response_freeze_for_behavioral_drift() {
    let assessment = ThreatAssessment {
        threat_level: ThreatLevel::High,
        description: "Behavioral anomaly".into(),
        contributing_signals: vec![
            make_signal(SisterName::Cognition, SignalType::BehavioralDrift, 0.7),
        ],
        matched_pattern: None,
        confidence: 0.7,
    };
    let response = responder::respond(&assessment);
    match response {
        ThreatResponse::FreezeAgent(agent) => assert!(agent.contains("Cognition")),
        other => panic!("Expected FreezeAgent, got {:?}", other),
    }
}

// ── Response tighten gates for high without drift ──

#[test]
fn test_response_tighten_for_high_no_drift() {
    let assessment = ThreatAssessment {
        threat_level: ThreatLevel::High,
        description: "Policy probing".into(),
        contributing_signals: vec![
            make_signal(SisterName::Contract, SignalType::PolicyProbe, 0.8),
        ],
        matched_pattern: None,
        confidence: 0.65,
    };
    let response = responder::respond(&assessment);
    assert_eq!(response, ThreatResponse::TightenGates);
}

// ── Buffer cap ──

#[test]
fn test_buffer_cap() {
    let mut tc = ThreatCorrelator::new();
    for _ in 0..1100 {
        tc.report_signal(make_signal(SisterName::Aegis, SignalType::InputAnomaly, 0.1));
    }
    assert!(tc.signal_count() <= 1000);
}
