//! Prime AST — semantic representation of intent (not syntax).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The core Prime AST node — represents semantic intent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimeNode {
    // === Data ===
    /// Define an entity (table, struct, model)
    Entity { name: String, fields: Vec<Field> },

    // === CRUD Operations ===
    /// Create a new record
    Create {
        entity: String,
        data: HashMap<String, PrimeValue>,
    },
    /// Read records with optional filter
    Read {
        entity: String,
        filter: Option<Filter>,
        fields: Vec<String>,
    },
    /// Update records matching filter
    Update {
        entity: String,
        filter: Filter,
        data: HashMap<String, PrimeValue>,
    },
    /// Delete records matching filter
    Delete { entity: String, filter: Filter },

    // === Control Flow ===
    /// Execute nodes in sequence
    Sequence(Vec<PrimeNode>),
    /// Conditional execution
    Conditional {
        condition: Condition,
        then: Box<PrimeNode>,
        else_branch: Option<Box<PrimeNode>>,
    },
    /// Loop over a collection
    Loop {
        variable: String,
        collection: PrimeValue,
        body: Box<PrimeNode>,
    },

    // === API ===
    /// Define an HTTP endpoint
    Endpoint {
        method: HttpMethod,
        path: String,
        params: Vec<Field>,
        handler: Box<PrimeNode>,
    },
    /// Define an API with multiple endpoints
    Api {
        name: String,
        endpoints: Vec<PrimeNode>,
    },

    // === Effects ===
    /// Call a function/method
    Call {
        target: String,
        method: String,
        args: Vec<PrimeValue>,
    },
    /// Store a value
    Store { key: String, value: Box<PrimeNode> },
    /// Return a value
    Return { value: PrimeValue },
    /// Assign a variable
    Assign { name: String, value: PrimeValue },
    /// Raw expression (escape hatch)
    Raw { content: String },
}

/// Field definition for entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub type_: PrimeType,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub optional: bool,
}

/// Prime type system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimeType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    Uuid,
    Array(Box<PrimeType>),
    Map,
    Entity(std::string::String),
    Any,
}

/// Field constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Constraint {
    Required,
    Unique,
    PrimaryKey,
    MinLength(usize),
    MaxLength(usize),
    Min(f64),
    Max(f64),
    Pattern(std::string::String),
    Default(PrimeValue),
    ForeignKey {
        entity: std::string::String,
        field: std::string::String,
    },
}

/// Prime value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrimeValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(std::string::String),
    Array(Vec<PrimeValue>),
    Map(HashMap<std::string::String, PrimeValue>),
    Variable(std::string::String),
}

/// Filter for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub op: FilterOp,
    pub value: PrimeValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    In,
}

/// Condition for conditionals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub left: PrimeValue,
    pub op: FilterOp,
    pub right: PrimeValue,
}

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

impl PrimeNode {
    /// Get the node type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Entity { .. } => "entity",
            Self::Create { .. } => "create",
            Self::Read { .. } => "read",
            Self::Update { .. } => "update",
            Self::Delete { .. } => "delete",
            Self::Sequence(_) => "sequence",
            Self::Conditional { .. } => "conditional",
            Self::Loop { .. } => "loop",
            Self::Endpoint { .. } => "endpoint",
            Self::Api { .. } => "api",
            Self::Call { .. } => "call",
            Self::Store { .. } => "store",
            Self::Return { .. } => "return",
            Self::Assign { .. } => "assign",
            Self::Raw { .. } => "raw",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_definition() {
        let entity = PrimeNode::Entity {
            name: "User".into(),
            fields: vec![
                Field {
                    name: "id".into(),
                    type_: PrimeType::Uuid,
                    constraints: vec![Constraint::PrimaryKey],
                    optional: false,
                },
                Field {
                    name: "name".into(),
                    type_: PrimeType::String,
                    constraints: vec![Constraint::Required, Constraint::MaxLength(100)],
                    optional: false,
                },
                Field {
                    name: "email".into(),
                    type_: PrimeType::String,
                    constraints: vec![Constraint::Required, Constraint::Unique],
                    optional: false,
                },
            ],
        };
        assert_eq!(entity.type_name(), "entity");
    }

    #[test]
    fn test_crud_operations() {
        let create = PrimeNode::Create {
            entity: "User".into(),
            data: HashMap::from([
                ("name".into(), PrimeValue::String("Alice".into())),
                (
                    "email".into(),
                    PrimeValue::String("alice@example.com".into()),
                ),
            ]),
        };
        assert_eq!(create.type_name(), "create");

        let read = PrimeNode::Read {
            entity: "User".into(),
            filter: Some(Filter {
                field: "id".into(),
                op: FilterOp::Eq,
                value: PrimeValue::Int(1),
            }),
            fields: vec!["name".into(), "email".into()],
        };
        assert_eq!(read.type_name(), "read");
    }

    #[test]
    fn test_api_definition() {
        let api = PrimeNode::Api {
            name: "UserAPI".into(),
            endpoints: vec![
                PrimeNode::Endpoint {
                    method: HttpMethod::Get,
                    path: "/users".into(),
                    params: vec![],
                    handler: Box::new(PrimeNode::Read {
                        entity: "User".into(),
                        filter: None,
                        fields: vec![],
                    }),
                },
                PrimeNode::Endpoint {
                    method: HttpMethod::Post,
                    path: "/users".into(),
                    params: vec![Field {
                        name: "name".into(),
                        type_: PrimeType::String,
                        constraints: vec![],
                        optional: false,
                    }],
                    handler: Box::new(PrimeNode::Create {
                        entity: "User".into(),
                        data: HashMap::new(),
                    }),
                },
            ],
        };
        assert_eq!(api.type_name(), "api");
    }

    #[test]
    fn test_control_flow() {
        let cond = PrimeNode::Conditional {
            condition: Condition {
                left: PrimeValue::Variable("count".into()),
                op: FilterOp::Gt,
                right: PrimeValue::Int(0),
            },
            then: Box::new(PrimeNode::Return {
                value: PrimeValue::String("found".into()),
            }),
            else_branch: Some(Box::new(PrimeNode::Return {
                value: PrimeValue::String("empty".into()),
            })),
        };
        assert_eq!(cond.type_name(), "conditional");
    }
}
