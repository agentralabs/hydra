//! ReachServer — the local authenticated server.
//! Any device that can speak HTTP/WebSocket can connect.
//! Authentication required. Your keys only.

use crate::{
    constants::MAX_DEVICE_CONNECTIONS,
    device::{DeviceCapabilities, DeviceProfile},
    errors::ReachError,
    session::{DeviceSession, SessionState},
};
use std::collections::HashMap;

/// The reach server state.
pub struct ReachServer {
    /// The port the server listens on.
    pub port: u16,
    /// Registered devices, keyed by device ID.
    devices: HashMap<String, DeviceProfile>,
    /// Active sessions, keyed by session ID.
    sessions: HashMap<String, DeviceSession>,
    /// Whether the server is running (will be used for lifecycle management).
    #[allow(dead_code)]
    running: bool,
}

impl ReachServer {
    /// Create a new reach server on the given port.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            devices: HashMap::new(),
            sessions: HashMap::new(),
            running: false,
        }
    }

    /// Register a new device. Returns the device ID.
    pub fn register_device(
        &mut self,
        name: impl Into<String>,
        capabilities: DeviceCapabilities,
        auth_token: impl Into<String>,
    ) -> Result<String, ReachError> {
        if self.devices.len() >= MAX_DEVICE_CONNECTIONS {
            return Err(ReachError::MaxConnectionsReached {
                max: MAX_DEVICE_CONNECTIONS,
            });
        }

        let id = uuid::Uuid::new_v4().to_string();
        let device = DeviceProfile::new(id.clone(), name, capabilities, auth_token);
        self.devices.insert(id.clone(), device);
        Ok(id)
    }

    /// Authenticate a device and create a session.
    pub fn connect(
        &mut self,
        device_id: &str,
        auth_token: &str,
    ) -> Result<String, ReachError> {
        let device =
            self.devices
                .get_mut(device_id)
                .ok_or(ReachError::DeviceNotConnected {
                    device_id: device_id.to_string(),
                })?;

        if device.auth_token != auth_token {
            return Err(ReachError::AuthenticationFailed {
                device_id: device_id.to_string(),
            });
        }

        device.record_connection();
        let output_mode = device.output_mode.clone();
        let session = DeviceSession::new(device_id, output_mode);
        let session_id = session.id.clone();
        self.sessions.insert(session_id.clone(), session);

        Ok(session_id)
    }

    /// Disconnect a device session.
    pub fn disconnect(&mut self, session_id: &str) {
        if let Some(s) = self.sessions.get_mut(session_id) {
            s.state = SessionState::Disconnected;
        }
    }

    /// Return the total number of registered devices.
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Return the number of active sessions.
    pub fn active_session_count(&self) -> usize {
        self.sessions.values().filter(|s| s.is_active()).count()
    }

    /// Get a reference to a device profile.
    pub fn get_device(&self, id: &str) -> Option<&DeviceProfile> {
        self.devices.get(id)
    }

    /// Get a reference to a session.
    pub fn get_session(&self, id: &str) -> Option<&DeviceSession> {
        self.sessions.get(id)
    }

    /// Get a mutable reference to a session.
    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut DeviceSession> {
        self.sessions.get_mut(id)
    }
}

impl Default for ReachServer {
    fn default() -> Self {
        Self::new(crate::constants::REACH_DEFAULT_PORT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps_desktop() -> DeviceCapabilities {
        DeviceCapabilities {
            has_keyboard: true,
            has_display: true,
            display_width: Some(1440),
            has_microphone: true,
            has_speaker: true,
            ..Default::default()
        }
    }

    #[test]
    fn register_and_connect() {
        let mut server = ReachServer::new(7474);
        let device_id = server
            .register_device("My MacBook", caps_desktop(), "secret-token-123")
            .unwrap();

        let session_id = server.connect(&device_id, "secret-token-123").unwrap();
        assert!(server.get_session(&session_id).is_some());
        assert_eq!(server.active_session_count(), 1);
    }

    #[test]
    fn wrong_token_rejected() {
        let mut server = ReachServer::new(7474);
        let device_id = server
            .register_device("Device", caps_desktop(), "correct-token")
            .unwrap();
        let result = server.connect(&device_id, "wrong-token");
        assert!(matches!(
            result,
            Err(ReachError::AuthenticationFailed { .. })
        ));
    }

    #[test]
    fn disconnect_marks_session_inactive() {
        let mut server = ReachServer::new(7474);
        let did = server
            .register_device("D", caps_desktop(), "tok")
            .unwrap();
        let sid = server.connect(&did, "tok").unwrap();
        server.disconnect(&sid);
        assert!(!server.get_session(&sid).unwrap().is_active());
    }

    #[test]
    fn max_connections_enforced() {
        let mut server = ReachServer::new(7474);
        for i in 0..MAX_DEVICE_CONNECTIONS {
            server
                .register_device(
                    format!("device-{}", i),
                    caps_desktop(),
                    format!("tok-{}", i),
                )
                .unwrap();
        }
        let result = server.register_device("overflow", caps_desktop(), "tok");
        assert!(matches!(
            result,
            Err(ReachError::MaxConnectionsReached { .. })
        ));
    }
}
