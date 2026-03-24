//! Per-domain rate limiter — prevents platform detection via request frequency.
//! Consulted before every mutation action in BrowserEngine.

use std::collections::HashMap;
use std::time::Instant;

/// Per-domain action rate tracker.
pub struct RateLimiter {
    domains: HashMap<String, DomainRate>,
}

struct DomainRate {
    action_count: u32,
    window_start: Instant,
    backoff_until: Option<Instant>,
    backoff_multiplier: u32,
}

/// Status returned by the limiter before each action.
#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitStatus {
    /// Proceed normally.
    Ok,
    /// Throttled — wait this many ms before proceeding.
    Throttled { wait_ms: u64 },
    /// Domain is in backoff from a 429 response (EC-12.2).
    BackedOff { remaining_ms: u64 },
}

impl RateLimiter {
    pub fn new() -> Self {
        Self { domains: HashMap::new() }
    }

    /// Check if an action is allowed for this domain.
    pub fn check(&mut self, domain: &str, actions_per_minute: u32) -> RateLimitStatus {
        let now = Instant::now();
        let rate = self.domains.entry(domain.to_string()).or_insert(DomainRate {
            action_count: 0, window_start: now, backoff_until: None, backoff_multiplier: 1,
        });
        // Check backoff (EC-12.2)
        if let Some(until) = rate.backoff_until {
            if now < until {
                return RateLimitStatus::BackedOff { remaining_ms: (until - now).as_millis() as u64 };
            }
            rate.backoff_until = None; // Backoff expired
        }
        // Reset window if 60s passed
        if now.duration_since(rate.window_start).as_secs() >= 60 {
            rate.action_count = 0;
            rate.window_start = now;
        }
        if rate.action_count >= actions_per_minute {
            let elapsed = now.duration_since(rate.window_start).as_millis() as u64;
            let wait = 60_000u64.saturating_sub(elapsed);
            return RateLimitStatus::Throttled { wait_ms: wait };
        }
        rate.action_count += 1;
        RateLimitStatus::Ok
    }

    /// Record a 429 response — triggers exponential backoff (EC-12.2).
    pub fn record_429(&mut self, domain: &str) {
        let now = Instant::now();
        let rate = self.domains.entry(domain.to_string()).or_insert(DomainRate {
            action_count: 0, window_start: now, backoff_until: None, backoff_multiplier: 1,
        });
        let backoff_ms = (30_000u64 * rate.backoff_multiplier as u64).min(900_000);
        rate.backoff_until = Some(now + std::time::Duration::from_millis(backoff_ms));
        rate.backoff_multiplier = (rate.backoff_multiplier * 2).min(30);
        eprintln!("hydra-limiter: 429 on {domain} — backoff {backoff_ms}ms (mult={})", rate.backoff_multiplier);
    }

    /// Record a successful action — resets backoff multiplier.
    pub fn record_success(&mut self, domain: &str) {
        if let Some(rate) = self.domains.get_mut(domain) {
            rate.backoff_multiplier = 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_action_allowed() {
        let mut limiter = RateLimiter::new();
        assert_eq!(limiter.check("example.com", 30), RateLimitStatus::Ok);
    }

    #[test]
    fn exceeding_limit_throttles() {
        let mut limiter = RateLimiter::new();
        for _ in 0..30 { limiter.check("test.com", 30); }
        let status = limiter.check("test.com", 30);
        assert!(matches!(status, RateLimitStatus::Throttled { .. }));
    }

    #[test]
    fn backoff_after_429() {
        let mut limiter = RateLimiter::new();
        limiter.record_429("api.com");
        let status = limiter.check("api.com", 30);
        assert!(matches!(status, RateLimitStatus::BackedOff { .. }));
    }

    #[test]
    fn success_resets_multiplier() {
        let mut limiter = RateLimiter::new();
        limiter.record_429("x.com");
        limiter.record_success("x.com");
        // Multiplier reset but backoff still active (time-based)
        assert!(limiter.domains.get("x.com").unwrap().backoff_multiplier == 1);
    }
}
