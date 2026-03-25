//! CachedValue — memoization with TTL for expensive computations.
//! Prevents git subprocess spawns, username lookups, etc. from running every frame.

use std::time::{Duration, Instant};

/// A value cached with a time-to-live. Recomputes only after TTL expires.
pub struct CachedValue<T> {
    value: T,
    last_computed: Instant,
    ttl: Duration,
}

impl<T> CachedValue<T> {
    pub fn new(initial: T, ttl: Duration) -> Self {
        Self { value: initial, last_computed: Instant::now(), ttl }
    }

    /// Get the cached value, recomputing if TTL has expired.
    pub fn get_or_refresh(&mut self, compute: impl FnOnce() -> T) -> &T {
        if self.last_computed.elapsed() >= self.ttl {
            self.value = compute();
            self.last_computed = Instant::now();
        }
        &self.value
    }

    pub fn get(&self) -> &T { &self.value }
}

/// Pre-computed values that rarely change. Updated on TTL expiry.
pub struct FrameCache {
    pub git_branch: CachedValue<String>,
    pub username: CachedValue<String>,
    pub project_path: CachedValue<String>,
}

impl FrameCache {
    pub fn new() -> Self {
        Self {
            git_branch: CachedValue::new(compute_git_branch(), Duration::from_secs(30)),
            username: CachedValue::new(whoami::username(), Duration::from_secs(3600)),
            project_path: CachedValue::new(
                std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default(),
                Duration::from_secs(3600),
            ),
        }
    }

    /// Refresh all values that have expired.
    pub fn refresh(&mut self) {
        self.git_branch.get_or_refresh(compute_git_branch);
    }
}

fn compute_git_branch() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output().ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

impl Default for FrameCache {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_value_returns_initial() {
        let cv = CachedValue::new(42, Duration::from_secs(60));
        assert_eq!(*cv.get(), 42);
    }

    #[test]
    fn cached_value_refreshes_after_zero_ttl() {
        let mut cv = CachedValue::new(0, Duration::from_secs(0));
        let val = cv.get_or_refresh(|| 99);
        assert_eq!(*val, 99);
    }

    #[test]
    fn frame_cache_creates() {
        let _fc = FrameCache::new();
    }
}
