//! Base vocabulary — the node and edge types that are always present.

use crate::vocab::growth::{is_growth_edge_type, is_growth_node_type};

/// All base node types. Never removed.
pub const BASE_NODE_TYPES: &[&str] = &[
    "Value",
    "TypeDef",
    "Function",
    "Soul",
    "Expression",
    "Statement",
    "Belief",
    "Trust",
    "Signal",
    "Receipt",
    "Agent",
    "Skill",
    "Persona",
    "Intent",
    "CausalAncestor",
];

/// Returns true if a node type name is in the base or growth vocabulary.
pub fn is_base_node_type(name: &str) -> bool {
    BASE_NODE_TYPES.contains(&name) || is_growth_node_type(name)
}

/// All base edge type names. Never removed.
pub const BASE_EDGE_TYPES: &[&str] = &[
    "Contains",
    "References",
    "Extends",
    "Implements",
    "DataFlow",
    "Dependency",
    "ControlFlow",
    "Calls",
    "Remembers",
    "Proves",
    "Foresees",
    "Trusts",
    "SoulBound",
    "CausalLink",
    "TemporalLink",
];

/// Returns true if an edge type name is in the base or growth vocabulary.
pub fn is_base_edge_type(name: &str) -> bool {
    BASE_EDGE_TYPES.contains(&name) || is_growth_edge_type(name)
}
