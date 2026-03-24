//! Platform warmup routines — establishes human browsing footprint before tasks.
//! Called from ComputerUseAgent before the main vision-action loop.

use crate::engine::BrowserEngine;
use crate::errors::BrowserError;

/// Warmup configuration per platform.
pub struct WarmupConfig {
    pub scroll_count: u32,
    pub read_delay_ms: u64,
    pub check_session: bool,
}

/// Result of a warmup attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum WarmupStatus {
    /// Platform warmed up, ready for actions.
    Ready,
    /// Session cookie expired — need re-login first (EC-12.3).
    SessionExpired,
    /// CAPTCHA appeared during warmup (EC-12.4).
    CaptchaDetected(String),
    /// Platform is blocking this IP entirely (EC-12.5).
    IpBlocked,
    /// Domain doesn't need warmup — proceed directly.
    Skipped,
}

/// Returns warmup config for domains that need it.
/// Social/content platforms need warmup; utility sites don't.
pub fn config_for_domain(domain: &str) -> Option<WarmupConfig> {
    let lower = domain.to_lowercase();
    if lower.contains("linkedin") {
        Some(WarmupConfig { scroll_count: 3, read_delay_ms: 2500, check_session: true })
    } else if lower.contains("twitter") || lower.contains("x.com") {
        Some(WarmupConfig { scroll_count: 2, read_delay_ms: 2000, check_session: true })
    } else if lower.contains("instagram") || lower.contains("facebook") {
        Some(WarmupConfig { scroll_count: 3, read_delay_ms: 3000, check_session: true })
    } else {
        None
    }
}

/// Execute warmup for a domain — browse naturally before performing the real task.
pub async fn warmup(engine: &mut BrowserEngine, domain: &str) -> Result<WarmupStatus, BrowserError> {
    let config = match config_for_domain(domain) {
        Some(c) => c,
        None => return Ok(WarmupStatus::Skipped),
    };
    // EC-12.3: Check session validity before warmup
    if config.check_session && !engine.session_manager_mut().has_valid_session(domain) {
        eprintln!("hydra-warmup: session expired for {domain}");
        return Ok(WarmupStatus::SessionExpired);
    }
    // Natural browsing: scroll + read pauses
    for i in 0..config.scroll_count {
        let scroll_amount = 300 + (i * 100);
        let scroll_action = crate::action::BrowserAction::Scroll {
            direction: crate::action::ScrollDirection::Down,
            amount: scroll_amount,
        };
        engine.execute(&scroll_action).await;
        tokio::time::sleep(std::time::Duration::from_millis(config.read_delay_ms)).await;
    }
    // EC-12.4: Check for CAPTCHA after warmup browsing
    let html = engine.execute(&crate::action::BrowserAction::GetHtml).await;
    if let Some(captcha_type) = crate::captcha::CaptchaSolver::detect_captcha(&html.data) {
        eprintln!("hydra-warmup: CAPTCHA detected during warmup on {domain}");
        return Ok(WarmupStatus::CaptchaDetected(format!("{captcha_type:?}")));
    }
    // EC-12.5: Check for block page
    let text = html.data.to_lowercase();
    if text.contains("access denied") || text.contains("ip has been blocked")
        || text.contains("account suspended") || text.contains("unusual activity") {
        eprintln!("hydra-warmup: IP blocked on {domain}");
        return Ok(WarmupStatus::IpBlocked);
    }
    eprintln!("hydra-warmup: {domain} ready ({} scrolls, {}ms delays)", config.scroll_count, config.read_delay_ms);
    Ok(WarmupStatus::Ready)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_domains_need_warmup() {
        assert!(config_for_domain("linkedin.com").is_some());
        assert!(config_for_domain("twitter.com").is_some());
        assert!(config_for_domain("x.com").is_some());
        assert!(config_for_domain("instagram.com").is_some());
    }

    #[test]
    fn unknown_domains_skip_warmup() {
        assert!(config_for_domain("example.com").is_none());
        assert!(config_for_domain("localhost").is_none());
        assert!(config_for_domain("google.com").is_none());
    }

    #[test]
    fn linkedin_has_conservative_config() {
        let config = config_for_domain("linkedin.com").unwrap();
        assert_eq!(config.scroll_count, 3);
        assert!(config.read_delay_ms >= 2000);
        assert!(config.check_session);
    }
}
