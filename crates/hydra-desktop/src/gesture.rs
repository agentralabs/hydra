//! O19 Gesture Engine — motion-based gesture classification.
//! No ML dependency. Uses pixel-diff motion patterns across a sliding window.
//! EC-19.5: Gestures must be held for 500ms before triggering.

use std::collections::VecDeque;
use std::time::Instant;
use crate::webcam::FrameDigest;

/// Recognized gesture types (motion-pattern based).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    Wave,           // Rapid oscillating motion
    StillPresent,   // Low but nonzero motion (person sitting)
    LargeMotion,    // Whole-frame motion (entering/leaving)
    None,
}

impl Gesture {
    pub fn label(&self) -> &'static str {
        match self { Self::Wave => "wave", Self::StillPresent => "still",
            Self::LargeMotion => "large_motion", Self::None => "none" }
    }
}

/// Command triggered by a gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureCommand {
    Confirm,    // Wave = yes/proceed
    Attention,  // LargeMotion = user returned
    None,
}

impl GestureCommand {
    pub fn label(&self) -> &'static str {
        match self { Self::Confirm => "confirm", Self::Attention => "attention", Self::None => "none" }
    }
}

/// Map gesture to command.
pub fn map_command(gesture: &Gesture) -> GestureCommand {
    match gesture {
        Gesture::Wave => GestureCommand::Confirm,
        Gesture::LargeMotion => GestureCommand::Attention,
        _ => GestureCommand::None,
    }
}

/// Gesture classifier using sliding window of frame digests.
pub struct GestureClassifier {
    window: VecDeque<(f64, Instant)>,  // (motion_score, timestamp)
    hold_threshold_ms: u64,
    motion_threshold: f64,
    last_gesture: Option<(Gesture, Instant)>,
}

impl GestureClassifier {
    pub fn new() -> Self {
        Self {
            window: VecDeque::with_capacity(10),
            hold_threshold_ms: 500,  // EC-19.5
            motion_threshold: 0.02,
            last_gesture: None,
        }
    }

    /// Feed a motion score (from FrameDigest comparison). Returns classified gesture.
    pub fn feed(&mut self, motion: f64) -> Gesture {
        let now = Instant::now();
        self.window.push_back((motion, now));
        if self.window.len() > 10 { self.window.pop_front(); }
        if self.window.len() < 2 { return Gesture::None; }

        let avg_motion = self.window.iter().map(|(m, _)| m).sum::<f64>() / self.window.len() as f64;
        let gesture = classify_from_motion(avg_motion, self.motion_threshold);

        // EC-19.5: Check hold threshold
        if gesture != Gesture::None {
            if let Some((last_g, last_t)) = &self.last_gesture {
                if *last_g == gesture && last_t.elapsed().as_millis() >= self.hold_threshold_ms as u128 {
                    self.last_gesture = None;
                    return gesture;
                }
            } else {
                self.last_gesture = Some((gesture, now));
            }
        } else {
            self.last_gesture = None;
        }
        Gesture::None // Holding or no gesture
    }

    pub fn clear(&mut self) { self.window.clear(); self.last_gesture = None; }
}

impl Default for GestureClassifier {
    fn default() -> Self { Self::new() }
}

/// Classify gesture from average motion level.
fn classify_from_motion(avg_motion: f64, threshold: f64) -> Gesture {
    if avg_motion > 0.15 { Gesture::LargeMotion }
    else if avg_motion > 0.05 { Gesture::Wave }
    else if avg_motion > threshold { Gesture::StillPresent }
    else { Gesture::None }
}

/// Compute motion between two frame digests (convenience wrapper).
pub fn detect_motion(prev: &FrameDigest, current: &FrameDigest) -> f64 {
    prev.motion_score(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_window_none() {
        let mut c = GestureClassifier::new();
        assert_eq!(c.feed(0.0), Gesture::None);
    }

    #[test]
    fn command_mapping() {
        assert_eq!(map_command(&Gesture::Wave), GestureCommand::Confirm);
        assert_eq!(map_command(&Gesture::LargeMotion), GestureCommand::Attention);
        assert_eq!(map_command(&Gesture::None), GestureCommand::None);
    }

    #[test]
    fn motion_classify() {
        assert_eq!(classify_from_motion(0.2, 0.02), Gesture::LargeMotion);
        assert_eq!(classify_from_motion(0.07, 0.02), Gesture::Wave);
        assert_eq!(classify_from_motion(0.03, 0.02), Gesture::StillPresent);
        assert_eq!(classify_from_motion(0.01, 0.02), Gesture::None);
    }

    #[test]
    fn gesture_labels() {
        assert_eq!(Gesture::Wave.label(), "wave");
        assert_eq!(GestureCommand::Confirm.label(), "confirm");
    }
}
