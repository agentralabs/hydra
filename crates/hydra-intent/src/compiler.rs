// compiler.rs — thin re-export module
//
// The compiler implementation is split across:
//   compiler_types.rs    — Complexity, CompileStatus, CompileResult
//   compiler_stages.rs   — entity extraction, action classification, complexity assessment
//   compiler_pipeline.rs — IntentCompiler struct and 7-stage pipeline
//   compiler_tests.rs    — all tests

pub use crate::compiler_pipeline::IntentCompiler;
pub use crate::compiler_types::{CompileResult, CompileStatus, Complexity};
