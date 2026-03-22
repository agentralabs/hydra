//! ProtocolFamily — known protocol types.
//! Extended by skills and cartography.

use serde::{Deserialize, Serialize};

/// A protocol family.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolFamily {
    // Web APIs
    RestHttp,
    GraphQL,
    Grpc,
    WebSocket,
    // Messaging
    Mqtt,
    Amqp,
    Kafka,
    // Financial
    Fix,
    Swift,
    // Industrial / embedded
    Modbus,
    CanBus,
    // Legacy
    CobolJcl,
    Mainframe,
    // Database
    SqlTcp,
    // Custom
    CustomBinary { identifier: String },
    Unknown,
}

impl ProtocolFamily {
    /// Return a human-readable label for this protocol family.
    pub fn label(&self) -> String {
        match self {
            Self::RestHttp => "rest-http".into(),
            Self::GraphQL => "graphql".into(),
            Self::Grpc => "grpc".into(),
            Self::WebSocket => "websocket".into(),
            Self::Mqtt => "mqtt".into(),
            Self::Amqp => "amqp".into(),
            Self::Kafka => "kafka".into(),
            Self::Fix => "fix".into(),
            Self::Swift => "swift".into(),
            Self::Modbus => "modbus".into(),
            Self::CanBus => "canbus".into(),
            Self::CobolJcl => "cobol-jcl".into(),
            Self::Mainframe => "mainframe".into(),
            Self::SqlTcp => "sql-tcp".into(),
            Self::CustomBinary { identifier } => format!("custom:{identifier}"),
            Self::Unknown => "unknown".into(),
        }
    }

    /// Is this a text-based, human-readable protocol?
    pub fn is_text_based(&self) -> bool {
        matches!(
            self,
            Self::RestHttp | Self::GraphQL | Self::CobolJcl | Self::Fix
        )
    }

    /// Does this protocol require authentication by default?
    pub fn requires_auth(&self) -> bool {
        matches!(
            self,
            Self::RestHttp
                | Self::Grpc
                | Self::Kafka
                | Self::Fix
                | Self::Swift
                | Self::Mainframe
                | Self::SqlTcp
        )
    }

    /// Return true if this protocol family supports streaming.
    pub fn is_streaming(&self) -> bool {
        matches!(
            self,
            Self::WebSocket | Self::Grpc | Self::Mqtt | Self::Amqp | Self::Kafka
        )
    }

    /// Return true if this protocol family is considered legacy.
    pub fn is_legacy(&self) -> bool {
        matches!(
            self,
            Self::CobolJcl | Self::Modbus | Self::Fix | Self::Mainframe
        )
    }
}

/// A hint about what a target endpoint speaks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolHint {
    pub target: String,
    pub likely_family: ProtocolFamily,
    pub confidence: f64,
    pub signals: Vec<String>,
}

/// Infer protocol from target URL or address hints.
pub fn infer_from_target(target: &str) -> ProtocolHint {
    let lower = target.to_lowercase();

    let (family, confidence, signals) = if lower.contains("/graphql") {
        (
            ProtocolFamily::GraphQL,
            0.92,
            vec!["graphql in path".into()],
        )
    } else if lower.starts_with("wss://") || lower.starts_with("ws://") {
        (
            ProtocolFamily::WebSocket,
            0.97,
            vec!["websocket scheme".into()],
        )
    } else if lower.starts_with("mqtt://") {
        (ProtocolFamily::Mqtt, 0.97, vec!["mqtt scheme".into()])
    } else if lower.starts_with("amqp://") || lower.starts_with("amqps://") {
        (ProtocolFamily::Amqp, 0.97, vec!["amqp scheme".into()])
    } else if lower.contains(":9092") || lower.contains("kafka") {
        (
            ProtocolFamily::Kafka,
            0.85,
            vec!["kafka port/hostname".into()],
        )
    } else if lower.contains(":50051") || lower.ends_with(".proto") {
        (ProtocolFamily::Grpc, 0.88, vec!["grpc port".into()])
    } else if lower.starts_with("http://") || lower.starts_with("https://") {
        (ProtocolFamily::RestHttp, 0.80, vec!["http scheme".into()])
    } else if lower.contains("mainframe") || lower.contains("jcl") || lower.contains("cobol") {
        (
            ProtocolFamily::CobolJcl,
            0.75,
            vec!["mainframe/cobol keyword".into()],
        )
    } else if lower.contains(":1521")
        || lower.contains("oracle")
        || lower.contains(":5432")
        || lower.contains("postgres")
    {
        (ProtocolFamily::SqlTcp, 0.82, vec!["database port".into()])
    } else {
        (ProtocolFamily::Unknown, 0.2, vec![])
    };

    ProtocolHint {
        target: target.to_string(),
        likely_family: family,
        confidence,
        signals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_inferred() {
        let h = infer_from_target("wss://api.example.com/stream");
        assert_eq!(h.likely_family, ProtocolFamily::WebSocket);
        assert!(h.confidence > 0.9);
    }

    #[test]
    fn rest_inferred() {
        let h = infer_from_target("https://api.example.com/v1/data");
        assert_eq!(h.likely_family, ProtocolFamily::RestHttp);
    }

    #[test]
    fn graphql_inferred() {
        let h = infer_from_target("https://api.example.com/graphql");
        assert_eq!(h.likely_family, ProtocolFamily::GraphQL);
    }

    #[test]
    fn cobol_inferred() {
        let h = infer_from_target("mainframe.corp.internal/jcl/batch");
        assert_eq!(h.likely_family, ProtocolFamily::CobolJcl);
    }

    #[test]
    fn kafka_inferred() {
        let h = infer_from_target("kafka-broker.internal:9092");
        assert_eq!(h.likely_family, ProtocolFamily::Kafka);
    }

    #[test]
    fn label_non_empty() {
        for f in [
            ProtocolFamily::RestHttp,
            ProtocolFamily::Fix,
            ProtocolFamily::CobolJcl,
            ProtocolFamily::Unknown,
        ] {
            assert!(!f.label().is_empty());
        }
    }

    #[test]
    fn text_based_protocols() {
        assert!(ProtocolFamily::RestHttp.is_text_based());
        assert!(ProtocolFamily::GraphQL.is_text_based());
        assert!(!ProtocolFamily::Grpc.is_text_based());
    }

    #[test]
    fn auth_required_protocols() {
        assert!(ProtocolFamily::RestHttp.requires_auth());
        assert!(ProtocolFamily::Swift.requires_auth());
        assert!(!ProtocolFamily::Mqtt.requires_auth());
    }

    #[test]
    fn streaming_protocols() {
        assert!(ProtocolFamily::WebSocket.is_streaming());
        assert!(ProtocolFamily::Kafka.is_streaming());
        assert!(!ProtocolFamily::RestHttp.is_streaming());
    }

    #[test]
    fn legacy_protocols() {
        assert!(ProtocolFamily::CobolJcl.is_legacy());
        assert!(ProtocolFamily::Modbus.is_legacy());
        assert!(!ProtocolFamily::RestHttp.is_legacy());
    }

    #[test]
    fn hint_includes_target() {
        let h = infer_from_target("https://example.com/api");
        assert_eq!(h.target, "https://example.com/api");
    }
}
