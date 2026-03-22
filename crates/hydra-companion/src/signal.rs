//! Signal types and classification for the companion system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants;

/// Classification of a signal by urgency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SignalClass {
    /// Requires immediate attention — interrupts now.
    Urgent,
    /// Worth noting — surfaces at next natural pause.
    Notable,
    /// Standard signal — batched for /digest.
    Routine,
    /// Below noise threshold — silently archived, never surfaces.
    Noise,
}

impl std::fmt::Display for SignalClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Urgent => write!(f, "urgent"),
            Self::Notable => write!(f, "notable"),
            Self::Routine => write!(f, "routine"),
            Self::Noise => write!(f, "noise"),
        }
    }
}

/// How a signal should be routed to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalRouting {
    /// Interrupt immediately — display now (Urgent signals).
    InterruptNow,
    /// Surface at next natural pause (Notable signals).
    NextPause,
    /// Batch for /digest review (Routine signals).
    BatchForDigest,
    /// Archive silently — never surface (Noise).
    Archive,
}

impl SignalClass {
    /// Determine the routing for this signal class.
    pub fn routing(&self) -> SignalRouting {
        match self {
            Self::Urgent => SignalRouting::InterruptNow,
            Self::Notable => SignalRouting::NextPause,
            Self::Routine => SignalRouting::BatchForDigest,
            Self::Noise => SignalRouting::Archive,
        }
    }

    /// Return the TUI symbol for this signal class.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Urgent => "▲",
            Self::Notable => "●",
            Self::Routine => "○",
            Self::Noise => "",
        }
    }
}

/// A signal item received by the companion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalItem {
    /// Unique ID for this signal.
    pub id: Uuid,
    /// Where the signal came from.
    pub source: String,
    /// The signal content.
    pub content: String,
    /// Signal classification.
    pub class: SignalClass,
    /// Relevance score (0.0 to 1.0).
    pub relevance: f64,
    /// When the signal was received.
    pub timestamp: DateTime<Utc>,
    /// Whether this signal has been surfaced to the user.
    pub surfaced: bool,
}

impl SignalItem {
    /// Create a new signal item with default relevance.
    pub fn new(source: String, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            content,
            class: SignalClass::Routine,
            relevance: constants::DEFAULT_SIGNAL_RELEVANCE,
            timestamp: Utc::now(),
            surfaced: false,
        }
    }

    /// Mark this signal as surfaced.
    pub fn mark_surfaced(&mut self) {
        self.surfaced = true;
    }
}

/// Classifies signals by source and content patterns.
///
/// NOTE: This uses keyword matching as a bootstrap classifier.
/// In production, this should be replaced with an LLM micro-call
/// or sister-based classification per CLAUDE.md rules.
/// Why isn't a sister doing this? — Companion is the sister for this.
#[derive(Debug, Clone, Default)]
pub struct SignalClassifier {
    /// Keywords that indicate urgency.
    urgent_keywords: Vec<String>,
    /// Keywords that indicate notable signals.
    notable_keywords: Vec<String>,
}

impl SignalClassifier {
    /// Create a new signal classifier with default keyword sets.
    pub fn new() -> Self {
        Self {
            urgent_keywords: vec![
                "error".to_string(),
                "fail".to_string(),
                "crash".to_string(),
                "critical".to_string(),
                "panic".to_string(),
            ],
            notable_keywords: vec![
                "warning".to_string(),
                "complete".to_string(),
                "ready".to_string(),
                "changed".to_string(),
                "update".to_string(),
            ],
        }
    }

    /// Classify a signal based on its content.
    pub fn classify(&self, signal: &mut SignalItem) {
        let lower = signal.content.to_lowercase();

        if self
            .urgent_keywords
            .iter()
            .any(|k| lower.contains(k.as_str()))
        {
            signal.class = SignalClass::Urgent;
            signal.relevance = 1.0;
        } else if self
            .notable_keywords
            .iter()
            .any(|k| lower.contains(k.as_str()))
        {
            signal.class = SignalClass::Notable;
            signal.relevance = 0.7;
        } else if signal.relevance < constants::NOISE_THRESHOLD {
            signal.class = SignalClass::Noise;
        } else {
            signal.class = SignalClass::Routine;
        }
    }

    /// Add an urgent keyword.
    pub fn add_urgent_keyword(&mut self, keyword: String) {
        self.urgent_keywords.push(keyword);
    }

    /// Add a notable keyword.
    pub fn add_notable_keyword(&mut self, keyword: String) {
        self.notable_keywords.push(keyword);
    }
}

/// Buffer for holding classified signals.
#[derive(Debug, Clone)]
pub struct SignalBuffer {
    /// The signals in the buffer.
    signals: Vec<SignalItem>,
}

impl SignalBuffer {
    /// Create a new empty signal buffer.
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
        }
    }

    /// Push a signal into the buffer, evicting oldest if at capacity.
    pub fn push(&mut self, signal: SignalItem) {
        if self.signals.len() >= constants::MAX_SIGNAL_BUFFER {
            self.signals.remove(0);
        }
        self.signals.push(signal);
    }

    /// Return all signals.
    pub fn signals(&self) -> &[SignalItem] {
        &self.signals
    }

    /// Return signals of a specific class.
    pub fn by_class(&self, class: SignalClass) -> Vec<&SignalItem> {
        self.signals.iter().filter(|s| s.class == class).collect()
    }

    /// Return unsurfaced signals that should interrupt now (Urgent).
    pub fn pending_urgent(&self) -> Vec<&SignalItem> {
        self.signals
            .iter()
            .filter(|s| s.class == SignalClass::Urgent && !s.surfaced)
            .collect()
    }

    /// Return unsurfaced Notable signals (for next-pause surfacing).
    pub fn pending_notable(&self) -> Vec<&SignalItem> {
        self.signals
            .iter()
            .filter(|s| s.class == SignalClass::Notable && !s.surfaced)
            .collect()
    }

    /// Return Routine signals for /digest review.
    pub fn digest_items(&self) -> Vec<&SignalItem> {
        self.signals
            .iter()
            .filter(|s| s.class == SignalClass::Routine && !s.surfaced)
            .collect()
    }

    /// Return all signals for /inbox (surfaced + batched + archived).
    pub fn inbox(&self) -> &[SignalItem] {
        &self.signals
    }

    /// Mark a signal as surfaced by ID.
    pub fn mark_surfaced(&mut self, signal_id: Uuid) {
        if let Some(s) = self.signals.iter_mut().find(|s| s.id == signal_id) {
            s.mark_surfaced();
        }
    }

    /// Mark all Routine signals as surfaced (after /digest review).
    pub fn mark_digest_surfaced(&mut self) {
        for s in &mut self.signals {
            if s.class == SignalClass::Routine && !s.surfaced {
                s.surfaced = true;
            }
        }
    }

    /// Return the number of signals in the buffer.
    pub fn len(&self) -> usize {
        self.signals.len()
    }

    /// Return whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.signals.is_empty()
    }

    /// Clear all signals.
    pub fn clear(&mut self) {
        self.signals.clear();
    }
}

impl Default for SignalBuffer {
    fn default() -> Self {
        Self::new()
    }
}
