use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

// ═══════════════════════════════════════════════════════════
// PUSH MESSAGE & ERROR TYPES
// ═══════════════════════════════════════════════════════════

/// A push notification message to deliver via an external provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushMessage {
    pub title: String,
    pub body: String,
    pub urgency: String,
    pub action_url: Option<String>,
}

/// Errors that can occur during push delivery
#[derive(Debug)]
pub enum PushError {
    NetworkError(String),
    AuthError(String),
    DeviceNotRegistered,
    RateLimited,
    ProviderError(String),
}

impl std::fmt::Display for PushError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PushError::NetworkError(msg) => write!(f, "network error: {}", msg),
            PushError::AuthError(msg) => write!(f, "authentication error: {}", msg),
            PushError::DeviceNotRegistered => write!(f, "device not registered"),
            PushError::RateLimited => write!(f, "rate limited"),
            PushError::ProviderError(msg) => write!(f, "provider error: {}", msg),
        }
    }
}

impl std::error::Error for PushError {}

// ═══════════════════════════════════════════════════════════
// PUSH PROVIDER TRAIT
// ═══════════════════════════════════════════════════════════

/// Trait for push notification delivery providers
#[async_trait]
pub trait PushProvider: Send + Sync {
    /// Send a push notification message
    async fn send(&self, message: &PushMessage) -> Result<(), PushError>;

    /// Human-readable name for logging
    fn provider_name(&self) -> &str;
}

// ═══════════════════════════════════════════════════════════
// DEVICE REGISTRY
// ═══════════════════════════════════════════════════════════

/// A device registered for push notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredDevice {
    pub name: String,
    pub provider_type: String,
    pub push_token: String,
    pub last_seen: DateTime<Utc>,
    pub urgency_filter: Vec<String>,
}

/// Registry of devices that receive push notifications.
/// Persists to `~/.hydra/devices.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistry {
    pub devices: Vec<RegisteredDevice>,
    #[serde(skip)]
    storage_path: PathBuf,
}

impl DeviceRegistry {
    /// Create a new empty registry with the default storage path
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            storage_path: Self::default_path(),
        }
    }

    /// Create a registry with a custom storage path (useful for testing)
    pub fn with_path(storage_path: PathBuf) -> Self {
        Self {
            devices: Vec::new(),
            storage_path,
        }
    }

    /// Default path: ~/.hydra/devices.json
    pub fn default_path() -> PathBuf {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".hydra").join("devices.json")
    }

    /// Add a device to the registry. If a device with the same name exists,
    /// it is replaced.
    pub fn add_device(&mut self, device: RegisteredDevice) {
        self.devices.retain(|d| d.name != device.name);
        self.devices.push(device);
    }

    /// Remove a device by name. Returns true if a device was removed.
    pub fn remove_device(&mut self, name: &str) -> bool {
        let before = self.devices.len();
        self.devices.retain(|d| d.name != name);
        self.devices.len() < before
    }

    /// Get a device by name
    pub fn get_device(&self, name: &str) -> Option<&RegisteredDevice> {
        self.devices.iter().find(|d| d.name == name)
    }

    /// List all registered devices
    pub fn list_devices(&self) -> &[RegisteredDevice] {
        &self.devices
    }

    /// Get devices that accept a given urgency level
    pub fn devices_for_urgency(&self, urgency: &str) -> Vec<&RegisteredDevice> {
        self.devices
            .iter()
            .filter(|d| d.urgency_filter.is_empty() || d.urgency_filter.iter().any(|u| u == urgency))
            .collect()
    }

    /// Save the registry to disk as JSON
    pub fn save(&self) -> Result<(), PushError> {
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                PushError::ProviderError(format!("failed to create directory: {}", e))
            })?;
        }
        let json = serde_json::to_string_pretty(&self).map_err(|e| {
            PushError::ProviderError(format!("failed to serialize registry: {}", e))
        })?;
        std::fs::write(&self.storage_path, json).map_err(|e| {
            PushError::ProviderError(format!("failed to write registry: {}", e))
        })?;
        debug!("Saved device registry to {:?}", self.storage_path);
        Ok(())
    }

    /// Load the registry from disk. Returns a new empty registry if the
    /// file does not exist.
    pub fn load(storage_path: PathBuf) -> Result<Self, PushError> {
        if !storage_path.exists() {
            debug!("No device registry at {:?}, starting fresh", storage_path);
            return Ok(Self::with_path(storage_path));
        }
        let content = std::fs::read_to_string(&storage_path).map_err(|e| {
            PushError::ProviderError(format!("failed to read registry: {}", e))
        })?;
        let mut registry: DeviceRegistry = serde_json::from_str(&content).map_err(|e| {
            PushError::ProviderError(format!("failed to parse registry: {}", e))
        })?;
        registry.storage_path = storage_path;
        Ok(registry)
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// We need dirs_next or a fallback. Since it may not be in workspace deps,
// use a cfg-gated fallback.
mod dirs_next {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .map(PathBuf::from)
    }
}
