//! Signal classification, routing, and buffer tests.

use hydra_companion::signal::{SignalBuffer, SignalClass, SignalClassifier, SignalItem, SignalRouting};

#[test]
fn classify_urgent_signal() {
    let classifier = SignalClassifier::new();
    let mut signal = SignalItem::new("test".to_string(), "critical error occurred".to_string());
    classifier.classify(&mut signal);
    assert_eq!(signal.class, SignalClass::Urgent);
    assert_eq!(signal.relevance, 1.0);
}

#[test]
fn classify_notable_signal() {
    let classifier = SignalClassifier::new();
    let mut signal = SignalItem::new("test".to_string(), "build complete".to_string());
    classifier.classify(&mut signal);
    assert_eq!(signal.class, SignalClass::Notable);
}

#[test]
fn classify_routine_signal() {
    let classifier = SignalClassifier::new();
    let mut signal = SignalItem::new("test".to_string(), "processing data".to_string());
    classifier.classify(&mut signal);
    assert_eq!(signal.class, SignalClass::Routine);
}

#[test]
fn classify_noise_signal() {
    let classifier = SignalClassifier::new();
    let mut signal = SignalItem::new("test".to_string(), "heartbeat".to_string());
    signal.relevance = 0.1;
    classifier.classify(&mut signal);
    assert_eq!(signal.class, SignalClass::Noise);
}

#[test]
fn urgent_routes_interrupt() {
    assert_eq!(SignalClass::Urgent.routing(), SignalRouting::InterruptNow);
}

#[test]
fn notable_routes_next_pause() {
    assert_eq!(SignalClass::Notable.routing(), SignalRouting::NextPause);
}

#[test]
fn routine_routes_batch() {
    assert_eq!(SignalClass::Routine.routing(), SignalRouting::BatchForDigest);
}

#[test]
fn noise_routes_archive() {
    assert_eq!(SignalClass::Noise.routing(), SignalRouting::Archive);
}

#[test]
fn signal_class_symbols() {
    assert_eq!(SignalClass::Urgent.symbol(), "▲");
    assert_eq!(SignalClass::Notable.symbol(), "●");
    assert_eq!(SignalClass::Routine.symbol(), "○");
    assert_eq!(SignalClass::Noise.symbol(), "");
}

#[test]
fn signal_buffer_eviction() {
    let mut buffer = SignalBuffer::new();
    for i in 0..150 {
        buffer.push(SignalItem::new("src".to_string(), format!("signal {i}")));
    }
    assert_eq!(buffer.len(), 100);
}

#[test]
fn signal_buffer_by_class() {
    let classifier = SignalClassifier::new();
    let mut buffer = SignalBuffer::new();

    let mut s1 = SignalItem::new("src".to_string(), "critical failure".to_string());
    classifier.classify(&mut s1);
    buffer.push(s1);

    let mut s2 = SignalItem::new("src".to_string(), "normal event".to_string());
    classifier.classify(&mut s2);
    buffer.push(s2);

    let urgent = buffer.by_class(SignalClass::Urgent);
    assert_eq!(urgent.len(), 1);
}

#[test]
fn signal_buffer_pending_urgent() {
    let classifier = SignalClassifier::new();
    let mut buffer = SignalBuffer::new();

    let mut s = SignalItem::new("src".to_string(), "critical error".to_string());
    classifier.classify(&mut s);
    let id = s.id;
    buffer.push(s);

    assert_eq!(buffer.pending_urgent().len(), 1);
    buffer.mark_surfaced(id);
    assert_eq!(buffer.pending_urgent().len(), 0);
}

#[test]
fn signal_buffer_digest_items() {
    let mut buffer = SignalBuffer::new();
    buffer.push(SignalItem::new("src".to_string(), "routine item".to_string()));
    assert_eq!(buffer.digest_items().len(), 1);
    buffer.mark_digest_surfaced();
    assert_eq!(buffer.digest_items().len(), 0);
}
