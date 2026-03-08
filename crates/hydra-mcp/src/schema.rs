//! SchemaValidator — validate inputs/outputs against JSON Schema.

use serde::{Deserialize, Serialize};

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn fail(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }
}

/// JSON Schema validator for MCP tool inputs/outputs
pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate a value against a JSON Schema
    pub fn validate(value: &serde_json::Value, schema: &serde_json::Value) -> ValidationResult {
        let mut errors = Vec::new();

        // Check type
        if let Some(expected_type) = schema.get("type").and_then(|t| t.as_str()) {
            if !Self::check_type(value, expected_type) {
                errors.push(format!(
                    "expected type '{}', got '{}'",
                    expected_type,
                    Self::value_type(value)
                ));
                return ValidationResult::fail(errors);
            }
        }

        // For objects, check properties and required fields
        if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
            Self::validate_object(value, schema, &mut errors);
        }

        // For arrays, check items
        if schema.get("type").and_then(|t| t.as_str()) == Some("array") {
            Self::validate_array(value, schema, &mut errors);
        }

        // Check enum constraint
        if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
            if !enum_values.contains(value) {
                errors.push(format!("value not in enum: {:?}", enum_values));
            }
        }

        // Check string constraints
        if value.is_string() {
            Self::validate_string(value, schema, &mut errors);
        }

        // Check number constraints
        if value.is_number() {
            Self::validate_number(value, schema, &mut errors);
        }

        if errors.is_empty() {
            ValidationResult::ok()
        } else {
            ValidationResult::fail(errors)
        }
    }

    fn check_type(value: &serde_json::Value, expected: &str) -> bool {
        match expected {
            "string" => value.is_string(),
            "number" | "integer" => value.is_number(),
            "boolean" => value.is_boolean(),
            "object" => value.is_object(),
            "array" => value.is_array(),
            "null" => value.is_null(),
            _ => true,
        }
    }

    fn value_type(value: &serde_json::Value) -> &'static str {
        match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }

    fn validate_object(
        value: &serde_json::Value,
        schema: &serde_json::Value,
        errors: &mut Vec<String>,
    ) {
        let obj = match value.as_object() {
            Some(o) => o,
            None => return,
        };

        // Check required fields
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            for req in required {
                if let Some(field_name) = req.as_str() {
                    if !obj.contains_key(field_name) {
                        errors.push(format!("missing required field: '{}'", field_name));
                    }
                }
            }
        }

        // Validate properties against their schemas
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            for (key, prop_schema) in properties {
                if let Some(prop_value) = obj.get(key) {
                    let prop_result = Self::validate(prop_value, prop_schema);
                    for err in prop_result.errors {
                        errors.push(format!("{}: {}", key, err));
                    }
                }
            }
        }
    }

    fn validate_array(
        value: &serde_json::Value,
        schema: &serde_json::Value,
        errors: &mut Vec<String>,
    ) {
        let arr = match value.as_array() {
            Some(a) => a,
            None => return,
        };

        // Check minItems
        if let Some(min) = schema.get("minItems").and_then(|m| m.as_u64()) {
            if (arr.len() as u64) < min {
                errors.push(format!("array has {} items, minimum is {}", arr.len(), min));
            }
        }

        // Check maxItems
        if let Some(max) = schema.get("maxItems").and_then(|m| m.as_u64()) {
            if (arr.len() as u64) > max {
                errors.push(format!("array has {} items, maximum is {}", arr.len(), max));
            }
        }

        // Validate items against item schema
        if let Some(items_schema) = schema.get("items") {
            for (i, item) in arr.iter().enumerate() {
                let item_result = Self::validate(item, items_schema);
                for err in item_result.errors {
                    errors.push(format!("[{}]: {}", i, err));
                }
            }
        }
    }

    fn validate_string(
        value: &serde_json::Value,
        schema: &serde_json::Value,
        errors: &mut Vec<String>,
    ) {
        let s = match value.as_str() {
            Some(s) => s,
            None => return,
        };

        if let Some(min_len) = schema.get("minLength").and_then(|m| m.as_u64()) {
            if (s.len() as u64) < min_len {
                errors.push(format!(
                    "string length {} is less than minimum {}",
                    s.len(),
                    min_len
                ));
            }
        }

        if let Some(max_len) = schema.get("maxLength").and_then(|m| m.as_u64()) {
            if (s.len() as u64) > max_len {
                errors.push(format!(
                    "string length {} exceeds maximum {}",
                    s.len(),
                    max_len
                ));
            }
        }
    }

    fn validate_number(
        value: &serde_json::Value,
        schema: &serde_json::Value,
        errors: &mut Vec<String>,
    ) {
        let num = match value.as_f64() {
            Some(n) => n,
            None => return,
        };

        if let Some(min) = schema.get("minimum").and_then(|m| m.as_f64()) {
            if num < min {
                errors.push(format!("value {} is less than minimum {}", num, min));
            }
        }

        if let Some(max) = schema.get("maximum").and_then(|m| m.as_f64()) {
            if num > max {
                errors.push(format!("value {} exceeds maximum {}", num, max));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_object() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "number" }
            },
            "required": ["name"]
        });

        let value = serde_json::json!({"name": "test", "age": 25});
        let result = SchemaValidator::validate(&value, &schema);
        assert!(result.valid);
    }

    #[test]
    fn test_missing_required_field() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });

        let value = serde_json::json!({});
        let result = SchemaValidator::validate(&value, &schema);
        assert!(!result.valid);
        assert!(result.errors[0].contains("missing required field"));
    }

    #[test]
    fn test_type_mismatch() {
        let schema = serde_json::json!({"type": "string"});
        let value = serde_json::json!(42);
        let result = SchemaValidator::validate(&value, &schema);
        assert!(!result.valid);
    }

    #[test]
    fn test_string_constraints() {
        let schema = serde_json::json!({
            "type": "string",
            "minLength": 3,
            "maxLength": 10
        });

        assert!(SchemaValidator::validate(&serde_json::json!("hello"), &schema).valid);
        assert!(!SchemaValidator::validate(&serde_json::json!("hi"), &schema).valid);
        assert!(!SchemaValidator::validate(&serde_json::json!("this is too long"), &schema).valid);
    }

    #[test]
    fn test_array_validation() {
        let schema = serde_json::json!({
            "type": "array",
            "items": { "type": "number" },
            "minItems": 1,
            "maxItems": 3
        });

        assert!(SchemaValidator::validate(&serde_json::json!([1, 2]), &schema).valid);
        assert!(!SchemaValidator::validate(&serde_json::json!([]), &schema).valid);
        assert!(!SchemaValidator::validate(&serde_json::json!([1, 2, 3, 4]), &schema).valid);
    }

    #[test]
    fn test_nested_validation() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "port": { "type": "number", "minimum": 1, "maximum": 65535 }
                    },
                    "required": ["port"]
                }
            },
            "required": ["config"]
        });

        let valid = serde_json::json!({"config": {"port": 3000}});
        assert!(SchemaValidator::validate(&valid, &schema).valid);

        let invalid_port = serde_json::json!({"config": {"port": 0}});
        assert!(!SchemaValidator::validate(&invalid_port, &schema).valid);
    }
}
