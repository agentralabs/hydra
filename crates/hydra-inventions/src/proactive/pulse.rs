//! ProactivePulse — anticipate user needs before they're expressed.
//!
//! Monitors context signals and generates proactive suggestions
//! based on patterns, time of day, recent activity, and user habits.

use serde::{Deserialize, Serialize};

/// Strength of a proactive pulse signal
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PulseStrength {
    Whisper,
    Nudge,
    Suggestion,
    Urgent,
}

/// A proactive signal emitted by the pulse engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseSignal {
    pub id: String,
    pub message: String,
    pub strength: PulseStrength,
    pub confidence: f64,
    pub context: String,
    pub timestamp: String,
    pub acted_upon: bool,
}

impl PulseSignal {
    pub fn new(message: &str, strength: PulseStrength, confidence: f64, context: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message: message.into(),
            strength,
            confidence: confidence.clamp(0.0, 1.0),
            context: context.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            acted_upon: false,
        }
    }
}

/// Context signal input for the pulse engine
#[derive(Debug, Clone)]
pub struct ContextSignal {
    pub name: String,
    pub value: f64,
    pub category: String,
}

/// Proactive pulse engine
pub struct ProactivePulse {
    signals: parking_lot::RwLock<Vec<PulseSignal>>,
    context_history: parking_lot::RwLock<Vec<ContextSignal>>,
    sensitivity: f64,
    max_signals: usize,
}

impl ProactivePulse {
    pub fn new(sensitivity: f64, max_signals: usize) -> Self {
        Self {
            signals: parking_lot::RwLock::new(Vec::new()),
            context_history: parking_lot::RwLock::new(Vec::new()),
            sensitivity: sensitivity.clamp(0.0, 1.0),
            max_signals,
        }
    }

    /// Feed a context signal into the pulse engine
    pub fn feed_context(&self, signal: ContextSignal) {
        self.context_history.write().push(signal);
    }

    /// Analyze context and generate proactive suggestions
    pub fn pulse(&self) -> Vec<PulseSignal> {
        let context = self.context_history.read();
        let mut new_signals = Vec::new();

        // Analyze patterns in context signals
        if context.is_empty() {
            return new_signals;
        }

        // Check for repeated high-value signals (potential need)
        let mut category_scores: std::collections::HashMap<String, (f64, usize)> =
            std::collections::HashMap::new();

        for signal in context.iter() {
            let entry = category_scores
                .entry(signal.category.clone())
                .or_insert((0.0, 0));
            entry.0 += signal.value;
            entry.1 += 1;
        }

        for (category, (total, count)) in &category_scores {
            let avg = total / *count as f64;
            if avg > self.sensitivity {
                let strength = match avg {
                    v if v > 0.9 => PulseStrength::Urgent,
                    v if v > 0.7 => PulseStrength::Suggestion,
                    v if v > 0.5 => PulseStrength::Nudge,
                    _ => PulseStrength::Whisper,
                };

                let signal = PulseSignal::new(
                    &format!("Detected high activity in '{}' — you may need assistance", category),
                    strength,
                    avg,
                    category,
                );
                new_signals.push(signal);
            }
        }

        // Store signals
        let mut stored = self.signals.write();
        stored.extend(new_signals.clone());
        while stored.len() > self.max_signals {
            stored.remove(0);
        }

        new_signals
    }

    /// Mark a signal as acted upon (positive feedback)
    pub fn mark_acted(&self, signal_id: &str) -> bool {
        if let Some(signal) = self.signals.write().iter_mut().find(|s| s.id == signal_id) {
            signal.acted_upon = true;
            true
        } else {
            false
        }
    }

    /// Get the hit rate (signals acted upon / total signals)
    pub fn hit_rate(&self) -> f64 {
        let signals = self.signals.read();
        if signals.is_empty() {
            return 0.0;
        }
        let acted = signals.iter().filter(|s| s.acted_upon).count();
        acted as f64 / signals.len() as f64
    }

    /// Get all pending (un-acted) signals
    pub fn pending_signals(&self) -> Vec<PulseSignal> {
        self.signals
            .read()
            .iter()
            .filter(|s| !s.acted_upon)
            .cloned()
            .collect()
    }

    pub fn signal_count(&self) -> usize {
        self.signals.read().len()
    }
}

impl Default for ProactivePulse {
    fn default() -> Self {
        Self::new(0.5, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pulse_generation() {
        let pulse = ProactivePulse::new(0.3, 100);

        pulse.feed_context(ContextSignal {
            name: "file_access".into(),
            value: 0.8,
            category: "development".into(),
        });
        pulse.feed_context(ContextSignal {
            name: "error_count".into(),
            value: 0.9,
            category: "debugging".into(),
        });

        let signals = pulse.pulse();
        assert!(!signals.is_empty());
    }

    #[test]
    fn test_below_sensitivity_no_signal() {
        let pulse = ProactivePulse::new(0.9, 100);

        pulse.feed_context(ContextSignal {
            name: "idle".into(),
            value: 0.1,
            category: "activity".into(),
        });

        let signals = pulse.pulse();
        assert!(signals.is_empty());
    }

    #[test]
    fn test_signal_feedback() {
        let pulse = ProactivePulse::new(0.3, 100);
        pulse.feed_context(ContextSignal {
            name: "test".into(),
            value: 0.8,
            category: "test".into(),
        });

        let signals = pulse.pulse();
        assert!(!signals.is_empty());

        let signal_id = signals[0].id.clone();
        assert!(pulse.mark_acted(&signal_id));
        assert!(pulse.hit_rate() > 0.0);
    }

    #[test]
    fn test_strength_classification() {
        let pulse = ProactivePulse::new(0.1, 100);

        pulse.feed_context(ContextSignal {
            name: "critical".into(),
            value: 0.95,
            category: "urgent".into(),
        });

        let signals = pulse.pulse();
        assert!(!signals.is_empty());
        assert_eq!(signals[0].strength, PulseStrength::Urgent);
    }
}
