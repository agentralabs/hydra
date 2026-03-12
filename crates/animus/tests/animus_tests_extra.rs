//! Extra integration tests for Animus Prime.

use animus::compiler::{self, Target};
use animus::integration::hydra::{AnimusEngine, CognitivePhase};
use animus::prime::validator::PrimeValidator;
use animus::script::lexer::Lexer;
use animus::script::parser::ScriptParser;

#[test]
fn test_end_to_end_script_to_code() {
    // Script → tokens → AST → validate → compile to JS
    let script = "entity User { name: string, age: int }";
    let tokens = Lexer::new(script).tokenize();
    let mut parser = ScriptParser::new(tokens);
    let ast = parser.parse().unwrap();

    let validation = PrimeValidator::validate(&ast);
    assert!(validation.valid);

    let js = compiler::compile(&ast, Target::JavaScript).unwrap();
    assert!(js.code.contains("class User"));

    let py = compiler::compile(&ast, Target::Python).unwrap();
    assert!(py.code.contains("@dataclass"));
}
