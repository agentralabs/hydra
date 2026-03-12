//! Error recovery patterns — classify errors, suggest actions, retry with backoff.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Network,
    Parse,
    Timeout,
    Auth,
    RateLimit,
    Unknown,
}

/// Classify an error message into an ErrorKind by pattern matching.
pub fn classify_error(error_msg: &str) -> ErrorKind {
    let lower = error_msg.to_lowercase();
    if lower.contains("rate limit") || lower.contains("429") || lower.contains("too many requests") {
        ErrorKind::RateLimit
    } else if lower.contains("unauthorized") || lower.contains("401") || lower.contains("forbidden")
        || lower.contains("invalid api key") || lower.contains("auth")
    {
        ErrorKind::Auth
    } else if lower.contains("timed out") || lower.contains("timeout") || lower.contains("deadline") {
        ErrorKind::Timeout
    } else if lower.contains("parse") || lower.contains("json") || lower.contains("expected")
        || lower.contains("invalid format") || lower.contains("deserialize")
    {
        ErrorKind::Parse
    } else if lower.contains("network") || lower.contains("connection") || lower.contains("http error")
        || lower.contains("builder error") || lower.contains("dns") || lower.contains("refused")
    {
        ErrorKind::Network
    } else {
        ErrorKind::Unknown
    }
}

/// Suggest a recovery action for a given error kind.
pub fn recovery_action(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Network => "Retry after checking network connectivity",
        ErrorKind::Parse => "Log the raw response and retry with stricter prompt",
        ErrorKind::Timeout => "Retry with a shorter prompt or increased timeout",
        ErrorKind::Auth => "Check API key validity — do not retry without fixing credentials",
        ErrorKind::RateLimit => "Back off and retry after the suggested wait period",
        ErrorKind::Unknown => "Log the error for analysis and retry once",
    }
}

/// Exponential backoff with jitter: base_ms * 2^attempt + random(0..base_ms).
pub fn backoff_delay(attempt: u32, base_ms: u64) -> u64 {
    let exp = base_ms.saturating_mul(1u64 << attempt.min(10));
    let jitter = exp / 4; // deterministic jitter approximation
    exp.saturating_add(jitter)
}

/// Tracks error frequency by kind.
pub struct ErrorTracker {
    counts: std::collections::HashMap<ErrorKind, usize>,
}

impl ErrorTracker {
    pub fn new() -> Self {
        Self { counts: std::collections::HashMap::new() }
    }

    pub fn track(&mut self, kind: ErrorKind) {
        *self.counts.entry(kind).or_insert(0) += 1;
    }

    pub fn count(&self, kind: ErrorKind) -> usize {
        self.counts.get(&kind).copied().unwrap_or(0)
    }

    pub fn most_common(&self) -> Option<(ErrorKind, usize)> {
        self.counts.iter().max_by_key(|(_, &v)| v).map(|(&k, &v)| (k, v))
    }

    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_network() {
        assert_eq!(classify_error("HTTP error: builder error"), ErrorKind::Network);
        assert_eq!(classify_error("connection refused"), ErrorKind::Network);
    }

    #[test]
    fn test_classify_auth() {
        assert_eq!(classify_error("401 Unauthorized"), ErrorKind::Auth);
        assert_eq!(classify_error("invalid api key provided"), ErrorKind::Auth);
    }

    #[test]
    fn test_classify_rate_limit() {
        assert_eq!(classify_error("429 Too Many Requests"), ErrorKind::RateLimit);
        assert_eq!(classify_error("rate limit exceeded"), ErrorKind::RateLimit);
    }

    #[test]
    fn test_classify_timeout() {
        assert_eq!(classify_error("request timed out"), ErrorKind::Timeout);
    }

    #[test]
    fn test_classify_parse() {
        assert_eq!(classify_error("JSON parse error at line 5"), ErrorKind::Parse);
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(classify_error("something went wrong"), ErrorKind::Unknown);
    }

    #[test]
    fn test_recovery_actions() {
        assert!(recovery_action(ErrorKind::Auth).contains("API key"));
        assert!(recovery_action(ErrorKind::RateLimit).contains("Back off"));
    }

    #[test]
    fn test_backoff_exponential() {
        let d0 = backoff_delay(0, 100);
        let d1 = backoff_delay(1, 100);
        let d2 = backoff_delay(2, 100);
        assert!(d1 > d0);
        assert!(d2 > d1);
    }

    #[test]
    fn test_error_tracker() {
        let mut tracker = ErrorTracker::new();
        tracker.track(ErrorKind::Network);
        tracker.track(ErrorKind::Network);
        tracker.track(ErrorKind::Timeout);
        assert_eq!(tracker.count(ErrorKind::Network), 2);
        assert_eq!(tracker.count(ErrorKind::Timeout), 1);
        assert_eq!(tracker.most_common(), Some((ErrorKind::Network, 2)));
        assert_eq!(tracker.total(), 3);
    }
}
