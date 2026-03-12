use serde::{Deserialize, Serialize};

/// Standard JSON-RPC 2.0 error codes
pub struct RpcErrorCodes;

impl RpcErrorCodes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    // Hydra-specific
    pub const SISTER_UNAVAILABLE: i32 = -32000;
    pub const RUN_FAILED: i32 = -32001;
    pub const APPROVAL_REQUIRED: i32 = -32002;
    pub const CAPABILITY_DENIED: i32 = -32003;
    pub const RESOURCE_EXHAUSTED: i32 = -32004;
    pub const TIMEOUT: i32 = -32005;
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn is_valid(&self) -> bool {
        self.jsonrpc == "2.0" && !self.method.is_empty()
    }
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: serde_json::Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }

    pub fn is_success(&self) -> bool {
        self.result.is_some() && self.error.is_none()
    }
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(RpcErrorCodes::PARSE_ERROR, -32700);
        assert_eq!(RpcErrorCodes::INVALID_REQUEST, -32600);
        assert_eq!(RpcErrorCodes::METHOD_NOT_FOUND, -32601);
        assert_eq!(RpcErrorCodes::INVALID_PARAMS, -32602);
        assert_eq!(RpcErrorCodes::INTERNAL_ERROR, -32603);
        assert_eq!(RpcErrorCodes::SISTER_UNAVAILABLE, -32000);
    }

    #[test]
    fn test_request_valid() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(1),
            method: "test_method".into(),
            params: serde_json::json!({}),
        };
        assert!(req.is_valid());
    }

    #[test]
    fn test_request_invalid_version() {
        let req = JsonRpcRequest {
            jsonrpc: "1.0".into(),
            id: serde_json::json!(1),
            method: "test".into(),
            params: serde_json::json!({}),
        };
        assert!(!req.is_valid());
    }

    #[test]
    fn test_request_empty_method() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(1),
            method: "".into(),
            params: serde_json::json!({}),
        };
        assert!(!req.is_valid());
    }

    #[test]
    fn test_success_response() {
        let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!("ok"));
        assert!(resp.is_success());
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.result, Some(serde_json::json!("ok")));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_error_response() {
        let resp = JsonRpcResponse::error(serde_json::json!(1), -32600, "bad request");
        assert!(!resp.is_success());
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32600);
        assert_eq!(err.message, "bad request");
    }

    #[test]
    fn test_request_serde_roundtrip() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::json!("abc"),
            method: "tools/call".into(),
            params: serde_json::json!({"tool": "memory_add"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let restored: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.method, "tools/call");
        assert_eq!(restored.id, serde_json::json!("abc"));
    }

    #[test]
    fn test_response_serde_roundtrip() {
        let resp = JsonRpcResponse::success(serde_json::json!(42), serde_json::json!({"data": true}));
        let json = serde_json::to_string(&resp).unwrap();
        let restored: JsonRpcResponse = serde_json::from_str(&json).unwrap();
        assert!(restored.is_success());
    }

    #[test]
    fn test_request_default_params() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert!(req.is_valid());
        assert_eq!(req.params, serde_json::Value::Null);
    }
}
