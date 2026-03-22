//! Scheduler middleware — hydra-scheduler job firing per-request.
//!
//! Ticks the scheduler on every cycle. Fires any jobs whose
//! constraints have been met.

use hydra_scheduler::SchedulerEngine;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct SchedulerMiddleware {
    scheduler: SchedulerEngine,
}

impl SchedulerMiddleware {
    pub fn new() -> Self {
        Self {
            scheduler: SchedulerEngine::new(),
        }
    }
}

impl CycleMiddleware for SchedulerMiddleware {
    fn name(&self) -> &'static str {
        "scheduler"
    }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        // Tick the scheduler — fire any ready jobs
        let result = self.scheduler.tick();
        if !result.fired.is_empty() {
            perceived.enrichments.insert(
                "scheduler.fired".into(),
                format!("{} jobs fired: {}", result.fired.len(), result.fired.join(", ")),
            );
            eprintln!(
                "hydra: scheduler fired {} jobs",
                result.fired.len()
            );
        }
    }

    fn post_deliver(&mut self, _cycle: &CycleResult) {
        // Tick again after delivery to catch any time-based triggers
        let _ = self.scheduler.tick();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_middleware_name() {
        let mw = SchedulerMiddleware::new();
        assert_eq!(mw.name(), "scheduler");
    }
}

impl Default for SchedulerMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
