//! Animus Prime — Hydra's internal cognitive language.
//!
//! - **Prime**: Semantic AST, parser, validator, serialization
//! - **Script**: Human-readable Animus Script surface syntax
//! - **Compiler**: Prime AST → JavaScript/Python/Rust/Go/SQL/Shell
//! - **Integration**: Wire into Hydra cognitive loop

pub mod compiler;
pub mod integration;
pub mod prime;
pub mod script;
