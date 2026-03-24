//! HumanBehavior — anti-detection module.
//! Generates human-like delays, mouse curves, typing cadence, and jitter.

use crate::constants::{
    BEZIER_POINTS, HUMAN_MAX_DELAY_MS, HUMAN_MIN_DELAY_MS, JITTER_RADIUS_PX, TYPING_MAX_MS,
    TYPING_MIN_MS,
};
use rand::Rng;

/// Simulates human-like interaction patterns.
#[derive(Debug, Clone)]
pub struct HumanBehavior {
    min_delay_ms: u64,
    max_delay_ms: u64,
    typing_min_ms: u64,
    typing_max_ms: u64,
    jitter_radius: f64,
}

impl Default for HumanBehavior {
    fn default() -> Self {
        Self {
            min_delay_ms: HUMAN_MIN_DELAY_MS,
            max_delay_ms: HUMAN_MAX_DELAY_MS,
            typing_min_ms: TYPING_MIN_MS,
            typing_max_ms: TYPING_MAX_MS,
            jitter_radius: JITTER_RADIUS_PX,
        }
    }
}

impl HumanBehavior {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a random delay between actions.
    pub fn random_delay_ms(&self) -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(self.min_delay_ms..=self.max_delay_ms)
    }

    /// Generate per-character typing delays for a string.
    /// Returns a Vec of millisecond delays, one per character.
    pub fn typing_cadence(&self, text: &str) -> Vec<u64> {
        let mut rng = rand::thread_rng();
        text.chars()
            .map(|c| {
                let base = rng.gen_range(self.typing_min_ms..=self.typing_max_ms);
                // Spaces and punctuation get slightly longer pauses
                if c == ' ' || c == '.' || c == ',' || c == '!' || c == '?' {
                    base + rng.gen_range(20..=60)
                } else {
                    base
                }
            })
            .collect()
    }

    /// Generate a bezier curve path from (x0, y0) to (x1, y1).
    /// Returns a Vec of (x, y) points along the curve.
    pub fn mouse_curve(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(f64, f64)> {
        let mut rng = rand::thread_rng();

        // Two random control points for a cubic bezier
        let cx1 = x0 + (x1 - x0) * rng.gen_range(0.2..0.5)
            + rng.gen_range(-30.0..30.0);
        let cy1 = y0 + (y1 - y0) * rng.gen_range(0.1..0.4)
            + rng.gen_range(-30.0..30.0);
        let cx2 = x0 + (x1 - x0) * rng.gen_range(0.5..0.8)
            + rng.gen_range(-20.0..20.0);
        let cy2 = y0 + (y1 - y0) * rng.gen_range(0.6..0.9)
            + rng.gen_range(-20.0..20.0);

        let n = BEZIER_POINTS;
        (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                let u = 1.0 - t;
                let x = u * u * u * x0
                    + 3.0 * u * u * t * cx1
                    + 3.0 * u * t * t * cx2
                    + t * t * t * x1;
                let y = u * u * u * y0
                    + 3.0 * u * u * t * cy1
                    + 3.0 * u * t * t * cy2
                    + t * t * t * y1;
                (x, y)
            })
            .collect()
    }

    /// Add small random jitter to a click position.
    pub fn jitter(&self, x: f64, y: f64) -> (f64, f64) {
        let mut rng = rand::thread_rng();
        let dx = rng.gen_range(-self.jitter_radius..=self.jitter_radius);
        let dy = rng.gen_range(-self.jitter_radius..=self.jitter_radius);
        (x + dx, y + dy)
    }

    /// Generate a natural scroll amount (not a fixed jump).
    /// Returns pixel amount with slight randomization.
    pub fn natural_scroll(&self, requested: u32) -> u32 {
        let mut rng = rand::thread_rng();
        let variance = (requested as f64 * 0.15) as u32;
        let min = requested.saturating_sub(variance);
        let max = requested + variance;
        rng.gen_range(min..=max)
    }

    /// Sleep for a human-like delay (async).
    pub async fn delay(&self) {
        let ms = self.random_delay_ms();
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_delay_within_range() {
        let h = HumanBehavior::new();
        for _ in 0..100 {
            let d = h.random_delay_ms();
            assert!(d >= HUMAN_MIN_DELAY_MS);
            assert!(d <= HUMAN_MAX_DELAY_MS);
        }
    }

    #[test]
    fn typing_cadence_length_matches_text() {
        let h = HumanBehavior::new();
        let delays = h.typing_cadence("Hello, world!");
        assert_eq!(delays.len(), 13);
    }

    #[test]
    fn mouse_curve_starts_and_ends_correctly() {
        let h = HumanBehavior::new();
        let points = h.mouse_curve(0.0, 0.0, 100.0, 200.0);
        assert_eq!(points.len(), BEZIER_POINTS + 1);
        // First point is start
        assert!((points[0].0 - 0.0).abs() < 0.01);
        assert!((points[0].1 - 0.0).abs() < 0.01);
        // Last point is end
        let last = points.last().unwrap();
        assert!((last.0 - 100.0).abs() < 0.01);
        assert!((last.1 - 200.0).abs() < 0.01);
    }

    #[test]
    fn jitter_stays_within_radius() {
        let h = HumanBehavior::new();
        for _ in 0..100 {
            let (jx, jy) = h.jitter(50.0, 50.0);
            assert!((jx - 50.0).abs() <= JITTER_RADIUS_PX);
            assert!((jy - 50.0).abs() <= JITTER_RADIUS_PX);
        }
    }

    #[test]
    fn natural_scroll_near_requested() {
        let h = HumanBehavior::new();
        for _ in 0..100 {
            let s = h.natural_scroll(300);
            assert!(s >= 255); // 300 - 15%
            assert!(s <= 345); // 300 + 15%
        }
    }
}
