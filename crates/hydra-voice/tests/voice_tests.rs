use std::io::Write as IoWrite;
use std::path::PathBuf;

use hydra_voice::config::VoiceConfig;
use hydra_voice::state::{VoiceSession, VoiceState};
use hydra_voice::stt::{SttEngine, SttError};
use hydra_voice::subsystem::VoiceSubsystem;
use hydra_voice::tts::{TtsEngine, TtsError};
use hydra_voice::wake_word::WakeWordDetector;

fn fake_model_path(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::File::create(&path)
        .unwrap()
        .write_all(b"fake-model")
        .unwrap();
    (dir, path)
}

// ═══════════════════════════════════════════════════════════
// WAKE WORD TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_wake_word_detection() {
    let detector = WakeWordDetector::new("hey hydra", 0.5);
    let event = detector.simulate_detection();
    assert!(event.confidence > 0.8);
    assert_eq!(detector.wake_phrase(), "hey hydra");
}

#[test]
fn test_wake_word_continuous() {
    let mut detector = WakeWordDetector::new("hey hydra", 0.5);
    assert!(!detector.is_running());
    detector.start();
    assert!(detector.is_running());
    detector.stop();
    assert!(!detector.is_running());
}

#[test]
fn test_wake_word_false_positive_rejection() {
    let detector = WakeWordDetector::new("hey hydra", 0.5);
    assert!(
        detector.process_with_confidence(0.3).is_none(),
        "Below threshold must be rejected"
    );
    assert!(
        detector.process_with_confidence(0.5).is_some(),
        "At threshold must trigger"
    );
    assert!(
        detector.process_with_confidence(0.95).is_some(),
        "Above threshold must trigger"
    );
}

// ═══════════════════════════════════════════════════════════
// STT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_stt_transcription() {
    let (_dir, model_path) = fake_model_path("whisper-base.bin");
    let mut stt = SttEngine::new();
    stt.load_model(&model_path).unwrap();
    assert!(stt.is_loaded());
    let result = stt.transcribe(&[0.0; 1000]);
    assert!(result.is_ok());
}

#[test]
fn test_stt_model_not_loaded() {
    let stt = SttEngine::new();
    assert!(!stt.is_loaded());
    let result = stt.transcribe(&[0.0; 100]);
    assert_eq!(result.unwrap_err(), SttError::ModelNotLoaded);
}

#[test]
fn test_stt_model_not_found() {
    let mut stt = SttEngine::new();
    let result = stt.load_model(&PathBuf::from("/nonexistent/whisper.bin"));
    assert!(matches!(result, Err(SttError::ModelNotFound(_))));
}

#[test]
fn test_stt_safe_fallback() {
    let stt = SttEngine::new(); // No model loaded
    let result = stt.transcribe_safe(&[0.0; 100]);
    assert!(result.contains("didn't catch"));
}

// ═══════════════════════════════════════════════════════════
// TTS TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_tts_synthesis() {
    let (_dir, model_path) = fake_model_path("piper.onnx");
    let mut tts = TtsEngine::new();
    tts.load_model(&model_path).unwrap();
    assert!(tts.is_loaded());
    let result = tts.synthesize("Hello world");
    assert!(result.is_ok());
    assert!(!result.unwrap().samples.is_empty());
}

#[test]
fn test_tts_model_not_loaded() {
    let tts = TtsEngine::new();
    let result = tts.synthesize("test");
    assert_eq!(result.unwrap_err(), TtsError::ModelNotLoaded);
}

#[test]
fn test_tts_model_not_found() {
    let mut tts = TtsEngine::new();
    let result = tts.load_model(std::path::Path::new("/nonexistent/piper.onnx"));
    assert!(matches!(result, Err(TtsError::ModelNotFound(_))));
}

#[test]
fn test_tts_empty_input() {
    let (_dir, model_path) = fake_model_path("piper.onnx");
    let mut tts = TtsEngine::new();
    tts.load_model(&model_path).unwrap();
    let result = tts.synthesize("");
    assert_eq!(result.unwrap_err(), TtsError::EmptyInput);
}

#[test]
fn test_tts_interruptible() {
    let tts = TtsEngine::new();
    assert!(tts.is_interruptible());
}

#[test]
fn test_tts_text_fallback() {
    let tts = TtsEngine::new(); // No model
    let result = tts.synthesize_or_text("Hello");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Hello");
}

// ═══════════════════════════════════════════════════════════
// STATE MACHINE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_voice_state_idle_to_listening() {
    let session = VoiceSession::new();
    assert_eq!(session.state(), VoiceState::Idle);
    assert!(session.transition(VoiceState::WakeDetected));
    assert_eq!(session.state(), VoiceState::WakeDetected);
    assert!(session.transition(VoiceState::Listening));
    assert_eq!(session.state(), VoiceState::Listening);
}

#[test]
fn test_voice_state_full_cycle() {
    let session = VoiceSession::new();
    assert!(session.transition(VoiceState::WakeDetected));
    assert!(session.transition(VoiceState::Listening));
    assert!(session.transition(VoiceState::Processing));
    assert!(session.transition(VoiceState::Speaking));
    assert!(session.transition(VoiceState::Idle));
}

