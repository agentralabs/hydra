#[path = "bridges_core.rs"]
mod bridges_core;
#[path = "bridges_extended.rs"]
mod bridges_extended;
#[path = "bridges_tests.rs"]
mod bridges_tests;
#[path = "bridges_tests_memory.rs"]
mod bridges_tests_memory;

// Re-export everything so existing `use crate::bridges::*` and
// `use hydra_sisters::bridges` continue to work unchanged.
pub use bridges_core::*;
pub use bridges_extended::*;

/// Create all 14 bridges
pub fn all_bridges() -> Vec<McpSisterBridge> {
    vec![
        memory_bridge(),
        vision_bridge(),
        codebase_bridge(),
        identity_bridge(),
        time_bridge(),
        contract_bridge(),
        comm_bridge(),
        planning_bridge(),
        cognition_bridge(),
        reality_bridge(),
        forge_bridge(),
        aegis_bridge(),
        veritas_bridge(),
        evolve_bridge(),
    ]
}
