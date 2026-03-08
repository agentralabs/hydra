//! Integration tests for Animus Prime.

use animus::compiler::{self, Target};
use animus::integration::hydra::{AnimusEngine, CognitivePhase};
use animus::prime::ast::*;
use animus::prime::parser::PrimeParser;
use animus::prime::serialize::PrimeSerializer;
use animus::prime::validator::PrimeValidator;
use animus::script::lexer::Lexer;
use animus::script::parser::ScriptParser;
use animus::script::printer::ScriptPrinter;

use std::collections::HashMap;

// === Prime AST Tests ===

#[test]
fn test_prime_ast_all_node_types() {
    let nodes = vec![
        PrimeNode::Entity {
            name: "Test".into(),
            fields: vec![],
        },
        PrimeNode::Create {
            entity: "Test".into(),
            data: HashMap::new(),
        },
        PrimeNode::Read {
            entity: "Test".into(),
            filter: None,
            fields: vec![],
        },
        PrimeNode::Return {
            value: PrimeValue::Null,
        },
        PrimeNode::Assign {
            name: "x".into(),
            value: PrimeValue::Int(42),
        },
        PrimeNode::Raw {
            content: "test".into(),
        },
    ];
    assert_eq!(nodes[0].type_name(), "entity");
    assert_eq!(nodes[1].type_name(), "create");
    assert_eq!(nodes[2].type_name(), "read");
    assert_eq!(nodes[3].type_name(), "return");
    assert_eq!(nodes[4].type_name(), "assign");
    assert_eq!(nodes[5].type_name(), "raw");
}

// === Parser Tests ===

#[test]
fn test_parse_complex_api() {
    let json = serde_json::json!({
        "type": "api",
        "name": "UserService",
        "endpoints": [
            {
                "type": "endpoint",
                "method": "GET",
                "path": "/users",
                "handler": { "type": "read", "entity": "User" }
            },
            {
                "type": "endpoint",
                "method": "POST",
                "path": "/users",
                "handler": {
                    "type": "create",
                    "entity": "User",
                    "data": { "name": "test" }
                }
            }
        ]
    });
    let node = PrimeParser::parse(&json).unwrap();
    assert_eq!(node.type_name(), "api");
}

#[test]
fn test_parse_conditional_with_else() {
    let json = serde_json::json!({
        "type": "conditional",
        "condition": {
            "left": "$count",
            "op": "gt",
            "right": 0
        },
        "then": { "type": "return", "value": "found" },
        "else": { "type": "return", "value": "empty" }
    });
    let node = PrimeParser::parse(&json).unwrap();
    assert_eq!(node.type_name(), "conditional");
}

// === Validator Tests ===

#[test]
fn test_validate_nested_structure() {
    let api = PrimeNode::Api {
        name: "TestAPI".into(),
        endpoints: vec![PrimeNode::Endpoint {
            method: HttpMethod::Get,
            path: "/items".into(),
            params: vec![],
            handler: Box::new(PrimeNode::Sequence(vec![
                PrimeNode::Read {
                    entity: "Item".into(),
                    filter: None,
                    fields: vec![],
                },
                PrimeNode::Return {
                    value: PrimeValue::String("ok".into()),
                },
            ])),
        }],
    };
    let result = PrimeValidator::validate(&api);
    assert!(result.valid);
}

#[test]
fn test_validate_catches_errors() {
    let node = PrimeNode::Entity {
        name: String::new(),
        fields: vec![],
    };
    let result = PrimeValidator::validate(&node);
    assert!(!result.valid);
    assert!(!result.errors.is_empty());
}

// === Serialization Tests ===

#[test]
fn test_serialize_round_trip() {
    let node = PrimeNode::Create {
        entity: "User".into(),
        data: HashMap::from([
            ("name".into(), PrimeValue::String("Alice".into())),
            ("age".into(), PrimeValue::Int(30)),
        ]),
    };
    let json = PrimeSerializer::to_json(&node).unwrap();
    let restored = PrimeSerializer::from_json(&json).unwrap();
    assert_eq!(restored.type_name(), "create");
}

// === Lexer Tests ===

#[test]
fn test_lexer_full_script() {
    let script = r#"
entity User {
    name: string,
    age: int
}
create User { name: "Alice", age: 30 }
"#;
    let tokens = Lexer::new(script).tokenize();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert!(kinds.contains(&&animus::script::lexer::TokenKind::Entity));
    assert!(kinds.contains(&&animus::script::lexer::TokenKind::Create));
}

// === Script Parser Tests ===

#[test]
fn test_script_parse_entity() {
    let tokens = Lexer::new("entity Product { name: string, price: float }").tokenize();
    let mut parser = ScriptParser::new(tokens);
    let node = parser.parse().unwrap();
    assert_eq!(node.type_name(), "entity");
}

#[test]
fn test_script_parse_assign_and_return() {
    let script = "let x = 42\nreturn \"done\"";
    let tokens = Lexer::new(script).tokenize();
    let mut parser = ScriptParser::new(tokens);
    let node = parser.parse().unwrap();
    assert_eq!(node.type_name(), "sequence");
}

// === Script Printer Tests ===

