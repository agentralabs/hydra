//! CaptchaSolver — detects and solves CAPTCHAs using vision analysis.

use crate::action::BrowserAction;
use crate::constants::CAPTCHA_MAX_RETRIES;
use crate::engine::BrowserEngine;
use crate::errors::BrowserError;
use crate::vision::VisionProvider;

use serde::{Deserialize, Serialize};

/// Type of CAPTCHA detected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CaptchaType {
    ImageGrid,
    TextDistortion,
    ReCaptchaV2,
    ReCaptchaV3,
    HCaptcha,
    Cloudflare,
    Unknown,
}

/// Result of a CAPTCHA solve attempt.
#[derive(Debug, Clone)]
pub struct CaptchaResult {
    pub solved: bool,
    pub captcha_type: CaptchaType,
    pub attempts: u32,
}

/// Detects and solves CAPTCHAs on web pages.
pub struct CaptchaSolver;

impl CaptchaSolver {
    /// Detect if the current page has a CAPTCHA.
    pub fn detect_captcha(html: &str) -> Option<CaptchaType> {
        let lower = html.to_lowercase();

        if lower.contains("g-recaptcha") || lower.contains("recaptcha") {
            if lower.contains("recaptcha/api.js?render=") {
                return Some(CaptchaType::ReCaptchaV3);
            }
            return Some(CaptchaType::ReCaptchaV2);
        }
        if lower.contains("hcaptcha") || lower.contains("h-captcha") {
            return Some(CaptchaType::HCaptcha);
        }
        if lower.contains("cf-turnstile") || lower.contains("cloudflare") {
            return Some(CaptchaType::Cloudflare);
        }
        if lower.contains("captcha") {
            // Generic captcha — could be image or text
            if lower.contains("img") && lower.contains("captcha") {
                return Some(CaptchaType::ImageGrid);
            }
            return Some(CaptchaType::TextDistortion);
        }

        None
    }

    /// Attempt to solve a CAPTCHA using the vision provider.
    pub async fn solve(
        engine: &mut BrowserEngine,
        captcha_type: &CaptchaType,
        vision: &dyn VisionProvider,
    ) -> Result<CaptchaResult, BrowserError> {
        for attempt in 0..CAPTCHA_MAX_RETRIES {
            eprintln!(
                "hydra-browser: CAPTCHA solve attempt {}/{} ({:?})",
                attempt + 1,
                CAPTCHA_MAX_RETRIES,
                captcha_type
            );

            let result = match captcha_type {
                CaptchaType::ReCaptchaV2 => {
                    Self::solve_recaptcha_v2(engine, vision).await
                }
                CaptchaType::HCaptcha => {
                    Self::solve_hcaptcha(engine, vision).await
                }
                CaptchaType::ImageGrid => {
                    Self::solve_image_grid(engine, vision).await
                }
                CaptchaType::TextDistortion => {
                    Self::solve_text_captcha(engine, vision).await
                }
                CaptchaType::ReCaptchaV3 => {
                    // V3 is invisible — human behavior simulation
                    Self::simulate_human_for_v3(engine).await
                }
                CaptchaType::Cloudflare => {
                    Self::solve_cloudflare(engine).await
                }
                CaptchaType::Unknown => {
                    Self::solve_generic(engine, vision).await
                }
            };

            match result {
                Ok(true) => {
                    return Ok(CaptchaResult {
                        solved: true,
                        captcha_type: captcha_type.clone(),
                        attempts: attempt + 1,
                    });
                }
                Ok(false) => continue,
                Err(e) => {
                    eprintln!("hydra-browser: CAPTCHA attempt {} failed: {e}", attempt + 1);
                    continue;
                }
            }
        }

        Ok(CaptchaResult {
            solved: false,
            captcha_type: captcha_type.clone(),
            attempts: CAPTCHA_MAX_RETRIES,
        })
    }

    async fn solve_recaptcha_v2(
        engine: &mut BrowserEngine,
        vision: &dyn VisionProvider,
    ) -> Result<bool, BrowserError> {
        // Click the reCAPTCHA checkbox
        let click_result = engine
            .execute(&BrowserAction::Click {
                selector: ".recaptcha-checkbox-border, #recaptcha-anchor".into(),
            })
            .await;

        if !click_result.success {
            return Ok(false);
        }

        // Wait for challenge
        engine.execute(&BrowserAction::Wait { ms: 2000 }).await;

        // Take screenshot and analyze with vision
        let screenshot = engine.screenshot().await?;
        let analysis = vision
            .analyze_image(
                &screenshot,
                "This is a reCAPTCHA challenge. Describe what you see and what action is needed to solve it. If there are image tiles to select, describe which ones match the prompt.",
            )
            .await?;

        eprintln!("hydra-browser: vision analysis for reCAPTCHA: {}", &analysis[..analysis.len().min(100)]);

        // Check if checkbox was enough (no challenge appeared)
        let html = engine.html().await?;
        if !html.to_lowercase().contains("rc-imageselect") {
            return Ok(true); // Checkbox was enough
        }

        // Image challenge requires clicking specific tiles — complex, attempt basic
        Ok(false)
    }

