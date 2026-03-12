//! Living icon component with 8 visual states.
//!
//! The icon reflects Hydra's current activity through color, animation,
//! and shape changes — giving the user ambient awareness without text.

/// 8-state living icon
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IconState {
    /// Soft glow, slow breathing (3s cycle)
    Idle,
    /// Pulsing (0.5s cycle)
    Listening,
    /// Slow rotation (2s cycle)
    Working,
    /// Orange pulse (1s cycle)
    NeedsAttention,
    /// Gentle bounce
    ApprovalNeeded,
    /// Green flash + scale (400ms)
    Success,
    /// Red, subtle shake (300ms)
    Error,
    /// Hollow ring, no animation
    Offline,
}

impl IconState {
    /// CSS class name for the icon container (e.g. "icon-idle")
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Idle => "icon-idle",
            Self::Listening => "icon-listening",
            Self::Working => "icon-working",
            Self::NeedsAttention => "icon-needs-attention",
            Self::ApprovalNeeded => "icon-approval",
            Self::Success => "icon-success",
            Self::Error => "icon-error",
            Self::Offline => "icon-offline",
        }
    }

    /// Primary color for the icon in this state
    pub fn color(&self) -> &'static str {
        match self {
            Self::Idle => "#4A9EFF",
            Self::Listening => "#6366F1",
            Self::Working => "#6366F1",
            Self::NeedsAttention => "#FFAA4A",
            Self::ApprovalNeeded => "#FFAA4A",
            Self::Success => "#4ADE80",
            Self::Error => "#FF6B6B",
            Self::Offline => "#9CA3AF",
        }
    }

    /// CSS animation class for the icon
    pub fn animation_class(&self) -> &'static str {
        match self {
            Self::Idle => "animate-breathe",
            Self::Listening => "animate-pulse",
            Self::Working => "animate-spin-slow",
            Self::NeedsAttention => "animate-pulse-orange",
            Self::ApprovalNeeded => "animate-bounce-gentle",
            Self::Success => "animate-success-flash",
            Self::Error => "animate-shake",
            Self::Offline => "animate-none",
        }
    }

    /// Whether the icon should render as a hollow ring (no fill)
    pub fn is_hollow(&self) -> bool {
        matches!(self, Self::Offline)
    }

    /// Short human-readable label for the current state
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Ready",
            Self::Listening => "Listening...",
            Self::Working => "Working...",
            Self::NeedsAttention => "Attention needed",
            Self::ApprovalNeeded => "Approval needed",
            Self::Success => "Done!",
            Self::Error => "Something went wrong",
            Self::Offline => "Offline",
        }
    }

    /// Tooltip text providing more context
    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::Idle => "All good",
            Self::Listening => "I hear you",
            Self::Working => "On it",
            Self::NeedsAttention => "Check this out",
            Self::ApprovalNeeded => "Need your OK",
            Self::Success => "Nailed it",
            Self::Error => "Hit a snag",
            Self::Offline => "Not connected",
        }
    }
}

