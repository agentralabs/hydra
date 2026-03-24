//! Self-model middleware — reflexive + morphic + persona + recovery + transform.

use hydra_morphic::{MorphicEventKind, MorphicIdentity};
use hydra_persona::PersonaRegistry;
use hydra_reflexive::SelfModel;
use hydra_resurrection::KernelStateSnapshot;
use hydra_transform::TransformEngine;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct SelfModelMiddleware {
    model: SelfModel,
    identity: MorphicIdentity,
    #[allow(dead_code)]
    personas: PersonaRegistry,
    last_snapshot: KernelStateSnapshot,
    transform: TransformEngine,
    cycles_processed: u64,
}

impl SelfModelMiddleware {
    pub fn new() -> Self {
        Self {
            model: SelfModel::bootstrap_layer1(),
            identity: MorphicIdentity::genesis(),
            personas: PersonaRegistry::new(),
            last_snapshot: KernelStateSnapshot::initial(),
            transform: TransformEngine::new(),
            cycles_processed: 0,
        }
    }
}

impl CycleMiddleware for SelfModelMiddleware {
    fn name(&self) -> &'static str {
        "selfmodel"
    }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        // Tick the self-model
        self.model.tick();

        // Add capability count as enrichment
        let caps = self.model.active_capabilities();
        if !caps.is_empty() {
            perceived.enrichments.insert(
                "selfmodel.capabilities".into(),
                format!("{} active capabilities", caps.len()),
            );
        }
    }

    fn post_deliver(&mut self, _cycle: &CycleResult) {
        self.cycles_processed += 1;

        // Deepen morphic hash chain on every cycle
        if let Err(e) = self.identity.record_event(MorphicEventKind::CapabilityAdded {
            name: format!("cycle-{}", self.cycles_processed),
        }) {
            eprintln!("hydra: selfmodel morphic record: {e}");
        }

        // Log milestones
        if self.cycles_processed % 100 == 0 {
            eprintln!(
                "hydra: selfmodel milestone: {} cycles, {} capabilities, morphic depth={}",
                self.cycles_processed,
                self.model.active_capabilities().len(),
                self.identity.depth()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selfmodel_middleware_name() {
        let mw = SelfModelMiddleware::new();
        assert_eq!(mw.name(), "selfmodel");
    }

    #[test]
    fn selfmodel_starts_with_capabilities() {
        let mw = SelfModelMiddleware::new();
        assert!(!mw.model.active_capabilities().is_empty());
    }
}

impl Default for SelfModelMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