#[test]
fn test_voice_barge_in() {
    let session = VoiceSession::new();
    session.transition(VoiceState::WakeDetected);
    session.transition(VoiceState::Listening);
    session.transition(VoiceState::Processing);
    session.transition(VoiceState::Speaking);
    assert!(session.transition(VoiceState::Listening));
    assert_eq!(session.state(), VoiceState::Listening);
}

#[test]
fn test_voice_silence_timeout() {
    let session = VoiceSession::new();
    session.transition(VoiceState::WakeDetected);
    session.transition(VoiceState::Listening);
    assert!(session.transition(VoiceState::Idle));
    assert_eq!(session.state(), VoiceState::Idle);
}

#[test]
fn test_voice_invalid_transition() {
    let session = VoiceSession::new();
    assert!(!session.transition(VoiceState::Speaking));
    assert_eq!(session.state(), VoiceState::Idle);
}

#[test]
fn test_voice_disabled_transition() {
    let session = VoiceSession::new();
    assert!(session.transition(VoiceState::Disabled));
    assert_eq!(session.state(), VoiceState::Disabled);
    assert!(session.transition(VoiceState::Idle));
}

#[test]
fn test_voice_session_duration() {
    let session = VoiceSession::new();
    assert!(session.session_duration().is_none());
    session.transition(VoiceState::WakeDetected);
    assert!(session.session_duration().is_some());
    session.transition(VoiceState::Idle);
    assert!(session.session_duration().is_none());
}

// ═══════════════════════════════════════════════════════════
// SUBSYSTEM TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_voice_subsystem_init_disabled() {
    let config = VoiceConfig::default();
    let subsystem = VoiceSubsystem::init(config).unwrap();
    assert!(!subsystem.is_active());
    assert_eq!(subsystem.state(), VoiceState::Idle);
}

#[test]
fn test_voice_subsystem_start_stop() {
    let mut config = VoiceConfig::default();
    config.enabled = true;
    let mut subsystem = VoiceSubsystem::init(config).unwrap();
    subsystem.start();
    assert!(subsystem.is_active());
    subsystem.stop();
    assert!(!subsystem.is_active());
    assert_eq!(subsystem.state(), VoiceState::Disabled);
}

#[test]
fn test_voice_missing_models() {
    let config = VoiceConfig {
        enabled: true,
        stt_model: None,
        tts_model: None,
        ..Default::default()
    };
    let subsystem = VoiceSubsystem::init(config).unwrap();
    assert!(!subsystem.stt().is_loaded());
    assert!(!subsystem.tts().is_loaded());
}

#[test]
fn test_voice_nonexistent_model_paths() {
    let config = VoiceConfig {
        enabled: true,
        stt_model: Some(PathBuf::from("/nonexistent/whisper.bin")),
        tts_model: Some(PathBuf::from("/nonexistent/piper.onnx")),
        ..Default::default()
    };
    // Should init gracefully — models fail to load but subsystem doesn't crash
    let subsystem = VoiceSubsystem::init(config).unwrap();
    assert!(!subsystem.stt().is_loaded());
    assert!(!subsystem.tts().is_loaded());
}

#[test]
fn test_voice_collision_detection() {
    let config = VoiceConfig::default();
    let subsystem = VoiceSubsystem::init(config).unwrap();
    let session = subsystem.session();
    session.transition(VoiceState::WakeDetected);
    session.transition(VoiceState::Listening);
    session.transition(VoiceState::Processing);
    session.transition(VoiceState::Speaking);
    assert!(subsystem.has_collision());
}

#[test]
fn test_voice_barge_in_handling() {
    let config = VoiceConfig::default();
    let subsystem = VoiceSubsystem::init(config).unwrap();
    let session = subsystem.session();
    session.transition(VoiceState::WakeDetected);
    session.transition(VoiceState::Listening);
    session.transition(VoiceState::Processing);
    session.transition(VoiceState::Speaking);
    assert!(subsystem.handle_barge_in());
    assert_eq!(subsystem.state(), VoiceState::Listening);
}

// ═══════════════════════════════════════════════════════════
// ERROR FORMAT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_stt_error_messages() {
    let errors = vec![
        SttError::ModelNotLoaded,
        SttError::ModelNotFound("/foo".into()),
        SttError::Timeout,
        SttError::AudioError("no mic".into()),
    ];
    for err in &errors {
        let msg = format!("{err}");
        assert!(!msg.is_empty());
        assert!(
            msg.contains('.'),
            "Error must follow sentence template: {msg}"
        );
    }
}

#[test]
fn test_tts_error_messages() {
    let errors = vec![
        TtsError::ModelNotLoaded,
        TtsError::ModelNotFound("/foo".into()),
        TtsError::EmptyInput,
        TtsError::Timeout,
        TtsError::AudioError("no speaker".into()),
    ];
    for err in &errors {
        let msg = format!("{err}");
        assert!(!msg.is_empty());
        assert!(msg.contains('.'));
    }
}

// ═══════════════════════════════════════════════════════════
// TIMEOUT CONSTANT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_timeout_constants_from_spec() {
    use hydra_voice::config::{STT_COMPLETE_TIMEOUT, TTS_COMPLETE_TIMEOUT, TTS_START_TIMEOUT};
    use std::time::Duration;

    assert_eq!(STT_COMPLETE_TIMEOUT, Duration::from_secs(10));
    assert_eq!(TTS_START_TIMEOUT, Duration::from_secs(2));
    assert_eq!(TTS_COMPLETE_TIMEOUT, Duration::from_secs(30));
}
