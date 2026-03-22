//! Execution mode types.

use serde::{Deserialize, Serialize};

/// An execution mode for running capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Execute as a native binary.
    NativeBinary,
    /// Execute in a WASM runtime.
    WasmRuntime,
    /// Execute in a container.
    ContainerExec,
    /// Execute via a remote shell.
    RemoteShell,
    /// Execute through an API gateway.
    ApiGateway,
    /// Execute via a legacy interface with a specific protocol.
    LegacyInterface {
        /// The legacy protocol name.
        protocol: String,
    },
}

impl ExecutionMode {
    /// Return the priority of this execution mode (lower = preferred).
    pub fn priority(&self) -> u32 {
        match self {
            Self::NativeBinary => 0,
            Self::WasmRuntime => 1,
            Self::ContainerExec => 2,
            Self::RemoteShell => 3,
            Self::ApiGateway => 4,
            Self::LegacyInterface { .. } => 5,
        }
    }

    /// Return a human-readable label for this mode.
    pub fn label(&self) -> &str {
        match self {
            Self::NativeBinary => "native-binary",
            Self::WasmRuntime => "wasm-runtime",
            Self::ContainerExec => "container-exec",
            Self::RemoteShell => "remote-shell",
            Self::ApiGateway => "api-gateway",
            Self::LegacyInterface { .. } => "legacy-interface",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_has_highest_priority() {
        assert_eq!(ExecutionMode::NativeBinary.priority(), 0);
    }

    #[test]
    fn priority_ordering() {
        assert!(ExecutionMode::NativeBinary.priority() < ExecutionMode::WasmRuntime.priority());
        assert!(ExecutionMode::WasmRuntime.priority() < ExecutionMode::ContainerExec.priority());
        assert!(ExecutionMode::ContainerExec.priority() < ExecutionMode::RemoteShell.priority());
        assert!(ExecutionMode::RemoteShell.priority() < ExecutionMode::ApiGateway.priority());
    }

    #[test]
    fn legacy_has_lowest_priority() {
        let legacy = ExecutionMode::LegacyInterface {
            protocol: "ftp".to_string(),
        };
        assert!(legacy.priority() > ExecutionMode::ApiGateway.priority());
    }
}
