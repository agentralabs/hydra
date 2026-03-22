//! Middleware implementations for the cognitive loop.
//!
//! Each middleware hooks into the 5-point pipeline:
//! post_perceive -> post_route -> enrich_prompt -> post_llm -> post_deliver

pub mod growth;
pub mod intelligence;
pub mod memory;
pub mod scheduler;
pub mod security;
pub mod selfmodel;
pub mod settlement;
pub mod signals;

use super::middleware::{CycleMiddleware, MiddlewareChain};

/// Build the full middleware chain with all subsystems wired.
pub fn build_chain() -> MiddlewareChain {
    let middlewares: Vec<Box<dyn CycleMiddleware>> = vec![
        // Wave 2: Foundation
        Box::new(security::SecurityMiddleware::new()),
        Box::new(memory::MemoryMiddleware::new()),
        Box::new(signals::SignalsMiddleware::new()),
        // Wave 3: Intelligence
        Box::new(intelligence::IntelligenceMiddleware::new()),
        Box::new(selfmodel::SelfModelMiddleware::new()),
        // Wave 4: Growth
        Box::new(growth::GrowthMiddleware::new()),
        // Wave 5: Execution
        Box::new(scheduler::SchedulerMiddleware::new()),
        // Wave 7: Settlement
        Box::new(settlement::SettlementMiddleware::new()),
    ];

    MiddlewareChain::new(middlewares)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_chain_creates_all_middlewares() {
        let chain = build_chain();
        assert_eq!(chain.len(), 8);
    }

    #[test]
    fn chain_names_are_unique() {
        let chain = build_chain();
        let names = chain.names();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }
}