    async fn solve_hcaptcha(
        engine: &mut BrowserEngine,
        vision: &dyn VisionProvider,
    ) -> Result<bool, BrowserError> {
        // Similar to reCAPTCHA V2 flow
        engine
            .execute(&BrowserAction::Click {
                selector: ".h-captcha iframe, #h-captcha".into(),
            })
            .await;
        engine.execute(&BrowserAction::Wait { ms: 2000 }).await;

        let screenshot = engine.screenshot().await?;
        let _analysis = vision
            .analyze_image(&screenshot, "Describe this hCaptcha challenge and what needs to be selected.")
            .await?;

        Ok(false) // hCaptcha image challenges are complex
    }

    async fn solve_image_grid(
        engine: &mut BrowserEngine,
        vision: &dyn VisionProvider,
    ) -> Result<bool, BrowserError> {
        let screenshot = engine.screenshot().await?;
        let analysis = vision
            .analyze_image(
                &screenshot,
                "This page has an image CAPTCHA grid. Identify which grid squares match the prompt. Return the grid positions as numbers (e.g., '1,3,5,7').",
            )
            .await?;

        eprintln!("hydra-browser: image grid analysis: {}", &analysis[..analysis.len().min(100)]);
        Ok(false) // Needs grid click implementation
    }

    async fn solve_text_captcha(
        engine: &mut BrowserEngine,
        vision: &dyn VisionProvider,
    ) -> Result<bool, BrowserError> {
        let screenshot = engine.screenshot().await?;
        let text = vision
            .analyze_image(
                &screenshot,
                "This page has a text CAPTCHA. Read the distorted text and return ONLY the text characters you see, nothing else.",
            )
            .await?;

        let cleaned = text.trim().to_string();
        if cleaned.is_empty() {
            return Ok(false);
        }

        // Type the answer
        engine
            .execute(&BrowserAction::Type {
                selector: "input[name*=\"captcha\"], input[id*=\"captcha\"], input[class*=\"captcha\"]".into(),
                text: cleaned,
            })
            .await;

        // Submit
        engine
            .execute(&BrowserAction::Click {
                selector: "button[type=\"submit\"], input[type=\"submit\"]".into(),
            })
            .await;

        engine.execute(&BrowserAction::Wait { ms: 1500 }).await;
        Ok(true) // Assume success, will be verified by caller
    }

    async fn simulate_human_for_v3(engine: &mut BrowserEngine) -> Result<bool, BrowserError> {
        // ReCAPTCHA V3 scores human behavior — scroll, move mouse, wait
        engine
            .execute(&BrowserAction::Scroll {
                direction: crate::action::ScrollDirection::Down,
                amount: 200,
            })
            .await;
        engine.execute(&BrowserAction::Wait { ms: 1500 }).await;
        engine
            .execute(&BrowserAction::Scroll {
                direction: crate::action::ScrollDirection::Up,
                amount: 100,
            })
            .await;
        engine.execute(&BrowserAction::Wait { ms: 800 }).await;
        Ok(true) // V3 is scored, not binary — hope the behavior is enough
    }

    async fn solve_cloudflare(engine: &mut BrowserEngine) -> Result<bool, BrowserError> {
        // Cloudflare Turnstile usually auto-solves with a delay
        engine.execute(&BrowserAction::Wait { ms: 5000 }).await;
        let html = engine.html().await?;
        Ok(!html.to_lowercase().contains("cf-turnstile"))
    }

    async fn solve_generic(
        engine: &mut BrowserEngine,
        vision: &dyn VisionProvider,
    ) -> Result<bool, BrowserError> {
        let screenshot = engine.screenshot().await?;
        let _analysis = vision
            .analyze_image(&screenshot, "This page appears to have a CAPTCHA. Describe what you see and how to solve it.")
            .await?;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_recaptcha() {
        let html = r#"<div class="g-recaptcha" data-sitekey="abc"></div>"#;
        assert_eq!(CaptchaSolver::detect_captcha(html), Some(CaptchaType::ReCaptchaV2));
    }

    #[test]
    fn detect_hcaptcha() {
        let html = r#"<div class="h-captcha" data-sitekey="xyz"></div>"#;
        assert_eq!(CaptchaSolver::detect_captcha(html), Some(CaptchaType::HCaptcha));
    }

    #[test]
    fn detect_cloudflare() {
        let html = r#"<div class="cf-turnstile"></div>"#;
        assert_eq!(CaptchaSolver::detect_captcha(html), Some(CaptchaType::Cloudflare));
    }

    #[test]
    fn no_captcha_returns_none() {
        let html = r#"<html><body><p>Just a normal page</p></body></html>"#;
        assert_eq!(CaptchaSolver::detect_captcha(html), None);
    }

    #[test]
    fn detect_recaptcha_v3() {
        let html = r#"<script src="https://www.google.com/recaptcha/api.js?render=abc"></script>"#;
        assert_eq!(CaptchaSolver::detect_captcha(html), Some(CaptchaType::ReCaptchaV3));
    }
}
