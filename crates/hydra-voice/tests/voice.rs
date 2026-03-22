//! Integration tests for hydra-voice.

use hydra_voice::constants;
use hydra_voice::speculative::{PredictionCandidate, SpeculativeProcessor, SpeculativeResult};
use hydra_voice::stt::{CaptureMode, PulseSTT, SttEvent};
use hydra_voice::system::{VoiceEvent, VoiceSystem};
use hydra_voice::tts::PulseTTS;

// ---- PulseSTT tests ----

#[test]
fn stt_silence_detection() {
    let mut stt = PulseSTT::new(CaptureMode::AlwaysListening);
    stt.start_capture();

    let silence = vec![0.0f32; 100];
    let mut found_silence = false;

    for _ in 0..10 {
        let events = stt.process_chunk(&silence);
        if events
            .iter()
            .any(|e| matches!(e, SttEvent::SilenceDetected))
        {
            found_silence = true;
            break;
        }
    }

    assert!(found_silence, "Should detect silence after enough chunks");
}

#[test]
fn stt_speech_duration_accumulation() {
    let mut stt = PulseSTT::new(CaptureMode::PushToTalk);
    stt.start_capture();

    let speech: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
    stt.process_chunk(&speech);
    stt.process_chunk(&speech);

    assert_eq!(
        stt.speech_duration_ms(),
        constants::CHUNK_SIZE_MS * 2,
        "Should accumulate duration for speech chunks"
    );
}

#[test]
fn stt_barge_in_detection() {
    let mut stt = PulseSTT::new(CaptureMode::AlwaysListening);
    stt.start_capture();
    stt.set_tts_playing(true);

    let loud: Vec<f32> = vec![0.5; 100];
    stt.process_chunk(&loud);

    assert!(stt.barge_in_detected(), "Should detect barge-in during TTS");

    stt.clear_barge_in();
    assert!(!stt.barge_in_detected());
}

#[test]
fn stt_not_capturing_returns_empty() {
    let mut stt = PulseSTT::new(CaptureMode::PushToTalk);
    // Not started.
    let events = stt.process_chunk(&[0.5; 100]);
    assert!(events.is_empty());
}

#[test]
fn stt_feed_partial_and_final() {
    let mut stt = PulseSTT::new(CaptureMode::AlwaysListening);
    stt.start_capture();

    let events = stt.feed_partial("hel".to_string());
    assert!(matches!(
        events.first(),
        Some(SttEvent::PartialTranscript { .. })
    ));
    assert_eq!(stt.partial_text(), "hel");

    let events = stt.feed_final("hello world".to_string());
    assert!(matches!(
        events.first(),
        Some(SttEvent::FinalTranscript { .. })
    ));
    assert!(stt.partial_text().is_empty());
}

// ---- SpeculativeProcessor tests ----

#[test]
fn speculative_partial_matching() {
    let mut proc = SpeculativeProcessor::new();
    proc.update_predictions(vec![
        PredictionCandidate {
            intent: "search files".to_string(),
            confidence: 0.9,
        },
        PredictionCandidate {
            intent: "open project".to_string(),
            confidence: 0.8,
        },
    ]);

    // Too short.
    let result = proc.check_partial("se");
    assert_eq!(result, SpeculativeResult::Pending);

    // Should match "search files".
    let result = proc.check_partial("search");
    assert!(
        matches!(result, SpeculativeResult::Match { ref intent, .. } if intent == "search files")
    );
}

#[test]
fn speculative_confirmation() {
    let mut proc = SpeculativeProcessor::new();
    proc.update_predictions(vec![PredictionCandidate {
        intent: "search files".to_string(),
        confidence: 0.9,
    }]);

    let _ = proc.check_partial("search");
    let result = proc.validate_final("search files please");
    assert!(matches!(result, SpeculativeResult::Confirmed { .. }));
}

#[test]
fn speculative_rejection() {
    let mut proc = SpeculativeProcessor::new();
    proc.update_predictions(vec![PredictionCandidate {
        intent: "search files".to_string(),
        confidence: 0.9,
    }]);

    let _ = proc.check_partial("search");
    let result = proc.validate_final("open the door");
    assert_eq!(result, SpeculativeResult::Rejected);
}

#[test]
fn speculative_no_candidates() {
    let mut proc = SpeculativeProcessor::new();
    let result = proc.check_partial("hello world");
    assert_eq!(result, SpeculativeResult::NoMatch);
}

// ---- PulseTTS tests ----

#[test]
fn tts_feed_and_speak() {
    let mut tts = PulseTTS::new();
    assert!(tts.is_idle());

    tts.feed_sentence("Hello".to_string()).expect("feed");
    assert!(tts.is_speaking());
    assert_eq!(tts.current_sentence(), Some("Hello"));

    tts.feed_sentence("World".to_string()).expect("feed");
    assert_eq!(tts.queue_depth(), 1);

    tts.finish_current();
    assert!(tts.is_speaking());
    assert_eq!(tts.current_sentence(), Some("World"));

    tts.finish_current();
    assert!(tts.is_idle());
}

#[test]
fn tts_interrupt_clears_queue() {
    let mut tts = PulseTTS::new();
    tts.feed_sentence("One".to_string()).expect("feed");
    tts.feed_sentence("Two".to_string()).expect("feed");
    tts.feed_sentence("Three".to_string()).expect("feed");

    tts.interrupt();
    assert!(tts.is_idle());
    assert_eq!(tts.queue_depth(), 0);
}

#[test]
fn tts_queue_limit() {
    let mut tts = PulseTTS::new();
    // First call advances to current_sentence, so queue holds MAX - 1.
    // We need MAX + 1 total calls to overflow.
    for i in 0..=constants::MAX_TTS_QUEUE {
        tts.feed_sentence(format!("Sentence {i}"))
            .expect("should not overflow");
    }
    let result = tts.feed_sentence("overflow".to_string());
    assert!(result.is_err());
}

// ---- VoiceSystem tests ----

#[test]
fn voice_system_barge_in_pipeline() {
    let mut system = VoiceSystem::new(CaptureMode::AlwaysListening);
    system.initialize();
    system.start_capture().expect("start");

    system
        .speak_sentence("Speaking now".to_string())
        .expect("speak");
    assert!(system.is_speaking());

    // Loud audio should trigger barge-in.
    let loud: Vec<f32> = vec![0.5; 100];
    let events = system.process_audio(&loud);
    assert!(events.iter().any(|e| matches!(e, VoiceEvent::BargeIn)));
    assert!(!system.is_speaking());
}

#[test]
fn voice_system_speculative_from_partial() {
    let mut system = VoiceSystem::new(CaptureMode::PushToTalk);
    system.initialize();

    system.update_predictions(vec![PredictionCandidate {
        intent: "run tests".to_string(),
        confidence: 0.85,
    }]);

    let events = system.feed_partial("run tests".to_string());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, VoiceEvent::SpeculativeMatch { .. }))
    );
}

#[test]
fn voice_system_not_initialized() {
    let mut system = VoiceSystem::new(CaptureMode::PushToTalk);
    let result = system.start_capture();
    assert!(result.is_err());
}

#[test]
fn voice_system_feed_final() {
    let mut system = VoiceSystem::new(CaptureMode::AlwaysListening);
    system.initialize();

    let events = system.feed_final("hello world".to_string());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, VoiceEvent::TranscriptReady { .. }))
    );
}
