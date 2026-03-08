//! PrimeParser — parse LLM JSON output into Prime AST.

use super::ast::*;

/// Parses JSON into Prime AST nodes
pub struct PrimeParser;

impl PrimeParser {
    /// Parse a JSON value into a Prime AST node
    pub fn parse(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let node_type = json
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or("missing 'type' field")?;

        match node_type {
            "entity" => Self::parse_entity(json),
            "create" => Self::parse_create(json),
            "read" => Self::parse_read(json),
            "update" => Self::parse_update(json),
            "delete" => Self::parse_delete(json),
            "sequence" => Self::parse_sequence(json),
            "conditional" => Self::parse_conditional(json),
            "loop" => Self::parse_loop(json),
            "endpoint" => Self::parse_endpoint(json),
            "api" => Self::parse_api(json),
            "call" => Self::parse_call(json),
            "return" => Self::parse_return(json),
            "assign" => Self::parse_assign(json),
            other => Err(format!("unknown node type: {}", other)),
        }
    }

    fn parse_entity(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let name = json
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("entity missing 'name'")?
            .into();
        let fields = json
            .get("fields")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| Self::parse_field(f).ok())
                    .collect()
            })
            .unwrap_or_default();
        Ok(PrimeNode::Entity { name, fields })
    }

    fn parse_field(json: &serde_json::Value) -> Result<Field, String> {
        Ok(Field {
            name: json
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("field missing 'name'")?
                .into(),
            type_: Self::parse_type(json.get("type").and_then(|t| t.as_str()).unwrap_or("any")),
            constraints: json
                .get("constraints")
                .and_then(|c| serde_json::from_value(c.clone()).ok())
                .unwrap_or_default(),
            optional: json
                .get("optional")
                .and_then(|o| o.as_bool())
                .unwrap_or(false),
        })
    }

    fn parse_type(s: &str) -> PrimeType {
        match s {
            "string" => PrimeType::String,
            "integer" | "int" => PrimeType::Integer,
            "float" | "number" => PrimeType::Float,
            "boolean" | "bool" => PrimeType::Boolean,
            "datetime" => PrimeType::DateTime,
            "uuid" => PrimeType::Uuid,
            "map" | "object" => PrimeType::Map,
            _ => PrimeType::Any,
        }
    }

    fn parse_create(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let entity = json
            .get("entity")
            .and_then(|e| e.as_str())
            .ok_or("create missing 'entity'")?
            .into();
        let data = json
            .get("data")
            .and_then(|d| d.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), Self::json_to_prime(v)))
                    .collect()
            })
            .unwrap_or_default();
        Ok(PrimeNode::Create { entity, data })
    }

    fn parse_read(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let entity = json
            .get("entity")
            .and_then(|e| e.as_str())
            .ok_or("read missing 'entity'")?
            .into();
        let filter = json.get("filter").and_then(|f| Self::parse_filter(f).ok());
        let fields = json
            .get("fields")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(PrimeNode::Read {
            entity,
            filter,
            fields,
        })
    }

    fn parse_update(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let entity = json
            .get("entity")
            .and_then(|e| e.as_str())
            .ok_or("update missing 'entity'")?
            .into();
        let filter = Self::parse_filter(json.get("filter").ok_or("update missing 'filter'")?)?;
        let data = json
            .get("data")
            .and_then(|d| d.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), Self::json_to_prime(v)))
                    .collect()
            })
            .unwrap_or_default();
        Ok(PrimeNode::Update {
            entity,
            filter,
            data,
        })
    }

    fn parse_delete(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let entity = json
            .get("entity")
            .and_then(|e| e.as_str())
            .ok_or("delete missing 'entity'")?
            .into();
        let filter = Self::parse_filter(json.get("filter").ok_or("delete missing 'filter'")?)?;
        Ok(PrimeNode::Delete { entity, filter })
    }

    fn parse_filter(json: &serde_json::Value) -> Result<Filter, String> {
        Ok(Filter {
            field: json
                .get("field")
                .and_then(|f| f.as_str())
                .ok_or("filter missing 'field'")?
                .into(),
            op: serde_json::from_value(json.get("op").cloned().unwrap_or(serde_json::json!("eq")))
                .unwrap_or(FilterOp::Eq),
            value: Self::json_to_prime(json.get("value").unwrap_or(&serde_json::Value::Null)),
        })
    }

    fn parse_sequence(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let nodes = json
            .get("nodes")
            .and_then(|n| n.as_array())
            .ok_or("sequence missing 'nodes'")?;
        let parsed: Result<Vec<_>, _> = nodes.iter().map(Self::parse).collect();
        Ok(PrimeNode::Sequence(parsed?))
    }

    fn parse_conditional(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let condition = json
            .get("condition")
            .ok_or("conditional missing 'condition'")?;
        let cond = Condition {
            left: Self::json_to_prime(condition.get("left").unwrap_or(&serde_json::Value::Null)),
            op: serde_json::from_value(
                condition
                    .get("op")
                    .cloned()
                    .unwrap_or(serde_json::json!("eq")),
            )
            .unwrap_or(FilterOp::Eq),
            right: Self::json_to_prime(condition.get("right").unwrap_or(&serde_json::Value::Null)),
        };
        let then = Self::parse(json.get("then").ok_or("conditional missing 'then'")?)?;
        let else_branch = json
            .get("else")
            .map(|e| Self::parse(e))
            .transpose()?
            .map(Box::new);
        Ok(PrimeNode::Conditional {
            condition: cond,
            then: Box::new(then),
            else_branch,
        })
    }

    fn parse_loop(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let variable = json
            .get("variable")
            .and_then(|v| v.as_str())
            .ok_or("loop missing 'variable'")?
            .into();
        let collection =
            Self::json_to_prime(json.get("collection").unwrap_or(&serde_json::Value::Null));
        let body = Self::parse(json.get("body").ok_or("loop missing 'body'")?)?;
        Ok(PrimeNode::Loop {
            variable,
            collection,
            body: Box::new(body),
        })
    }

    fn parse_endpoint(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let method: HttpMethod = serde_json::from_value(
            json.get("method")
                .cloned()
                .unwrap_or(serde_json::json!("GET")),
        )
        .unwrap_or(HttpMethod::Get);
        let path = json
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or("endpoint missing 'path'")?
            .into();
        let params = json
            .get("params")
            .and_then(|p| p.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| Self::parse_field(f).ok())
                    .collect()
            })
            .unwrap_or_default();
        let handler = Self::parse(json.get("handler").ok_or("endpoint missing 'handler'")?)?;
        Ok(PrimeNode::Endpoint {
            method,
            path,
            params,
            handler: Box::new(handler),
        })
    }

    fn parse_api(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let name = json
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("api missing 'name'")?
            .into();
        let endpoints = json
            .get("endpoints")
            .and_then(|e| e.as_array())
            .ok_or("api missing 'endpoints'")?;
        let parsed: Result<Vec<_>, _> = endpoints.iter().map(Self::parse).collect();
        Ok(PrimeNode::Api {
            name,
            endpoints: parsed?,
        })
    }

    fn parse_call(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let target = json
            .get("target")
            .and_then(|t| t.as_str())
            .ok_or("call missing 'target'")?
            .into();
        let method = json
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or("call missing 'method'")?
            .into();
        let args = json
            .get("args")
            .and_then(|a| a.as_array())
            .map(|arr| arr.iter().map(Self::json_to_prime).collect())
            .unwrap_or_default();
        Ok(PrimeNode::Call {
            target,
            method,
            args,
        })
    }

    fn parse_return(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let value = Self::json_to_prime(json.get("value").unwrap_or(&serde_json::Value::Null));
        Ok(PrimeNode::Return { value })
    }

    fn parse_assign(json: &serde_json::Value) -> Result<PrimeNode, String> {
        let name = json
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("assign missing 'name'")?
            .into();
        let value = Self::json_to_prime(json.get("value").unwrap_or(&serde_json::Value::Null));
        Ok(PrimeNode::Assign { name, value })
    }

    /// Convert a JSON value to a PrimeValue
    pub fn json_to_prime(json: &serde_json::Value) -> PrimeValue {
        match json {
            serde_json::Value::Null => PrimeValue::Null,
            serde_json::Value::Bool(b) => PrimeValue::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    PrimeValue::Int(i)
                } else {
                    PrimeValue::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => {
                if s.starts_with('$') {
                    PrimeValue::Variable(s[1..].to_string())
                } else {
                    PrimeValue::String(s.clone())
                }
            }
            serde_json::Value::Array(arr) => {
                PrimeValue::Array(arr.iter().map(Self::json_to_prime).collect())
            }
            serde_json::Value::Object(obj) => PrimeValue::Map(
                obj.iter()
                    .map(|(k, v)| (k.clone(), Self::json_to_prime(v)))
                    .collect(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entity() {
        let json = serde_json::json!({
            "type": "entity",
            "name": "User",
            "fields": [
                { "name": "id", "type": "uuid" },
                { "name": "name", "type": "string" },
            ]
        });
        let node = PrimeParser::parse(&json).unwrap();
        assert_eq!(node.type_name(), "entity");
    }

    #[test]
    fn test_parse_crud() {
        let json = serde_json::json!({
            "type": "create",
            "entity": "User",
            "data": { "name": "Alice", "age": 30 }
        });
        let node = PrimeParser::parse(&json).unwrap();
        assert_eq!(node.type_name(), "create");
    }

    #[test]
    fn test_parse_api() {
        let json = serde_json::json!({
            "type": "api",
            "name": "UserAPI",
            "endpoints": [
                {
                    "type": "endpoint",
                    "method": "GET",
                    "path": "/users",
                    "handler": { "type": "read", "entity": "User" }
                }
            ]
        });
        let node = PrimeParser::parse(&json).unwrap();
        assert_eq!(node.type_name(), "api");
    }
}
