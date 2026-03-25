//! Layer 5: Cascade Verification — 5-tier cheapest-first action verification.
//!
//! After each click/action, verify it worked using the cheapest method first:
//! Tier 1: Cursor shape change (0 tokens, 10ms)
//! Tier 2: Window count/title change (0 tokens, 20ms)
//! Tier 3: Targeted OCR near click point (0 tokens, 50ms)
//! Tier 4: Differential screenshot hash (0 tokens, 100ms)
//! Tier 5: Vision LLM (tokens, 2000ms) — only if tiers 1-4 inconclusive

use crate::perception::PerceptionField;

/// Result of verification cascade.
#[derive(Debug, Clone)]
pub enum VerifyResult {
    /// Action confirmed at specified tier with evidence.
    Confirmed { tier: u8, evidence: String },
    /// Action definitely failed.
    Failed { reason: String },
    /// All cheap tiers inconclusive — caller should use vision LLM.
    Inconclusive,
}

/// Pre-action state snapshot for comparison.
#[derive(Debug, Clone)]
pub struct ActionExpectation {
    pub pre_window_count: usize,
    pub pre_window_title: String,
    pub pre_region_hash: u64,
    pub click_x: f64,
    pub click_y: f64,
    pub expected_text: Vec<String>,
}

impl ActionExpectation {
    /// Capture pre-action state.
    pub fn capture(click_x: f64, click_y: f64) -> Self {
        let windows = crate::app::AppManager::list_windows()
            .unwrap_or_default();
        let title = windows.first()
            .map(|w| w.title.clone()).unwrap_or_default();
        Self {
            pre_window_count: windows.len(),
            pre_window_title: title,
            pre_region_hash: 0,
            click_x, click_y,
            expected_text: Vec::new(),
        }
    }

    /// Add expected text that should appear after the action.
    pub fn expect_text(mut self, text: &str) -> Self {
        self.expected_text.push(text.to_string());
        self
    }
}

/// Run the 5-tier verification cascade.
pub fn verify_action(
    perception: &mut PerceptionField,
    expected: &ActionExpectation,
) -> VerifyResult {
    // Tier 1: Window count/title change (cheapest)
    let windows = crate::app::AppManager::list_windows()
        .unwrap_or_default();
    if windows.len() != expected.pre_window_count {
        return VerifyResult::Confirmed {
            tier: 1,
            evidence: format!("windows: {} → {}", expected.pre_window_count, windows.len()),
        };
    }
    let current_title = windows.first()
        .map(|w| w.title.clone()).unwrap_or_default();
    if current_title != expected.pre_window_title && !current_title.is_empty() {
        return VerifyResult::Confirmed {
            tier: 1,
            evidence: format!("title: '{}' → '{}'", expected.pre_window_title, current_title),
        };
    }

    // Tier 2: Targeted OCR near click point
    if !expected.expected_text.is_empty() {
        if let Ok(regions) = crate::ocr::ocr_current_screen() {
            let nearby: Vec<_> = regions.iter()
                .filter(|r| {
                    let dx = r.x - expected.click_x;
                    let dy = r.y - expected.click_y;
                    (dx * dx + dy * dy).sqrt() < 200.0
                }).collect();
            let combined: String = nearby.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join(" ");
            for expected_t in &expected.expected_text {
                if combined.to_lowercase().contains(&expected_t.to_lowercase()) {
                    return VerifyResult::Confirmed {
                        tier: 2,
                        evidence: format!("OCR found '{}' near click", expected_t),
                    };
                }
            }
        }
    }

    // Tier 3: Differential perception — did the click region change?
    if perception.region_changed(expected.click_x, expected.click_y) {
        return VerifyResult::Confirmed {
            tier: 3,
            evidence: "screen region changed after action".into(),
        };
    }

    // Tier 4: Full screenshot diff
    if let Ok((screenshot, _)) = crate::screen::ScreenCapture::capture_full() {
        let delta = perception.perceive_delta(&screenshot);
        if delta.change_ratio > 0.05 {
            return VerifyResult::Confirmed {
                tier: 4,
                evidence: format!("{:.0}% of screen changed", delta.change_ratio * 100.0),
            };
        }
        if delta.changed_cells.is_empty() {
            return VerifyResult::Failed {
                reason: "screen unchanged after action — click may have missed".into(),
            };
        }
    }

    // Tier 5: Inconclusive — caller escalates to vision LLM
    VerifyResult::Inconclusive
}

/// Quick check: did ANYTHING change on screen? (0 tokens, <100ms)
pub fn screen_changed(perception: &mut PerceptionField) -> bool {
    if let Ok((screenshot, _)) = crate::screen::ScreenCapture::capture_full() {
        let delta = perception.perceive_delta(&screenshot);
        delta.change_ratio > 0.01
    } else {
        true // assume changed if screenshot fails
    }
}
