//! PrimeValidator — type checking and coherence validation.

use super::ast::*;

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: vec![],
            warnings: vec![],
        }
    }
    pub fn fail(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: vec![],
        }
    }
}

/// Validates Prime AST for type coherence
pub struct PrimeValidator;

impl PrimeValidator {
    /// Validate a Prime AST node
    pub fn validate(node: &PrimeNode) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        Self::validate_node(node, &mut errors, &mut warnings);
        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    fn validate_node(node: &PrimeNode, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
        match node {
            PrimeNode::Entity { name, fields } => {
                if name.is_empty() {
                    errors.push("entity name cannot be empty".into());
                }
                if fields.is_empty() {
                    warnings.push(format!("entity '{}' has no fields", name));
                }
                for field in fields {
                    if field.name.is_empty() {
                        errors.push("field name cannot be empty".into());
                    }
                }
            }
            PrimeNode::Create { entity, data } => {
                if entity.is_empty() {
                    errors.push("create: entity cannot be empty".into());
                }
                if data.is_empty() {
                    warnings.push("create: no data provided".into());
                }
            }
            PrimeNode::Read { entity, .. } => {
                if entity.is_empty() {
                    errors.push("read: entity cannot be empty".into());
                }
            }
            PrimeNode::Update { entity, data, .. } => {
                if entity.is_empty() {
                    errors.push("update: entity cannot be empty".into());
                }
                if data.is_empty() {
                    warnings.push("update: no data to update".into());
                }
            }
            PrimeNode::Delete { entity, .. } => {
                if entity.is_empty() {
                    errors.push("delete: entity cannot be empty".into());
                }
            }
            PrimeNode::Sequence(nodes) => {
                if nodes.is_empty() {
                    warnings.push("empty sequence".into());
                }
                for n in nodes {
                    Self::validate_node(n, errors, warnings);
                }
            }
            PrimeNode::Conditional {
                then, else_branch, ..
            } => {
                Self::validate_node(then, errors, warnings);
                if let Some(e) = else_branch {
                    Self::validate_node(e, errors, warnings);
                }
            }
            PrimeNode::Loop { variable, body, .. } => {
                if variable.is_empty() {
                    errors.push("loop: variable name cannot be empty".into());
                }
                Self::validate_node(body, errors, warnings);
            }
            PrimeNode::Endpoint { path, handler, .. } => {
                if !path.starts_with('/') {
                    errors.push(format!("endpoint path must start with '/': {}", path));
                }
                Self::validate_node(handler, errors, warnings);
            }
            PrimeNode::Api { name, endpoints } => {
                if name.is_empty() {
                    errors.push("api name cannot be empty".into());
                }
                if endpoints.is_empty() {
                    warnings.push("api has no endpoints".into());
                }
                for ep in endpoints {
                    Self::validate_node(ep, errors, warnings);
                }
            }
            PrimeNode::Call { target, method, .. } => {
                if target.is_empty() {
                    errors.push("call: target cannot be empty".into());
                }
                if method.is_empty() {
                    errors.push("call: method cannot be empty".into());
                }
            }
            PrimeNode::Store { key, value } => {
                if key.is_empty() {
                    errors.push("store: key cannot be empty".into());
                }
                Self::validate_node(value, errors, warnings);
            }
            PrimeNode::Assign { name, .. } => {
                if name.is_empty() {
                    errors.push("assign: name cannot be empty".into());
                }
            }
            PrimeNode::Return { .. } | PrimeNode::Raw { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_validate_valid() {
        let node = PrimeNode::Entity {
            name: "User".into(),
            fields: vec![Field {
                name: "id".into(),
                type_: PrimeType::Uuid,
                constraints: vec![],
                optional: false,
            }],
        };
        let result = PrimeValidator::validate(&node);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_empty_name() {
        let node = PrimeNode::Entity {
            name: String::new(),
            fields: vec![],
        };
        let result = PrimeValidator::validate(&node);
        assert!(!result.valid);
        assert!(result.errors[0].contains("empty"));
    }

    #[test]
    fn test_validate_endpoint_path() {
        let node = PrimeNode::Endpoint {
            method: HttpMethod::Get,
            path: "users".into(), // missing leading /
            params: vec![],
            handler: Box::new(PrimeNode::Return {
                value: PrimeValue::Null,
            }),
        };
        let result = PrimeValidator::validate(&node);
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_coherence() {
        let api = PrimeNode::Api {
            name: "TestAPI".into(),
            endpoints: vec![PrimeNode::Endpoint {
                method: HttpMethod::Post,
                path: "/items".into(),
                params: vec![],
                handler: Box::new(PrimeNode::Create {
                    entity: "Item".into(),
                    data: HashMap::from([("name".into(), PrimeValue::String("test".into()))]),
                }),
            }],
        };
        let result = PrimeValidator::validate(&api);
        assert!(result.valid);
    }
}
