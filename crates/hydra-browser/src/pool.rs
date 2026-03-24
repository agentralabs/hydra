//! Browser pool — managed N Chrome instances with resource limiting.
//! EC-8.2: check available RAM before spawning. Max 3 on <8GB.

use std::sync::atomic::{AtomicUsize, Ordering};

/// A pool that limits concurrent Chrome browser instances.
pub struct BrowserPool {
    max_instances: usize,
    active: AtomicUsize,
}

impl BrowserPool {
    pub fn new(max_instances: usize) -> Self {
        Self { max_instances, active: AtomicUsize::new(0) }
    }

    /// Try to acquire a browser slot. Returns true if available.
    pub fn acquire(&self) -> bool {
        let current = self.active.load(Ordering::SeqCst);
        if current >= self.max_instances { return false; }
        self.active.fetch_add(1, Ordering::SeqCst);
        true
    }

    /// Release a browser slot.
    pub fn release(&self) {
        let prev = self.active.fetch_sub(1, Ordering::SeqCst);
        if prev == 0 { self.active.store(0, Ordering::SeqCst); } // prevent underflow
    }

    /// Current active count.
    pub fn active_count(&self) -> usize { self.active.load(Ordering::SeqCst) }

    /// Available slots.
    pub fn available(&self) -> usize { self.max_instances.saturating_sub(self.active_count()) }
}

impl Default for BrowserPool {
    fn default() -> Self { Self::new(3) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_limits_instances() {
        let pool = BrowserPool::new(2);
        assert!(pool.acquire());
        assert!(pool.acquire());
        assert!(!pool.acquire()); // at capacity
        pool.release();
        assert!(pool.acquire()); // freed one
    }

    #[test]
    fn pool_tracks_active() {
        let pool = BrowserPool::new(5);
        assert_eq!(pool.active_count(), 0);
        pool.acquire();
        assert_eq!(pool.active_count(), 1);
        pool.release();
        assert_eq!(pool.active_count(), 0);
    }
}