#[test]
fn test_printer_entity_output() {
    let node = PrimeNode::Entity {
        name: "Item".into(),
        fields: vec![
            Field {
                name: "id".into(),
                type_: PrimeType::Uuid,
                constraints: vec![],
                optional: false,
            },
            Field {
                name: "name".into(),
                type_: PrimeType::String,
                constraints: vec![],
                optional: false,
            },
        ],
    };
    let output = ScriptPrinter::new().print(&node);
    assert!(output.contains("entity Item"));
    assert!(output.contains("id: uuid"));
    assert!(output.contains("name: string"));
}

// === Compiler Tests ===

#[test]
fn test_compile_to_all_targets() {
    let node = PrimeNode::Entity {
        name: "User".into(),
        fields: vec![
            Field {
                name: "id".into(),
                type_: PrimeType::Uuid,
                constraints: vec![],
                optional: false,
            },
            Field {
                name: "name".into(),
                type_: PrimeType::String,
                constraints: vec![],
                optional: false,
            },
        ],
    };
    let targets = vec![
        Target::JavaScript,
        Target::Python,
        Target::Rust,
        Target::Go,
        Target::Sql,
    ];
    for target in targets {
        let result = compiler::compile(&node, target).unwrap();
        assert!(!result.code.is_empty(), "empty output for {:?}", target);
    }
}

#[test]
fn test_js_output_idiomatic() {
    let node = PrimeNode::Entity {
        name: "User".into(),
        fields: vec![Field {
            name: "name".into(),
            type_: PrimeType::String,
            constraints: vec![],
            optional: false,
        }],
    };
    let result = compiler::compile(&node, Target::JavaScript).unwrap();
    assert!(result.code.contains("class User"));
    assert!(result.code.contains("constructor"));
    assert!(result.code.contains("this.name"));
}

#[test]
fn test_python_output_idiomatic() {
    let node = PrimeNode::Entity {
        name: "User".into(),
        fields: vec![Field {
            name: "name".into(),
            type_: PrimeType::String,
            constraints: vec![],
            optional: false,
        }],
    };
    let result = compiler::compile(&node, Target::Python).unwrap();
    assert!(result.code.contains("@dataclass"));
    assert!(result.code.contains("class User:"));
    assert!(result.code.contains("name: str"));
}

#[test]
fn test_rust_output_idiomatic() {
    let node = PrimeNode::Entity {
        name: "User".into(),
        fields: vec![Field {
            name: "name".into(),
            type_: PrimeType::String,
            constraints: vec![],
            optional: false,
        }],
    };
    let result = compiler::compile(&node, Target::Rust).unwrap();
    assert!(result.code.contains("pub struct User"));
    assert!(result.code.contains("#[derive("));
    assert!(result.code.contains("pub name: String"));
}

#[test]
fn test_go_output_idiomatic() {
    let node = PrimeNode::Entity {
        name: "User".into(),
        fields: vec![Field {
            name: "name".into(),
            type_: PrimeType::String,
            constraints: vec![],
            optional: false,
        }],
    };
    let result = compiler::compile(&node, Target::Go).unwrap();
    assert!(result.code.contains("type User struct"));
    assert!(result.code.contains("`json:\"name\"`"));
}

#[test]
fn test_sql_creates_table() {
    let node = PrimeNode::Entity {
        name: "User".into(),
        fields: vec![
            Field {
                name: "id".into(),
                type_: PrimeType::Uuid,
                constraints: vec![Constraint::PrimaryKey],
                optional: false,
            },
            Field {
                name: "email".into(),
                type_: PrimeType::String,
                constraints: vec![Constraint::Unique],
                optional: false,
            },
        ],
    };
    let result = compiler::compile(&node, Target::Sql).unwrap();
    assert!(result.code.contains("CREATE TABLE"));
    assert!(result.code.contains("PRIMARY KEY"));
    assert!(result.code.contains("UNIQUE"));
}

#[test]
fn test_shell_generates_script() {
    let node = PrimeNode::Conditional {
        condition: Condition {
            left: PrimeValue::Variable("count".into()),
            op: FilterOp::Gt,
            right: PrimeValue::Int(0),
        },
        then: Box::new(PrimeNode::Return {
            value: PrimeValue::String("found".into()),
        }),
        else_branch: None,
    };
    let result = compiler::compile(&node, Target::Shell).unwrap();
    assert!(result.code.contains("#!/bin/bash"));
    assert!(result.code.contains("if ["));
}

// === Integration Tests ===

#[test]
fn test_animus_engine_full_pipeline() {
    let engine = AnimusEngine::with_targets(vec![Target::JavaScript, Target::Python, Target::Sql]);
    let json = serde_json::json!({
        "type": "entity",
        "name": "Order",
        "fields": [
            { "name": "id", "type": "uuid" },
            { "name": "amount", "type": "float" },
            { "name": "status", "type": "string" },
        ]
    });
    let result = engine.process(&json, CognitivePhase::Act).unwrap();
    assert!(result.validation.as_ref().unwrap().valid);
    assert_eq!(result.compiled.len(), 3);
    assert!(result.compiled["javascript"].contains("class Order"));
    assert!(result.compiled["python"].contains("class Order"));
    assert!(result.compiled["sql"].contains("CREATE TABLE"));
}

#[test]
fn test_animus_engine_validation_failure() {
    let engine = AnimusEngine::new();
    let json = serde_json::json!({
        "type": "entity",
        "name": "",
        "fields": []
    });
    let result = engine.process(&json, CognitivePhase::Act).unwrap();
    assert!(!result.validation.as_ref().unwrap().valid);
}

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
