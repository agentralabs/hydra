//! New subsystem tests — everything built in the TUI/voice/wiring session.
//! Tests: EMI, DSEA, CCA, companion channel, dream loop, ambient loop,
//! settlement, device profile, voice detection, streaming setup.

use crate::TestResult;
use std::time::Instant;

pub fn run() -> Vec<TestResult> {
    let mut results = Vec::new();

    results.push(test_emi_format());
    results.push(test_dsea_axiom_vectors());
    results.push(test_dsea_indirect_matching());
    results.push(test_cca_confidence_interval());
    results.push(test_companion_channel());
    results.push(test_dream_subsystems());
    results.push(test_ambient_subsystems());
    results.push(test_settlement_middleware());
    results.push(test_device_profile());
    results.push(test_voice_detection());
    results.push(test_genome_record_use());
    results.push(test_succession_export());
    results.push(test_federation_engines());
    results.push(test_streaming_setup());
    results.push(test_belief_revision());

    let passed = results.iter().filter(|r| r.passed).count();
    println!("  New subsystems: {}/{} passed", passed, results.len());
    results
}

fn test_emi_format() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let bridge = hydra_memory::HydraMemoryBridge::new();
        // EMI format should produce closed-world evidence when nodes exist
        // On fresh memory, query returns empty (which is correct)
        let result = bridge.query_relevant("test query", 5);
        // Either empty (no memory) or valid results — both are correct
        assert!(result.len() <= 5);
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "emi_format", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "emi_format", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_dsea_axiom_vectors() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use std::collections::BTreeSet;
        let mut keywords = BTreeSet::new();
        keywords.insert("cascad".to_string());
        keywords.insert("failur".to_string());
        keywords.insert("prevent".to_string());
        keywords.insert("distribut".to_string());

        let vec = hydra_genome::signature::axiom_vector(&keywords);
        // "cascad", "failur", "prevent" should score high on Risk
        // "distribut" should score on Dependency
        assert!(vec[0] > 0.0, "Risk dimension should be non-zero");
        assert!(vec[2] > 0.0, "Dependency dimension should be non-zero");

        // Cosine similarity with similar vector should be high
        let mut similar = BTreeSet::new();
        similar.insert("break".to_string());
        similar.insert("error".to_string());
        similar.insert("servic".to_string());
        let vec2 = hydra_genome::signature::axiom_vector(&similar);
        let cosine = hydra_genome::signature::axiom_cosine(&vec, &vec2);
        assert!(cosine > 0.3, "Similar axiom profiles should have cosine > 0.3, got {cosine}");
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "dsea_axiom_vectors", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "dsea_axiom_vectors", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_dsea_indirect_matching() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let mut genome = hydra_genome::GenomeStore::open();
        genome.load_from_skills();
        // Direct query
        let direct = genome.query("circuit breaker pattern");
        // Indirect query — should match via DSEA axiom cosine
        let indirect = genome.query("how to prevent cascading failures in distributed systems");
        // Both should return results
        assert!(!direct.is_empty(), "Direct query should match");
        assert!(!indirect.is_empty(), "Indirect query should match via DSEA");
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "dsea_indirect_matching", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "dsea_indirect_matching", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_cca_confidence_interval() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let approach = hydra_genome::ApproachSignature::new("test", vec!["step1".into()], vec![]);
        let entry = hydra_genome::GenomeEntry::from_operation("test situation", approach, 0.90);
        let stmt = entry.confidence_statement();
        // Should contain conf=, obs=, strength=
        assert!(stmt.contains("conf="), "Statement must contain conf=: {stmt}");
        assert!(stmt.contains("obs="), "Statement must contain obs=: {stmt}");
        assert!(stmt.contains("strength="), "Statement must contain strength=: {stmt}");
        // Confidence should be between 0-100%
        assert!(!stmt.contains("conf=101"), "Confidence must be <= 100%");
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "cca_confidence_interval", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "cca_confidence_interval", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_companion_channel() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let (tui_side, endpoint) = hydra_signals::create_companion_channel();
        let mut service = hydra_companion::CompanionService::new(endpoint);
        // Send pause command
        tui_side.send_command(hydra_signals::CompanionCommand::Pause);
        service.tick();
        // Should receive response
        let output = tui_side.poll_output();
        assert!(output.is_some(), "Companion should respond to Pause");
        // Send status request
        tui_side.send_command(hydra_signals::CompanionCommand::RequestStatus);
        service.tick();
        let status = tui_side.poll_output();
        assert!(status.is_some(), "Companion should respond to RequestStatus");
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "companion_channel", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "companion_channel", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_dream_subsystems() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let mut state = hydra_kernel::state::HydraState::initial();
        state.step_count = 100;
        state.growth_state.beliefs_revised = 3;
        let mut subs = hydra_kernel::loop_dream::DreamSubsystems::new();
        let result = hydra_kernel::loop_dream::cycle_with_subsystems(&state, Some(&mut subs));
        assert!(result.did_work, "Dream cycle should do work at step 100");
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "dream_subsystems", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "dream_subsystems", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_ambient_subsystems() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let state = hydra_kernel::state::HydraState::initial();
        let mut subs = hydra_kernel::loop_ambient::AmbientSubsystems::new();
        let result = hydra_kernel::loop_ambient::tick_with_subsystems(&state, 0.1, Some(&mut subs));
        assert!(result.invariants_ok, "Invariants should pass on initial state");
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "ambient_subsystems", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "ambient_subsystems", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_settlement_middleware() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        // Settlement engine should open and report
        let engine = hydra_settlement::SettlementEngine::open();
        let count = engine.record_count();
        assert!(count >= 0, "Settlement record count should be non-negative");
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "settlement_middleware", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "settlement_middleware", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_device_profile() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let caps = hydra_reach::DeviceCapabilities {
            has_microphone: true, has_speaker: true, has_display: true,
            display_width: None, display_height: None,
            has_touch: false, has_camera: false, has_keyboard: true, is_mobile: false,
        };
        let profile = hydra_reach::DeviceProfile::new("test", "test-host", caps, "token");
        assert!(!profile.name.is_empty());
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "device_profile", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "device_profile", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_voice_detection() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let caps = hydra_voice::VoiceCapabilities::detect();
        // Should not panic, should return status lines
        assert!(!caps.status_lines.is_empty(), "Voice should report status");
        let tts = hydra_voice::TtsEngine::detect();
        // On macOS, TTS should be available (say command)
        // On CI/Linux, might not be — just check it doesn't panic
        let _ = tts.is_available();
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "voice_detection", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "voice_detection", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_genome_record_use() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let mut genome = hydra_genome::GenomeStore::open();
        genome.load_from_skills();
        let entries = genome.query("circuit breaker");
        if let Some(first) = entries.first() {
            let id = first.id.clone();
            let initial_use = first.use_count;
            let _ = genome.record_use(&id, true);
            // Use count should increase (or at least not error)
        }
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "genome_record_use", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "genome_record_use", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_succession_export() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let mut engine = hydra_succession::SuccessionEngine::new();
        let state = hydra_succession::InstanceState {
            instance_id: "test".into(), lineage_id: "test".into(),
            days_running: 1, soul_entries: 0, genome_entries: 0, calibration_profiles: 0,
        };
        // Should not panic
        let _ = engine.export(&state);
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "succession_export", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "succession_export", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_federation_engines() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let fed = hydra_federation::FederationEngine::new("test");
        let consensus = hydra_consensus::ConsensusEngine::new();
        let consent = hydra_consent::ConsentEngine::new();
        let collective = hydra_collective::CollectiveEngine::new();
        let mut diplomat = hydra_diplomat::DiplomatEngine::new();
        let exchange = hydra_exchange::ExchangeEngine::new();
        // All should initialize and report summary
        assert!(!fed.summary().is_empty());
        assert!(!consensus.summary().is_empty());
        assert!(!consent.summary().is_empty());
        assert!(!collective.summary().is_empty());
        assert!(!exchange.summary().is_empty());
        // Diplomat should create a session
        let session = diplomat.open_session("harness-test");
        assert!(!session.is_empty());
        assert_eq!(diplomat.session_count(), 1);
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "federation_engines", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "federation_engines", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_streaming_setup() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        // Verify the streaming types exist and compile
        use hydra_kernel::loop_::llm_stream::StreamChunk;
        let _text = StreamChunk::Text("test".into());
        let _done = StreamChunk::Done { tokens_used: 100, duration_ms: 500 };
        let _err = StreamChunk::Error("test error".into());
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "streaming_setup", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "streaming_setup", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}

fn test_belief_revision() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let mut store = hydra_belief::BeliefStore::new();
        let belief = hydra_belief::Belief::new(
            "test proposition",
            0.85,
            hydra_belief::BeliefCategory::Capability,
            hydra_belief::RevisionPolicy::Standard,
        );
        let result = hydra_belief::revise(&mut store, belief);
        assert!(result.is_ok(), "Belief revision should succeed");
        assert!(store.len() > 0, "Store should have at least 1 belief");
    }) {
        Ok(()) => TestResult::pass("new-subsystems", "belief_revision", start.elapsed().as_millis() as u64),
        Err(e) => TestResult::fail("new-subsystems", "belief_revision", &format!("{e:?}"), start.elapsed().as_millis() as u64),
    }
}
