//! ProtocolAdapter — adapts an intent to a specific protocol.
//! The adapter takes a generic intent and produces a protocol-specific request.
//! All adapters receipt every protocol event (constitutional).

use crate::errors::ProtocolError;
use crate::family::ProtocolFamily;
use serde::{Deserialize, Serialize};

/// A protocol-adapted request — ready to send to a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptedRequest {
    pub protocol: String,
    pub target: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub receipt_id: String,
}

/// The result of adapting an intent to a protocol.
#[derive(Debug, Clone)]
pub struct AdaptationResult {
    pub request: AdaptedRequest,
    pub confidence: f64,
}

/// Adapt a generic intent to the appropriate protocol format.
pub fn adapt_to_protocol(
    target: &str,
    intent: &str,
    payload: Option<&str>,
    family: &ProtocolFamily,
) -> Result<AdaptationResult, ProtocolError> {
    let receipt_id = uuid::Uuid::new_v4().to_string();

    let (method, headers, body, confidence) = match family {
        ProtocolFamily::RestHttp => {
            let method = if payload.is_some() { "POST" } else { "GET" };
            let headers = vec![
                ("Content-Type".into(), "application/json".into()),
                ("X-Hydra-Receipt".into(), receipt_id.clone()),
            ];
            (method.to_string(), headers, payload.map(String::from), 0.90)
        }
        ProtocolFamily::GraphQL => {
            let body = serde_json::json!({
                "query": intent,
                "variables": payload.unwrap_or("{}"),
            })
            .to_string();
            let headers = vec![
                ("Content-Type".into(), "application/json".into()),
                ("X-Hydra-Receipt".into(), receipt_id.clone()),
            ];
            ("POST".to_string(), headers, Some(body), 0.88)
        }
        ProtocolFamily::CobolJcl => {
            let jcl = format!(
                "//HYDRAJOB JOB\n//STEP1 EXEC {}\n{}\n/*",
                intent,
                payload.unwrap_or(""),
            );
            (
                "SUBMIT".to_string(),
                vec![("X-Hydra-Receipt".into(), receipt_id.clone())],
                Some(jcl),
                0.75,
            )
        }
        ProtocolFamily::Grpc => {
            let headers = vec![
                ("content-type".into(), "application/grpc".into()),
                ("x-hydra-receipt".into(), receipt_id.clone()),
            ];
            ("POST".to_string(), headers, payload.map(String::from), 0.85)
        }
        ProtocolFamily::Mqtt => (
            "PUBLISH".to_string(),
            vec![("x-hydra-receipt".into(), receipt_id.clone())],
            Some(
                serde_json::json!({
                    "topic": intent,
                    "payload": payload.unwrap_or(""),
                })
                .to_string(),
            ),
            0.87,
        ),
        ProtocolFamily::Unknown => {
            return Err(ProtocolError::ProtocolNotSupported {
                protocol: "unknown".into(),
            });
        }
        _ => {
            // Generic adapter for other known protocols
            (
                "SEND".to_string(),
                vec![("x-hydra-receipt".into(), receipt_id.clone())],
                payload.map(String::from),
                0.70,
            )
        }
    };

    Ok(AdaptationResult {
        request: AdaptedRequest {
            protocol: family.label(),
            target: target.to_string(),
            method,
            headers,
            body,
            receipt_id,
        },
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_get_adapter() {
        let r = adapt_to_protocol(
            "https://api.example.com/status",
            "GET /status",
            None,
            &ProtocolFamily::RestHttp,
        )
        .expect("adaptation failed");
        assert_eq!(r.request.method, "GET");
        assert!(r
            .request
            .headers
            .iter()
            .any(|(k, _)| k == "X-Hydra-Receipt"));
        assert!(!r.request.receipt_id.is_empty());
    }

    #[test]
    fn rest_post_with_payload() {
        let r = adapt_to_protocol(
            "https://api.example.com/deploy",
            "POST /deploy",
            Some(r#"{"env":"staging"}"#),
            &ProtocolFamily::RestHttp,
        )
        .expect("adaptation failed");
        assert_eq!(r.request.method, "POST");
        assert!(r.request.body.is_some());
    }

    #[test]
    fn graphql_wrapped() {
        let r = adapt_to_protocol(
            "https://api.example.com/graphql",
            "{ deployment { status } }",
            None,
            &ProtocolFamily::GraphQL,
        )
        .expect("adaptation failed");
        assert!(r.request.body.as_ref().expect("body").contains("query"));
    }

    #[test]
    fn cobol_jcl_format() {
        let r = adapt_to_protocol(
            "mainframe.corp:23",
            "BATCH_MIGRATE",
            Some("//SOURCEPGM"),
            &ProtocolFamily::CobolJcl,
        )
        .expect("adaptation failed");
        assert!(r.request.body.as_ref().expect("body").contains("HYDRAJOB"));
        assert!(r
            .request
            .body
            .as_ref()
            .expect("body")
            .contains("BATCH_MIGRATE"));
    }

    #[test]
    fn unknown_protocol_returns_error() {
        let r = adapt_to_protocol("unknown://target", "intent", None, &ProtocolFamily::Unknown);
        assert!(r.is_err());
    }

    #[test]
    fn every_adaptation_has_receipt() {
        let families = vec![
            ProtocolFamily::RestHttp,
            ProtocolFamily::GraphQL,
            ProtocolFamily::Grpc,
            ProtocolFamily::Mqtt,
        ];
        for family in families {
            let r =
                adapt_to_protocol("target", "intent", None, &family).expect("adaptation failed");
            assert!(
                !r.request.receipt_id.is_empty(),
                "No receipt for {:?}",
                family
            );
        }
    }
}
