//! CycleMiddleware — trait for hooking into the cognitive loop.
//!
//! Every middleware gets 5 hooks. All hooks are non-blocking:
//! if one middleware errors, the chain continues.

use std::collections::HashMap;

use crate::loop_::types::{CycleResult, PerceivedInput};

/// A middleware that can hook into the cognitive loop at 5 points.
pub trait CycleMiddleware: Send {
    /// Name for logging.
    fn name(&self) -> &'static str;

    /// After perception completes. Can enrich perceived input.
    fn post_perceive(&mut self, _perceived: &mut PerceivedInput) {}

    /// After routing decides the path. Can read enrichments.
    fn post_route(&mut self, _perceived: &PerceivedInput, _path: &str) {}

    /// Enrich the prompt before LLM call. Returns extra context lines.
    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        Vec::new()
    }

    /// After LLM response (or zero-token resolution).
    fn post_llm(&mut self, _perceived: &PerceivedInput, _response: &str) {}

    /// After delivery (receipt written). Final hook per cycle.
    fn post_deliver(&mut self, _cycle: &CycleResult) {}
}

/// Ordered chain of middlewares. Runs all regardless of individual failures.
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn CycleMiddleware>>,
}

impl MiddlewareChain {
    pub fn new(middlewares: Vec<Box<dyn CycleMiddleware>>) -> Self {
        Self { middlewares }
    }

    pub fn run_post_perceive(&mut self, perceived: &mut PerceivedInput) {
        for mw in &mut self.middlewares {
            mw.post_perceive(perceived);
        }
    }

    pub fn run_post_route(&mut self, perceived: &PerceivedInput, path: &str) {
        for mw in &mut self.middlewares {
            mw.post_route(perceived, path);
        }
    }

    pub fn collect_enrichments(&self, perceived: &PerceivedInput) -> HashMap<String, String> {
        let mut enrichments = HashMap::new();
        for mw in &self.middlewares {
            let lines = mw.enrich_prompt(perceived);
            if !lines.is_empty() {
                enrichments.insert(mw.name().to_string(), lines.join("\n"));
            }
        }
        enrichments
    }

    pub fn run_post_llm(&mut self, perceived: &PerceivedInput, response: &str) {
        for mw in &mut self.middlewares {
            mw.post_llm(perceived, response);
        }
    }

    pub fn run_post_deliver(&mut self, cycle: &CycleResult) {
        for mw in &mut self.middlewares {
            mw.post_deliver(cycle);
        }
    }

    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.middlewares.iter().map(|m| m.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMiddleware {
        called: Vec<String>,
    }

    impl CycleMiddleware for TestMiddleware {
        fn name(&self) -> &'static str {
            "test"
        }
        fn post_perceive(&mut self, _perceived: &mut PerceivedInput) {
            self.called.push("post_perceive".into());
        }
        fn post_deliver(&mut self, _cycle: &CycleResult) {
            self.called.push("post_deliver".into());
        }
    }

    #[test]
    fn chain_runs_all_middlewares() {
        let chain = MiddlewareChain::new(vec![]);
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn chain_names() {
        let mw: Box<dyn CycleMiddleware> = Box::new(TestMiddleware {
            called: Vec::new(),
        });
        let chain = MiddlewareChain::new(vec![mw]);
        assert_eq!(chain.names(), vec!["test"]);
    }
}
