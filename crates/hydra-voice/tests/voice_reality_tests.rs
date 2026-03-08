//! Integration tests for the hydra-voice reality system.
//! Tests graceful fallback behavior when models/devices are unavailable.

use hydra_voice::{
    MicrophoneInput, MockSttEngine, MockTtsEngine, PiperStub, SttBackend, SttEngine, SttError,
    TtsBackend, TtsEngine, TtsError, VoiceConfig, VoiceSession, VoiceState, VoiceSubsystem,
    WakeWordDetector, WakeWordStub, WhisperStub, WakeWordBackend,
};

// ═══════════════════════════════════════════════════════════
// STT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_whisper_load_model_graceful() {
    // WhisperStub should gracefully report that no model is available
    let stub = WhisperStub::new();
    assert!(!stub.is_ready());
    assert_eq!(stub.backend_name(), "whisper-stub");

    // Transcription should fail gracefully
    let audio = vec![0.0f32; 16000];
    let result = stub.transcribe(&audio);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SttError::ModelNotLoaded));
}

#[test]
fn test_whisper_transcribe_fallback() {
    // Real SttEngine without a model should return fallback
    let engine = SttEngine::new();
    assert!(!engine.is_loaded());

    let audio = vec![0.0f32; 16000];
    let fallback = engine.transcribe_safe(&audio);
    assert!(
        fallback.contains("didn't catch") || fallback.contains("say it again"),
        "Fallback message should be user-friendly, got: {fallback}"
    );
}

#[test]
fn test_piper_synthesize_fallback() {
    // PiperStub should gracefully report model not available
    let stub = PiperStub::new();
    assert!(!stub.is_ready());
    assert_eq!(stub.backend_name(), "piper-stub");

    let result = stub.synthesize("hello hydra");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TtsError::ModelNotLoaded));
}

#[test]
fn test_piper_output_graceful() {
    // TtsEngine without model should fall back to text
    let engine = TtsEngine::new();
    assert!(!engine.is_loaded());

    let result = engine.synthesize_or_text("hello");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "hello");
}

#[test]
fn test_microphone_detect() {
    let mic = MicrophoneInput::new();
    // Stub always returns false
    assert!(!mic.is_available());
    assert!(!mic.is_running());
    assert_eq!(mic.sample_rate(), 16000);
}

#[test]
fn test_microphone_no_device_graceful() {
    let mic = MicrophoneInput::new();

    // Start should fail gracefully
    let result = mic.start();
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.user_message().contains("microphone") || err.user_message().contains("No"));

    // Should not be running after failed start
    assert!(!mic.is_running());

    // Stop should be safe even when not running
    mic.stop();
    assert!(!mic.is_running());
}

#[test]
fn test_wake_word_detection_stub() {
    let stub = WakeWordStub::new();
    assert!(!stub.is_ready());
    assert_eq!(stub.backend_name(), "wake-word-stub");

    // Should never detect a wake word
    let audio = vec![0.0f32; 16000];
    let result = stub.process_audio(&audio, 0.5);
    assert!(result.is_none());

    // The existing WakeWordDetector should still work for simulation
    let detector = WakeWordDetector::new("hey hydra", 0.5);
    assert!(detector.is_configured());
    let event = detector.simulate_detection();
    assert!(event.confidence >= 0.5);
}

#[test]
fn test_voice_command_pipeline() {
    // Full pipeline: STT → command parse → action
    let mock_stt = MockSttEngine::with_text("hydra approve", 0.95);
    assert!(mock_stt.is_ready());

    let audio = vec![0.0f32; 16000];
    let transcript = mock_stt.transcribe(&audio).unwrap();
    assert_eq!(transcript.text, "hydra approve");
    assert!(transcript.confidence > 0.9);

    // Parse the transcribed text
    let command = hydra_voice::parse_command(&transcript.text);
    assert_eq!(command.action, hydra_voice::VoiceAction::Approve);
    assert!(hydra_voice::is_safe_to_execute(&command));
}

