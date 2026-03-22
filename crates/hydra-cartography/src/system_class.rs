//! System classification types.

use serde::{Deserialize, Serialize};

/// Classification of a digital system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemClass {
    /// A REST API service.
    RestApi,
    /// A GraphQL API service.
    GraphQlApi,
    /// A gRPC service.
    GrpcService,
    /// A WebSocket service.
    WebSocket,
    /// A relational database.
    RelationalDatabase,
    /// A document database.
    DocumentDatabase,
    /// A message queue.
    MessageQueue,
    /// A file system.
    FileSystem,
    /// A command line interface.
    CommandLine,
    /// An embedded device.
    EmbeddedDevice,
    /// A mainframe system.
    Mainframe,
    /// A cloud service.
    CloudService,
    /// A browser environment.
    BrowserEnvironment,
    /// A custom binary protocol.
    CustomBinaryProtocol,
    /// An unknown system type.
    Unknown,
}

impl SystemClass {
    /// Compute similarity between two system classes.
    ///
    /// Returns 1.0 for identical classes, higher values for related classes,
    /// and 0.0 for unrelated classes.
    pub fn similarity(&self, other: &Self) -> f64 {
        if self == other {
            return 1.0;
        }
        let group_a = self.semantic_group();
        let group_b = other.semantic_group();
        if group_a == group_b {
            return 0.7;
        }
        if self.is_cross_related(other) {
            return 0.3;
        }
        0.0
    }

    /// Internal grouping for similarity computation.
    fn semantic_group(&self) -> u8 {
        match self {
            Self::RestApi | Self::GraphQlApi | Self::GrpcService | Self::WebSocket => 0,
            Self::RelationalDatabase | Self::DocumentDatabase => 1,
            Self::MessageQueue => 2,
            Self::FileSystem | Self::CommandLine => 3,
            Self::EmbeddedDevice | Self::Mainframe => 4,
            Self::CloudService | Self::BrowserEnvironment => 5,
            Self::CustomBinaryProtocol => 6,
            Self::Unknown => 7,
        }
    }

    /// Check cross-group relatedness.
    fn is_cross_related(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::RestApi, Self::CloudService)
                | (Self::CloudService, Self::RestApi)
                | (Self::FileSystem, Self::DocumentDatabase)
                | (Self::DocumentDatabase, Self::FileSystem)
                | (Self::CommandLine, Self::EmbeddedDevice)
                | (Self::EmbeddedDevice, Self::CommandLine)
                | (Self::MessageQueue, Self::WebSocket)
                | (Self::WebSocket, Self::MessageQueue)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_similarity() {
        assert!(
            (SystemClass::RestApi.similarity(&SystemClass::RestApi) - 1.0).abs() < f64::EPSILON
        );
    }

    #[test]
    fn same_group_similarity() {
        let sim = SystemClass::RestApi.similarity(&SystemClass::GraphQlApi);
        assert!((sim - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn cross_related_similarity() {
        let sim = SystemClass::RestApi.similarity(&SystemClass::CloudService);
        assert!((sim - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn unrelated_similarity() {
        let sim = SystemClass::RestApi.similarity(&SystemClass::Mainframe);
        assert!(sim.abs() < f64::EPSILON);
    }
}
