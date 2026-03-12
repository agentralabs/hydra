//! Codebase sister semantic wiring — AST-aware code understanding.
//!
//! Phase 3, C6: Replaces grep-based code queries with semantic search,
//! impact analysis, and self-understanding capabilities via the
//! Codebase sister's MCP tools.

pub mod semantic;

pub use semantic::{
    CodebaseSemanticEngine, ImpactReport, SemanticSearchResult,
    SemanticSearchHit, ImpactEntry,
};