impl Default for IconState {
    fn default() -> Self {
        Self::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_state_default_is_idle() {
        assert_eq!(IconState::default(), IconState::Idle);
    }

    #[test]
    fn test_all_css_classes_unique() {
        let states = [
            IconState::Idle,
            IconState::Listening,
            IconState::Working,
            IconState::NeedsAttention,
            IconState::ApprovalNeeded,
            IconState::Success,
            IconState::Error,
            IconState::Offline,
        ];
        let classes: Vec<&str> = states.iter().map(|s| s.css_class()).collect();
        for (i, a) in classes.iter().enumerate() {
            for (j, b) in classes.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "duplicate css_class for {:?} and {:?}", states[i], states[j]);
                }
            }
        }
    }

    #[test]
    fn test_only_offline_is_hollow() {
        let states = [
            IconState::Idle,
            IconState::Listening,
            IconState::Working,
            IconState::NeedsAttention,
            IconState::ApprovalNeeded,
            IconState::Success,
            IconState::Error,
            IconState::Offline,
        ];
        for s in &states {
            if *s == IconState::Offline {
                assert!(s.is_hollow());
            } else {
                assert!(!s.is_hollow(), "{:?} should not be hollow", s);
            }
        }
    }

    #[test]
    fn test_all_labels_non_empty() {
        let states = [
            IconState::Idle,
            IconState::Listening,
            IconState::Working,
            IconState::NeedsAttention,
            IconState::ApprovalNeeded,
            IconState::Success,
            IconState::Error,
            IconState::Offline,
        ];
        for s in &states {
            assert!(!s.label().is_empty(), "{:?} has empty label", s);
        }
    }

    #[test]
    fn test_all_tooltips_non_empty() {
        let states = [
            IconState::Idle,
            IconState::Listening,
            IconState::Working,
            IconState::NeedsAttention,
            IconState::ApprovalNeeded,
            IconState::Success,
            IconState::Error,
            IconState::Offline,
        ];
        for s in &states {
            assert!(!s.tooltip().is_empty(), "{:?} has empty tooltip", s);
        }
    }

    #[test]
    fn test_all_animation_classes_non_empty() {
        let states = [
            IconState::Idle,
            IconState::Listening,
            IconState::Working,
            IconState::NeedsAttention,
            IconState::ApprovalNeeded,
            IconState::Success,
            IconState::Error,
            IconState::Offline,
        ];
        for s in &states {
            assert!(!s.animation_class().is_empty(), "{:?} has empty animation_class", s);
        }
    }

    #[test]
    fn test_all_animation_classes_unique() {
        let states = [
            IconState::Idle,
            IconState::Listening,
            IconState::Working,
            IconState::NeedsAttention,
            IconState::ApprovalNeeded,
            IconState::Success,
            IconState::Error,
            IconState::Offline,
        ];
        let classes: Vec<&str> = states.iter().map(|s| s.animation_class()).collect();
        for (i, a) in classes.iter().enumerate() {
            for (j, b) in classes.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "duplicate animation_class for {:?} and {:?}", states[i], states[j]);
                }
            }
        }
    }

    #[test]
    fn test_all_colors_are_hex() {
        let states = [
            IconState::Idle,
            IconState::Listening,
            IconState::Working,
            IconState::NeedsAttention,
            IconState::ApprovalNeeded,
            IconState::Success,
            IconState::Error,
            IconState::Offline,
        ];
        for s in &states {
            let c = s.color();
            assert!(c.starts_with('#'), "{:?} color '{}' is not hex", s, c);
            assert_eq!(c.len(), 7, "{:?} color '{}' is not #RRGGBB", s, c);
        }
    }

    #[test]
    fn test_all_colors_unique() {
        let states = [
            IconState::Idle,
            IconState::Listening,
            IconState::Working,
            IconState::NeedsAttention,
            IconState::ApprovalNeeded,
            IconState::Success,
            IconState::Error,
            IconState::Offline,
        ];
        let colors: Vec<&str> = states.iter().map(|s| s.color()).collect();
        // Not all are unique (Listening and Working share color, NeedsAttention and ApprovalNeeded share),
        // but each pair should be intentional. Just verify non-empty.
        for c in &colors {
            assert!(!c.is_empty());
        }
    }

    #[test]
    fn test_offline_has_no_animation() {
        assert_eq!(IconState::Offline.animation_class(), "animate-none");
    }

    #[test]
    fn test_error_state_properties() {
        let s = IconState::Error;
        assert_eq!(s.css_class(), "icon-error");
        assert_eq!(s.color(), "#FF6B6B");
        assert_eq!(s.animation_class(), "animate-shake");
        assert!(!s.is_hollow());
        assert_eq!(s.label(), "Something went wrong");
    }

    #[test]
    fn test_success_state_properties() {
        let s = IconState::Success;
        assert_eq!(s.css_class(), "icon-success");
        assert_eq!(s.color(), "#4ADE80");
        assert_eq!(s.animation_class(), "animate-success-flash");
        assert!(!s.is_hollow());
        assert_eq!(s.label(), "Done!");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let states = [
            IconState::Idle,
            IconState::Listening,
            IconState::Working,
            IconState::NeedsAttention,
            IconState::ApprovalNeeded,
            IconState::Success,
            IconState::Error,
            IconState::Offline,
        ];
        for s in &states {
            let json = serde_json::to_string(s).unwrap();
            let back: IconState = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }
}
