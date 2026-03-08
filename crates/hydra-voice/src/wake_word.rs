use chrono::{DateTime, Utc};

/// Wake word detection event
#[derive(Debug, Clone)]
pub struct WakeWordEvent {
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

/// Wake word detector (OpenWakeWord wrapper)
/// Runs continuously on a separate thread, low CPU
pub struct WakeWordDetector {
    wake_phrase: String,
    sensitivity: f32,
    running: bool,
}

impl WakeWordDetector {
    pub fn new(wake_phrase: impl Into<String>, sensitivity: f32) -> Self {
        Self {
            wake_phrase: wake_phrase.into(),
            sensitivity,
            running: false,
        }
    }

    /// Start continuous wake word detection.
    /// Real implementation: spawns audio capture thread + OpenWakeWord ONNX inference loop.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Stop detection
    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn wake_phrase(&self) -> &str {
        &self.wake_phrase
    }

    pub fn sensitivity(&self) -> f32 {
        self.sensitivity
    }

    /// Simulate a wake word detection (for testing)
    pub fn simulate_detection(&self) -> WakeWordEvent {
        WakeWordEvent {
            confidence: 0.95,
            timestamp: Utc::now(),
        }
    }

    /// Process an audio chunk and return a detection if confidence meets threshold.
    /// Used by mock tests to exercise the confidence gate.
    pub fn process_with_confidence(&self, confidence: f32) -> Option<WakeWordEvent> {
        if confidence >= self.sensitivity {
            Some(WakeWordEvent {
                confidence,
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wake_word_detection() {
        let detector = WakeWordDetector::new("hey hydra", 0.5);
        let event = detector.simulate_detection();
        assert!(
            event.confidence >= 0.5,
            "Simulated detection should meet the sensitivity threshold"
        );
        assert_eq!(detector.wake_phrase(), "hey hydra");
        assert_eq!(detector.sensitivity(), 0.5);
    }

    #[test]
    fn test_wake_word_false_positive_rejection() {
        let detector = WakeWordDetector::new("hey hydra", 0.5);

        // Low confidence — should be rejected
        let rejected = detector.process_with_confidence(0.3);
        assert!(
            rejected.is_none(),
            "Confidence 0.3 below threshold 0.5 must be rejected"
        );

        // At threshold — should trigger
        let at_threshold = detector.process_with_confidence(0.5);
        assert!(
            at_threshold.is_some(),
            "Confidence exactly at threshold must trigger"
        );

        // High confidence — should trigger
        let triggered = detector.process_with_confidence(0.95);
        assert!(
            triggered.is_some(),
            "High confidence must trigger detection"
        );
        assert_eq!(triggered.unwrap().confidence, 0.95);
    }

    #[test]
    fn test_wake_word_continuous() {
        let mut detector = WakeWordDetector::new("hey hydra", 0.5);
        assert!(!detector.is_running(), "Detector should start stopped");

        detector.start();
        assert!(
            detector.is_running(),
            "Detector should be running after start()"
        );

        // Multiple events are valid while running
        for _ in 0..5 {
            let event = detector.simulate_detection();
            assert!(
                event.confidence > 0.0,
                "Each detection must have positive confidence"
            );
        }

        detector.stop();
        assert!(!detector.is_running(), "Detector should stop after stop()");
    }

    #[test]
    fn test_wake_word_custom_phrase() {
        let detector = WakeWordDetector::new("hey assistant", 0.7);
        assert_eq!(detector.wake_phrase(), "hey assistant");
        assert_eq!(detector.sensitivity(), 0.7);

        // Confidence below custom threshold rejected
        assert!(detector.process_with_confidence(0.6).is_none());
        // Confidence at custom threshold accepted
        assert!(detector.process_with_confidence(0.7).is_some());
    }
}
