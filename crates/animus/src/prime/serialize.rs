//! PrimeSerializer — Prime AST ↔ JSON serialization.

use super::ast::PrimeNode;

/// Handles Prime AST serialization
pub struct PrimeSerializer;

impl PrimeSerializer {
    /// Serialize a Prime AST to JSON
    pub fn to_json(node: &PrimeNode) -> Result<String, String> {
        serde_json::to_string_pretty(node).map_err(|e| format!("serialization error: {}", e))
    }

    /// Serialize to compact JSON
    pub fn to_json_compact(node: &PrimeNode) -> Result<String, String> {
        serde_json::to_string(node).map_err(|e| format!("serialization error: {}", e))
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<PrimeNode, String> {
        serde_json::from_str(json).map_err(|e| format!("deserialization error: {}", e))
    }

    /// Serialize to serde_json::Value
    pub fn to_value(node: &PrimeNode) -> Result<serde_json::Value, String> {
        serde_json::to_value(node).map_err(|e| format!("to_value error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prime::ast::*;

    #[test]
    fn test_round_trip() {
        let node = PrimeNode::Return {
            value: PrimeValue::String("hello".into()),
        };
        let json = PrimeSerializer::to_json(&node).unwrap();
        let restored = PrimeSerializer::from_json(&json).unwrap();
        assert_eq!(restored.type_name(), "return");
    }
}
