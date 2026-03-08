//! Animus Script — human-readable surface syntax for Prime AST.

pub mod lexer;
pub mod parser;
pub mod printer;

pub use lexer::{Lexer, Token, TokenKind};
pub use parser::ScriptParser;
pub use printer::ScriptPrinter;
