//! Voice globe component — SVG orb with animation states.

use crate::state::hydra::GlobeState;

/// SVG parameters for the globe at each state
#[derive(Debug, Clone)]
pub struct GlobeRenderParams {
    pub state: GlobeState,
    /// Primary fill color
    pub fill: &'static str,
    /// Glow/shadow color
    pub glow: &'static str,
    /// CSS animation class
    pub animation: &'static str,
    /// Opacity of the outer ring
    pub ring_opacity: f64,
    /// Scale factor
    pub scale: f64,
    /// Inner icon type
    pub icon: GlobeIcon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobeIcon {
    Waveform,
    Microphone,
    Spinner,
    Check,
    X,
    Warning,
}

/// Get render parameters for a globe state
pub fn globe_params(state: GlobeState) -> GlobeRenderParams {
    match state {
        GlobeState::Idle => GlobeRenderParams {
            state,
            fill: "#6366f1",
            glow: "rgba(99, 102, 241, 0.3)",
            animation: "globe-breathe",
            ring_opacity: 0.0,
            scale: 1.0,
            icon: GlobeIcon::Waveform,
        },
        GlobeState::Listening => GlobeRenderParams {
            state,
            fill: "#22c55e",
            glow: "rgba(34, 197, 94, 0.4)",
            animation: "globe-pulse",
            ring_opacity: 0.6,
            scale: 1.05,
            icon: GlobeIcon::Microphone,
        },
        GlobeState::Processing => GlobeRenderParams {
            state,
            fill: "#3b82f6",
            glow: "rgba(59, 130, 246, 0.4)",
            animation: "globe-rotate",
            ring_opacity: 0.3,
            scale: 1.0,
            icon: GlobeIcon::Spinner,
        },
        GlobeState::Speaking => GlobeRenderParams {
            state,
            fill: "#8b5cf6",
            glow: "rgba(139, 92, 246, 0.4)",
            animation: "globe-ring-out",
            ring_opacity: 0.5,
            scale: 1.02,
            icon: GlobeIcon::Waveform,
        },
        GlobeState::Error => GlobeRenderParams {
            state,
            fill: "#ef4444",
            glow: "rgba(239, 68, 68, 0.4)",
            animation: "globe-shake",
            ring_opacity: 0.0,
            scale: 1.0,
            icon: GlobeIcon::X,
        },
        GlobeState::Approval => GlobeRenderParams {
            state,
            fill: "#f59e0b",
            glow: "rgba(245, 158, 11, 0.4)",
            animation: "globe-glow",
            ring_opacity: 0.7,
            scale: 1.03,
            icon: GlobeIcon::Warning,
        },
    }
}

/// Generate the SVG markup string for the globe
pub fn globe_svg(params: &GlobeRenderParams, size: u32) -> String {
    let r = size / 2 - 4;
    let cx = size / 2;
    let cy = size / 2;

    format!(
        r#"<svg width="{size}" height="{size}" viewBox="0 0 {size} {size}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <radialGradient id="globe-grad">
      <stop offset="0%" stop-color="{fill}" stop-opacity="0.9"/>
      <stop offset="100%" stop-color="{fill}" stop-opacity="0.6"/>
    </radialGradient>
    <filter id="globe-glow">
      <feGaussianBlur stdDeviation="4" result="blur"/>
      <feFlood flood-color="{glow}" result="color"/>
      <feComposite in="color" in2="blur" operator="in" result="shadow"/>
      <feMerge><feMergeNode in="shadow"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>
  <circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#globe-grad)" filter="url(#globe-glow)"/>
  <circle cx="{cx}" cy="{cy}" r="{ring_r}" fill="none" stroke="{fill}" stroke-width="1.5" opacity="{ring_opacity}"/>
</svg>"#,
        size = size,
        fill = params.fill,
        glow = params.glow,
        cx = cx,
        cy = cy,
        r = r,
        ring_r = r + 6,
        ring_opacity = params.ring_opacity,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_globe_params_all_states() {
        for &state in &[
            GlobeState::Idle,
            GlobeState::Listening,
            GlobeState::Processing,
            GlobeState::Speaking,
            GlobeState::Error,
            GlobeState::Approval,
        ] {
            let params = globe_params(state);
            assert_eq!(params.state, state);
            assert!(!params.fill.is_empty());
            assert!(!params.animation.is_empty());
        }
    }

    #[test]
    fn test_globe_state_transitions() {
        // Idle → Processing → Speaking → Idle (normal flow)
        let idle = globe_params(GlobeState::Idle);
        assert_eq!(idle.icon, GlobeIcon::Waveform);
        assert_eq!(idle.animation, "globe-breathe");

        let processing = globe_params(GlobeState::Processing);
        assert_eq!(processing.icon, GlobeIcon::Spinner);
        assert_eq!(processing.animation, "globe-rotate");

        let speaking = globe_params(GlobeState::Speaking);
        assert_eq!(speaking.icon, GlobeIcon::Waveform);

        // Error state
        let error = globe_params(GlobeState::Error);
        assert_eq!(error.icon, GlobeIcon::X);
        assert_eq!(error.fill, "#ef4444");

        // Approval state
        let approval = globe_params(GlobeState::Approval);
        assert_eq!(approval.icon, GlobeIcon::Warning);
        assert_eq!(approval.fill, "#f59e0b");
    }

    #[test]
    fn test_globe_svg_generation() {
        let params = globe_params(GlobeState::Idle);
        let svg = globe_svg(&params, 64);
        assert!(svg.contains("svg"));
        assert!(svg.contains("circle"));
        assert!(svg.contains(params.fill));
        assert!(svg.contains("radialGradient"));
    }

    #[test]
    fn test_globe_svg_sizes() {
        for size in [32, 48, 64, 128] {
            let params = globe_params(GlobeState::Processing);
            let svg = globe_svg(&params, size);
            assert!(svg.contains(&format!("width=\"{}\"", size)));
        }
    }
}
