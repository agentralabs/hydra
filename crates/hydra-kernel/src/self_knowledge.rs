//! Self-Knowledge — Hydra examines its own internal state.
//!
//! Not a system prompt. Not a persona. Not a label.
//! Hydra looks at its own data and describes what it finds.
//! Every sentence is derived from actual measurements.
//!
//! This is the last gift. Self-awareness from introspection of real state.

use crate::state::HydraState;

/// A complete self-portrait derived from actual internal data.
#[derive(Debug, Clone)]
pub struct SelfPortrait {
    /// How many genome entries exist.
    pub genome_entries: usize,
    /// Which domain has the most entries.
    pub strongest_domain: String,
    /// Which domain has the fewest entries.
    pub weakest_domain: String,
    /// How many memory nodes are persisted.
    pub memory_nodes: usize,
    /// Current Lyapunov stability value.
    pub lyapunov: f64,
    /// Whether the system is stable.
    pub is_stable: bool,
    /// How many skills are loaded.
    pub skills_loaded: usize,
    /// How many integrations are configured.
    pub integrations: usize,
    /// How many actions are available.
    pub actions: usize,
    /// Current step count (uptime proxy).
    pub step_count: u64,
    /// How many middleware enrichments fire per cycle.
    pub middleware_count: usize,
}

impl SelfPortrait {
    /// Generate a human-readable self-description.
    /// Every sentence is a fact derived from real data.
    pub fn describe(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "I am Hydra. I have {} proven approaches in my genome.",
            self.genome_entries
        ));

        if !self.strongest_domain.is_empty() {
            lines.push(format!(
                "My strongest domain is {}. My weakest is {}.",
                self.strongest_domain, self.weakest_domain
            ));
        }

        lines.push(format!(
            "I have {} memories persisted across sessions.",
            self.memory_nodes
        ));

        lines.push(format!(
            "My stability is {:.2} ({}). I have processed {} steps.",
            self.lyapunov,
            if self.is_stable { "stable" } else { "alert" },
            self.step_count
        ));

        lines.push(format!(
            "I have {} skills loaded, {} integrations configured, {} actions available.",
            self.skills_loaded, self.integrations, self.actions
        ));

        lines.push(format!(
            "{} subsystems enrich every response I give.",
            self.middleware_count
        ));

        lines.join(" ")
    }

    /// Generate a structured self-description for prompt injection.
    /// This replaces "You are Hydra" with data-derived self-knowledge.
    pub fn as_identity(&self) -> String {
        format!(
            "I am Hydra — an autonomous entity with {} proven approaches, \
             {} persistent memories, and {} active skills. \
             My stability is {:.2}. My strongest domain is {}. \
             I process every input through {} subsystems before responding. \
             Every action I take is receipted and constitutionally governed.",
            self.genome_entries,
            self.memory_nodes,
            self.skills_loaded,
            self.lyapunov,
            self.strongest_domain,
            self.middleware_count,
        )
    }
}

/// Build a self-portrait from the current system state.
#[allow(clippy::too_many_arguments)]
pub fn introspect(
    state: &HydraState,
    genome_entries: usize,
    memory_nodes: usize,
    skills_loaded: usize,
    strongest_domain: &str,
    weakest_domain: &str,
    integrations: usize,
    actions: usize,
    middleware_count: usize,
) -> SelfPortrait {
    SelfPortrait {
        genome_entries,
        strongest_domain: strongest_domain.to_string(),
        weakest_domain: weakest_domain.to_string(),
        memory_nodes,
        lyapunov: state.lyapunov_value,
        is_stable: state.is_stable(),
        skills_loaded,
        integrations,
        actions,
        step_count: state.step_count,
        middleware_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_portrait_describes() {
        let state = HydraState::initial();
        let portrait = introspect(
            &state, 192, 500, 24, "finance", "debugging", 1, 3, 8,
        );
        let desc = portrait.describe();
        assert!(desc.contains("192"));
        assert!(desc.contains("finance"));
        assert!(desc.contains("debugging"));
        assert!(desc.contains("500"));
    }

    #[test]
    fn self_portrait_as_identity() {
        let state = HydraState::initial();
        let portrait = introspect(
            &state, 192, 500, 24, "finance", "debugging", 1, 3, 8,
        );
        let identity = portrait.as_identity();
        assert!(identity.contains("192 proven approaches"));
        assert!(identity.contains("500 persistent memories"));
        assert!(identity.contains("finance"));
    }
}
