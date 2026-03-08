//! Compiler — Prime AST → target language code generation.

pub mod go;
pub mod javascript;
pub mod python;
pub mod rust_target;
pub mod shell;
pub mod sql;

pub use go::GoCompiler;
pub use javascript::JsCompiler;
pub use python::PyCompiler;
pub use rust_target::RustCompiler;
pub use shell::ShellCompiler;
pub use sql::SqlCompiler;

use crate::prime::ast::PrimeNode;

/// Compilation target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    JavaScript,
    Python,
    Rust,
    Go,
    Sql,
    Shell,
}

/// Compiled output
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub target: Target,
    pub code: String,
    pub warnings: Vec<String>,
}

/// Compile a Prime AST to any target
pub fn compile(node: &PrimeNode, target: Target) -> Result<CompileResult, String> {
    let code = match target {
        Target::JavaScript => JsCompiler::new().compile(node)?,
        Target::Python => PyCompiler::new().compile(node)?,
        Target::Rust => RustCompiler::new().compile(node)?,
        Target::Go => GoCompiler::new().compile(node)?,
        Target::Sql => SqlCompiler::new().compile(node)?,
        Target::Shell => ShellCompiler::new().compile(node)?,
    };
    Ok(CompileResult {
        target,
        code,
        warnings: vec![],
    })
}