#[test]
fn test_stt_to_command_parser() {
    // Mock STT produces text, command parser interprets it
    let mock = MockSttEngine::with_text("stop everything", 0.88);
    let audio = vec![0.0f32; 8000];
    let transcript = mock.transcribe(&audio).unwrap();

    let command = hydra_voice::parse_command(&transcript.text);
    assert_eq!(command.action, hydra_voice::VoiceAction::Stop);
}

#[test]
fn test_tts_engine_mock() {
    // MockTtsEngine should produce audio bytes
    let mock = MockTtsEngine::new();
    assert!(mock.is_ready());
    assert_eq!(mock.backend_name(), "mock");

    let result = mock.synthesize("hello world");
    assert!(result.is_ok());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty());

    // Empty input should fail
    let empty_result = mock.synthesize("");
    assert!(matches!(empty_result.unwrap_err(), TtsError::EmptyInput));
}

#[test]
fn test_voice_toggle_enable_disable() {
    let config = VoiceConfig {
        enabled: true,
        ..VoiceConfig::default()
    };
    let mut subsystem = VoiceSubsystem::init(config).unwrap();

    // Start and verify active
    subsystem.start();
    assert!(subsystem.is_active());

    // Stop and verify disabled
    subsystem.stop();
    assert!(!subsystem.is_active());
    assert_eq!(subsystem.state(), VoiceState::Disabled);
}

#[test]
fn test_voice_state_transitions() {
    let session = VoiceSession::new();
    assert_eq!(session.state(), VoiceState::Idle);

    // Full lifecycle: Idle → WakeDetected → Listening → Processing → Speaking → Idle
    assert!(session.transition(VoiceState::WakeDetected));
    assert_eq!(session.state(), VoiceState::WakeDetected);

    assert!(session.transition(VoiceState::Listening));
    assert_eq!(session.state(), VoiceState::Listening);

    assert!(session.transition(VoiceState::Processing));
    assert_eq!(session.state(), VoiceState::Processing);

    assert!(session.transition(VoiceState::Speaking));
    assert_eq!(session.state(), VoiceState::Speaking);

    assert!(session.transition(VoiceState::Idle));
    assert_eq!(session.state(), VoiceState::Idle);

    // Invalid: Idle → Speaking
    assert!(!session.transition(VoiceState::Speaking));
    assert_eq!(session.state(), VoiceState::Idle);
}

#[test]
fn test_voice_config_defaults() {
    let config = VoiceConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.wake_word, "hey hydra");
    assert!(config.local_only);
    assert!(config.stt_model.is_none());
    assert!(config.tts_model.is_none());
    assert!(!config.models_available());
    assert!(!config.models_configured());
}

#[test]
fn test_stt_error_handling() {
    // Test various STT error types
    let errors = vec![
        SttError::ModelNotLoaded,
        SttError::ModelNotFound("/nonexistent/model.bin".into()),
        SttError::Timeout,
        SttError::AudioError("no microphone".into()),
    ];

    for err in &errors {
        // All errors should have meaningful messages
        let display = format!("{err}");
        assert!(!display.is_empty(), "Error display should not be empty");
        assert!(!err.user_message().is_empty());
        assert!(!err.suggested_action().is_empty());
    }

    // Test MockSttEngine failure mode
    let failing = MockSttEngine::failing(SttError::Timeout);
    assert!(!failing.is_ready());
    let result = failing.transcribe(&[0.0f32; 100]);
    assert!(matches!(result.unwrap_err(), SttError::Timeout));
}

#[test]
fn test_tts_error_handling() {
    // Test various TTS error types
    let errors = vec![
        TtsError::ModelNotLoaded,
        TtsError::ModelNotFound("/nonexistent/model.onnx".into()),
        TtsError::EmptyInput,
        TtsError::Timeout,
        TtsError::AudioError("no speaker".into()),
    ];

    for err in &errors {
        let display = format!("{err}");
        assert!(!display.is_empty(), "Error display should not be empty");
        assert!(!err.user_message().is_empty());
        assert!(!err.suggested_action().is_empty());
    }

    // Test MockTtsEngine failure mode
    let failing = MockTtsEngine::failing(TtsError::AudioError("speaker disconnected".into()));
    assert!(!failing.is_ready());
    let result = failing.synthesize("hello");
    assert!(result.is_err());
}
