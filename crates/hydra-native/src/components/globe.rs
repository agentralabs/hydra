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
    /// Audio reactivity level (0.0 - 1.0)
    pub audio_level: f32,
    /// Number of particles (for Processing state)
    pub particle_count: u32,
}

/// Mode-responsive globe sizing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobeSize {
    /// Immersive mode: 400px, full detail
    Full,
    /// Companion mode: 200px, reduced particles
    Medium,
    /// Invisible mode: 64px, minimal
    Compact,
    /// Workspace topbar: 48px, ring-only
    TopBar,
}

impl GlobeSize {
    pub fn pixels(&self) -> u32 {
        match self {
            Self::Full => 400,
            Self::Medium => 200,
            Self::Compact => 64,
            Self::TopBar => 48,
        }
    }

    pub fn particle_scale(&self) -> f32 {
        match self {
            Self::Full => 1.0,
            Self::Medium => 0.5,
            Self::Compact => 0.1,
            Self::TopBar => 0.0,
        }
    }
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

/// Get render parameters for a globe state (using Hydra blue design system palette).
pub fn globe_params(state: GlobeState) -> GlobeRenderParams {
    match state {
        GlobeState::Idle => GlobeRenderParams {
            state,
            fill: "#6495ED",          // Hydra blue accent
            glow: "rgba(100, 149, 237, 0.25)",
            animation: "globe-breathe",
            ring_opacity: 0.0,
            scale: 1.0,
            icon: GlobeIcon::Waveform,
            audio_level: 0.0,
            particle_count: 0,
        },
        GlobeState::Listening => GlobeRenderParams {
            state,
            fill: "#7BA8F0",          // accent-hover (lighter blue)
            glow: "rgba(122, 168, 240, 0.4)",
            animation: "globe-pulse",
            ring_opacity: 0.6,
            scale: 1.05,
            icon: GlobeIcon::Microphone,
            audio_level: 0.0,
            particle_count: 0,
        },
        GlobeState::Processing => GlobeRenderParams {
            state,
            fill: "#7B8CDE",          // cool blue-lavender for processing
            glow: "rgba(123, 140, 222, 0.4)",
            animation: "globe-rotate",
            ring_opacity: 0.3,
            scale: 1.0,
            icon: GlobeIcon::Spinner,
            audio_level: 0.0,
            particle_count: 30,
        },
        GlobeState::Speaking => GlobeRenderParams {
            state,
            fill: "#6495ED",          // Hydra blue for speaking (brand voice)
            glow: "rgba(100, 149, 237, 0.4)",
            animation: "globe-ring-out",
            ring_opacity: 0.5,
            scale: 1.02,
            icon: GlobeIcon::Waveform,
            audio_level: 0.0,
            particle_count: 10,
        },
        GlobeState::Error => GlobeRenderParams {
            state,
            fill: "#EF5350",          // design system error red
            glow: "rgba(239, 83, 80, 0.4)",
            animation: "globe-shake",
            ring_opacity: 0.0,
            scale: 1.0,
            icon: GlobeIcon::X,
            audio_level: 0.0,
            particle_count: 0,
        },
        GlobeState::Approval => GlobeRenderParams {
            state,
            fill: "#F5A623",          // design system warning amber
            glow: "rgba(245, 166, 35, 0.4)",
            animation: "globe-glow",
            ring_opacity: 0.7,
            scale: 1.03,
            icon: GlobeIcon::Warning,
            audio_level: 0.0,
            particle_count: 0,
        },
    }
}

/// Derive globe state from kernel phase + gate status + voice activity.
pub fn derive_globe_state(
    phase: &str,
    has_pending_approval: bool,
    voice_active: bool,
    kill_switch_active: bool,
) -> GlobeState {
    if kill_switch_active {
        return GlobeState::Error;
    }
    if has_pending_approval {
        return GlobeState::Approval;
    }
    if voice_active {
        return GlobeState::Listening;
    }
    match phase {
        "Perceive" | "Think" | "Act" | "Decide" | "Learn" => GlobeState::Processing,
        "Done" => GlobeState::Idle,
        "Error" => GlobeState::Error,
        _ => GlobeState::Idle,
    }
}

/// Generate the SVG markup string for the globe with audio-reactive effects.
pub fn globe_svg(params: &GlobeRenderParams, size: u32) -> String {
    let r = size / 2 - 4;
    let cx = size / 2;
    let cy = size / 2;
    // Audio level affects glow intensity and ring displacement
    let glow_std = 4.0 + (params.audio_level * 8.0);
    let ring_displacement = (params.audio_level * 4.0) as u32;

    let mut svg = format!(
        r#"<svg width="{size}" height="{size}" viewBox="0 0 {size} {size}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <radialGradient id="globe-grad">
      <stop offset="0%" stop-color="{fill}" stop-opacity="0.9"/>
      <stop offset="100%" stop-color="{fill}" stop-opacity="0.6"/>
    </radialGradient>
    <filter id="globe-glow">
      <feGaussianBlur stdDeviation="{glow_std}" result="blur"/>
      <feFlood flood-color="{glow}" result="color"/>
      <feComposite in="color" in2="blur" operator="in" result="shadow"/>
      <feMerge><feMergeNode in="shadow"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>
  <circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#globe-grad)" filter="url(#globe-glow)" class="{animation}"/>
  <circle cx="{cx}" cy="{cy}" r="{ring_r}" fill="none" stroke="{fill}" stroke-width="1.5" opacity="{ring_opacity}"/>"#,
        size = size,
        fill = params.fill,
        glow = params.glow,
        glow_std = glow_std,
        cx = cx,
        cy = cy,
        r = r,
        ring_r = r + 6 + ring_displacement,
        ring_opacity = params.ring_opacity,
        animation = params.animation,
    );

    // Add particles for Processing/Speaking states
    if params.particle_count > 0 && size >= 64 {
        let orbit_r = r as f64 + 12.0;
        for i in 0..params.particle_count {
            let angle = (i as f64 / params.particle_count as f64) * std::f64::consts::TAU;
            let px = cx as f64 + orbit_r * angle.cos();
            let py = cy as f64 + orbit_r * angle.sin();
            svg.push_str(&format!(
                r#"
  <circle cx="{:.1}" cy="{:.1}" r="1.5" fill="{}" opacity="0.6" class="globe-particle"/>"#,
                px, py, params.fill,
            ));
        }
    }

    svg.push_str("\n</svg>");
    svg
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
    fn test_globe_hydra_blue_palette() {
        let idle = globe_params(GlobeState::Idle);
        assert_eq!(idle.fill, "#6495ED"); // Hydra blue accent
        assert_eq!(idle.icon, GlobeIcon::Waveform);
        assert_eq!(idle.animation, "globe-breathe");

        let listening = globe_params(GlobeState::Listening);
        assert_eq!(listening.fill, "#7BA8F0"); // accent-hover

        let processing = globe_params(GlobeState::Processing);
        assert_eq!(processing.fill, "#7B8CDE"); // blue-lavender
        assert_eq!(processing.icon, GlobeIcon::Spinner);
        assert!(processing.particle_count > 0);

        let error = globe_params(GlobeState::Error);
        assert_eq!(error.fill, "#EF5350"); // design system error

        let approval = globe_params(GlobeState::Approval);
        assert_eq!(approval.fill, "#F5A623"); // design system warning
    }

    #[test]
    fn test_globe_svg_generation() {
        let params = globe_params(GlobeState::Idle);
        let svg = globe_svg(&params, 64);
        assert!(svg.contains("svg"));
        assert!(svg.contains("circle"));
        assert!(svg.contains(params.fill));
        assert!(svg.contains("radialGradient"));
        assert!(svg.contains("globe-breathe"));
    }

    #[test]
    fn test_globe_svg_sizes() {
        for size in [32, 48, 64, 128] {
            let params = globe_params(GlobeState::Processing);
            let svg = globe_svg(&params, size);
            assert!(svg.contains(&format!("width=\"{}\"", size)));
        }
    }

    #[test]
    fn test_globe_svg_particles() {
        let params = globe_params(GlobeState::Processing);
        assert!(params.particle_count > 0);
        let svg = globe_svg(&params, 200);
        assert!(svg.contains("globe-particle"));
    }

    #[test]
    fn test_globe_svg_no_particles_when_small() {
        let params = globe_params(GlobeState::Processing);
        let svg = globe_svg(&params, 32); // too small for particles
        assert!(!svg.contains("globe-particle"));
    }

    #[test]
    fn test_globe_audio_reactive() {
        let mut params = globe_params(GlobeState::Listening);
        params.audio_level = 0.8;
        let svg = globe_svg(&params, 200);
        // Higher audio level = larger glow stdDeviation
        assert!(svg.contains("stdDeviation=\"10.4\"")); // 4.0 + 0.8*8.0
    }

    #[test]
    fn test_globe_size_enum() {
        assert_eq!(GlobeSize::Full.pixels(), 400);
        assert_eq!(GlobeSize::Medium.pixels(), 200);
        assert_eq!(GlobeSize::Compact.pixels(), 64);
        assert_eq!(GlobeSize::TopBar.pixels(), 48);

        assert_eq!(GlobeSize::TopBar.particle_scale(), 0.0);
        assert!(GlobeSize::Full.particle_scale() > GlobeSize::Medium.particle_scale());
    }

    #[test]
    fn test_derive_globe_state() {
        assert_eq!(derive_globe_state("Idle", false, false, false), GlobeState::Idle);
        assert_eq!(derive_globe_state("Think", false, false, false), GlobeState::Processing);
        assert_eq!(derive_globe_state("Act", false, false, false), GlobeState::Processing);
        assert_eq!(derive_globe_state("Error", false, false, false), GlobeState::Error);
        // Kill switch overrides everything
        assert_eq!(derive_globe_state("Think", false, false, true), GlobeState::Error);
        // Approval overrides normal processing
        assert_eq!(derive_globe_state("Decide", true, false, false), GlobeState::Approval);
        // Voice overrides idle
        assert_eq!(derive_globe_state("Idle", false, true, false), GlobeState::Listening);
    }
}
