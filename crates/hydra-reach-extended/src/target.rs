//! ReachTarget — what we want to connect to and what we know about it.

use hydra_protocol::ProtocolFamily;
use serde::{Deserialize, Serialize};

/// Classification of a reach target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TargetClass {
    /// Public HTTP/HTTPS API.
    PublicApi,
    /// Private internal service.
    InternalService,
    /// Source code repository.
    Repository { host: String },
    /// Database endpoint.
    Database { engine: String },
    /// Cloud service.
    CloudService { provider: String, service: String },
    /// Legacy mainframe or COBOL system.
    LegacyMainframe,
    /// IoT or embedded device.
    IoTDevice,
    /// Message queue or event stream.
    MessageQueue { broker_type: String },
    /// Unknown — cartography will classify on first contact.
    Unknown,
}

impl TargetClass {
    pub fn label(&self) -> String {
        match self {
            Self::PublicApi => "public-api".into(),
            Self::InternalService => "internal-service".into(),
            Self::Repository { host } => format!("repo:{}", host),
            Self::Database { engine } => format!("db:{}", engine),
            Self::CloudService { provider, service } => format!("cloud:{}:{}", provider, service),
            Self::LegacyMainframe => "mainframe".into(),
            Self::IoTDevice => "iot".into(),
            Self::MessageQueue { broker_type } => format!("mq:{}", broker_type),
            Self::Unknown => "unknown".into(),
        }
    }

    /// Infer target class from URL/address hints.
    pub fn infer(target: &str) -> Self {
        let lower = target.to_lowercase();

        if lower.contains("github.com")
            || lower.contains("gitlab.com")
            || lower.contains("bitbucket.org")
        {
            return Self::Repository { host: "git".into() };
        }
        if lower.contains("postgres")
            || lower.contains(":5432")
            || lower.contains("mysql")
            || lower.contains(":3306")
            || lower.contains("oracle")
            || lower.contains(":1521")
            || lower.contains("mongodb")
            || lower.contains(":27017")
        {
            let engine = if lower.contains("postgres") {
                "postgresql"
            } else if lower.contains("mysql") {
                "mysql"
            } else if lower.contains("oracle") {
                "oracle"
            } else if lower.contains("mongodb") {
                "mongodb"
            } else {
                "unknown"
            };
            return Self::Database {
                engine: engine.into(),
            };
        }
        if lower.contains("amazonaws.com")
            || lower.contains("azure.com")
            || lower.contains("googleapis.com")
        {
            let provider = if lower.contains("amazonaws") {
                "aws"
            } else if lower.contains("azure") {
                "azure"
            } else {
                "gcp"
            };
            return Self::CloudService {
                provider: provider.into(),
                service: "api".into(),
            };
        }
        if lower.contains("mainframe") || lower.contains("jcl") || lower.contains("cobol") {
            return Self::LegacyMainframe;
        }
        if lower.contains("kafka")
            || lower.contains(":9092")
            || lower.contains("rabbitmq")
            || lower.contains("amqp")
        {
            return Self::MessageQueue {
                broker_type: "kafka-or-amqp".into(),
            };
        }
        if lower.starts_with("http") {
            return Self::PublicApi;
        }
        Self::Unknown
    }
}

/// A reach target — what Hydra wants to connect to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReachTarget {
    pub id: String,
    pub address: String,
    pub class: TargetClass,
    pub protocol: Option<ProtocolFamily>,
    pub requires_auth: bool,
    pub notes: Option<String>,
}

impl ReachTarget {
    pub fn new(address: impl Into<String>) -> Self {
        let addr = address.into();
        let class = TargetClass::infer(&addr);
        let proto = hydra_protocol::infer_from_target(&addr);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            address: addr,
            class: class.clone(),
            protocol: Some(proto.likely_family),
            requires_auth: matches!(
                class,
                TargetClass::Database { .. }
                    | TargetClass::InternalService
                    | TargetClass::LegacyMainframe
                    | TargetClass::CloudService { .. }
            ),
            notes: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_classified_as_repo() {
        let t = ReachTarget::new("https://github.com/org/repo.git");
        assert!(matches!(t.class, TargetClass::Repository { .. }));
    }

    #[test]
    fn postgres_classified_as_database() {
        let t = ReachTarget::new("postgres://user:pass@db.internal:5432/hydra");
        assert!(matches!(t.class, TargetClass::Database { .. }));
    }

    #[test]
    fn aws_classified_as_cloud() {
        let t = ReachTarget::new("https://s3.amazonaws.com/bucket/file");
        assert!(matches!(t.class, TargetClass::CloudService { .. }));
    }

    #[test]
    fn mainframe_classified_correctly() {
        let t = ReachTarget::new("mainframe.corp.internal/jcl");
        assert_eq!(t.class, TargetClass::LegacyMainframe);
    }

    #[test]
    fn http_api_classified() {
        let t = ReachTarget::new("https://api.example.com/v1/data");
        assert!(matches!(t.class, TargetClass::PublicApi));
    }
}
