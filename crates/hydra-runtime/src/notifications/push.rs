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
// NTFY PROVIDER
// ═══════════════════════════════════════════════════════════

/// Push provider using ntfy.sh (or self-hosted ntfy instance)
pub struct NtfyProvider {
    pub topic_id: String,
    pub server_url: String,
    client: reqwest::Client,
}

impl NtfyProvider {
    pub fn new(topic_id: String) -> Self {
        Self {
            topic_id,
            server_url: "https://ntfy.sh".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_server(topic_id: String, server_url: String) -> Self {
        Self {
            topic_id,
            server_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PushProvider for NtfyProvider {
    async fn send(&self, message: &PushMessage) -> Result<(), PushError> {
        let url = format!("{}/{}", self.server_url, self.topic_id);
        debug!("Sending ntfy notification to {}", url);

        let mut request = self
            .client
            .post(&url)
            .header("Title", &message.title)
            .header("Priority", urgency_to_ntfy_priority(&message.urgency));

        if let Some(ref action_url) = message.action_url {
            request = request.header("Click", action_url);
        }

        let response = request
            .body(message.body.clone())
            .send()
            .await
            .map_err(|e| PushError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status.as_u16() == 401 || status.as_u16() == 403 {
            Err(PushError::AuthError(format!(
                "ntfy returned {}",
                status.as_u16()
            )))
        } else if status.as_u16() == 429 {
            Err(PushError::RateLimited)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(PushError::ProviderError(format!(
                "ntfy returned {}: {}",
                status.as_u16(),
                body
            )))
        }
    }

    fn provider_name(&self) -> &str {
        "ntfy"
    }
}

fn urgency_to_ntfy_priority(urgency: &str) -> &'static str {
    match urgency {
        "high" | "urgent" => "urgent",
        "low" => "low",
        _ => "default",
    }
}

// ═══════════════════════════════════════════════════════════
// WEB PUSH PROVIDER
// ═══════════════════════════════════════════════════════════

/// Web Push provider using VAPID keys
///
/// Web Push requires VAPID key generation and encryption of the payload
/// using the push subscription's public key. This implementation provides
/// the structure; full encryption would use the `web-push` crate.
pub struct WebPushProvider {
    pub vapid_key: String,
    pub subscription_endpoint: String,
    client: reqwest::Client,
}

impl WebPushProvider {
    pub fn new(vapid_key: String, subscription_endpoint: String) -> Self {
        Self {
            vapid_key,
            subscription_endpoint,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PushProvider for WebPushProvider {
    async fn send(&self, message: &PushMessage) -> Result<(), PushError> {
        // Web Push requires VAPID JWT signing and payload encryption using
        // the subscription's p256dh and auth keys. A production implementation
        // would use the `web-push` crate. For now we POST the JSON payload
        // to the subscription endpoint as a structural placeholder.
        debug!(
            "Sending web push to endpoint: {}",
            self.subscription_endpoint
        );

        let payload = serde_json::json!({
            "title": message.title,
            "body": message.body,
            "urgency": message.urgency,
            "action_url": message.action_url,
        });

        let response = self
            .client
            .post(&self.subscription_endpoint)
            .header("TTL", "86400")
            .header("Urgency", &message.urgency)
            .json(&payload)
            .send()
            .await
            .map_err(|e| PushError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 201 {
            Ok(())
        } else if status.as_u16() == 404 || status.as_u16() == 410 {
            Err(PushError::DeviceNotRegistered)
        } else if status.as_u16() == 429 {
            Err(PushError::RateLimited)
        } else {
            Err(PushError::ProviderError(format!(
                "web push returned {}",
                status.as_u16()
            )))
        }
    }

    fn provider_name(&self) -> &str {
        "web_push"
    }
}

// ═══════════════════════════════════════════════════════════
// TELEGRAM PROVIDER
// ═══════════════════════════════════════════════════════════

/// Push provider using the Telegram Bot API
pub struct TelegramProvider {
    pub bot_token: String,
    pub chat_id: String,
    client: reqwest::Client,
}

impl TelegramProvider {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token,
            chat_id,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PushProvider for TelegramProvider {
    async fn send(&self, message: &PushMessage) -> Result<(), PushError> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );
        debug!("Sending Telegram notification to chat {}", self.chat_id);

        let text = if let Some(ref action_url) = message.action_url {
            format!(
                "<b>{}</b>\n\n{}\n\n<a href=\"{}\">Open</a>",
                message.title, message.body, action_url
            )
        } else {
            format!("<b>{}</b>\n\n{}", message.title, message.body)
        };

        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "disable_web_page_preview": true,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| PushError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status.as_u16() == 401 {
            Err(PushError::AuthError(
                "invalid Telegram bot token".to_string(),
            ))
        } else if status.as_u16() == 429 {
            Err(PushError::RateLimited)
        } else {
            let resp_body = response.text().await.unwrap_or_default();
            Err(PushError::ProviderError(format!(
                "Telegram API returned {}: {}",
                status.as_u16(),
                resp_body
            )))
        }
    }

    fn provider_name(&self) -> &str {
        "telegram"
    }
}

// ═══════════════════════════════════════════════════════════
// EMAIL PROVIDER
// ═══════════════════════════════════════════════════════════

/// Push provider using email (SMTP)
///
/// A production implementation would use the `lettre` crate for SMTP.
/// This provides the structure and returns Ok(()) as a placeholder for
/// the actual SMTP integration.
pub struct EmailProvider {
    pub smtp_host: String,
    pub from_address: String,
    pub to_address: String,
}

impl EmailProvider {
    pub fn new(smtp_host: String, from_address: String, to_address: String) -> Self {
        Self {
            smtp_host,
            from_address,
            to_address,
        }
    }
}

#[async_trait]
impl PushProvider for EmailProvider {
    async fn send(&self, message: &PushMessage) -> Result<(), PushError> {
        // SMTP integration point: a production implementation would use the
        // `lettre` crate to connect to self.smtp_host and send an email from
        // self.from_address to self.to_address with the notification content.
        debug!(
            "Email notification (SMTP integration point): from={} to={} via={} subject='{}'",
            self.from_address, self.to_address, self.smtp_host, message.title
        );
        Ok(())
    }

    fn provider_name(&self) -> &str {
        "email"
    }
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

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_device(name: &str) -> RegisteredDevice {
        RegisteredDevice {
            name: name.to_string(),
            provider_type: "ntfy".to_string(),
            push_token: format!("token-{}", name),
            last_seen: Utc::now(),
            urgency_filter: vec![],
        }
    }

    fn make_device_with_filter(name: &str, filter: Vec<&str>) -> RegisteredDevice {
        RegisteredDevice {
            name: name.to_string(),
            provider_type: "telegram".to_string(),
            push_token: format!("token-{}", name),
            last_seen: Utc::now(),
            urgency_filter: filter.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_add_device() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        assert_eq!(reg.devices.len(), 1);
        assert_eq!(reg.devices[0].name, "phone");
    }

    #[test]
    fn test_add_device_replaces_existing() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        let mut updated = make_device("phone");
        updated.push_token = "new-token".to_string();
        reg.add_device(updated);
        assert_eq!(reg.devices.len(), 1);
        assert_eq!(reg.devices[0].push_token, "new-token");
    }

    #[test]
    fn test_remove_device() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        reg.add_device(make_device("tablet"));
        assert!(reg.remove_device("phone"));
        assert_eq!(reg.devices.len(), 1);
        assert_eq!(reg.devices[0].name, "tablet");
    }

    #[test]
    fn test_remove_nonexistent_device() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        assert!(!reg.remove_device("laptop"));
        assert_eq!(reg.devices.len(), 1);
    }

    #[test]
    fn test_get_device() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        assert!(reg.get_device("phone").is_some());
        assert!(reg.get_device("tablet").is_none());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("devices.json");

        let mut reg = DeviceRegistry::with_path(path.clone());
        reg.add_device(make_device("phone"));
        reg.add_device(make_device("tablet"));
        reg.save().unwrap();

        let loaded = DeviceRegistry::load(path).unwrap();
        assert_eq!(loaded.devices.len(), 2);
        assert_eq!(loaded.devices[0].name, "phone");
        assert_eq!(loaded.devices[1].name, "tablet");
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");
        let reg = DeviceRegistry::load(path).unwrap();
        assert!(reg.devices.is_empty());
    }

    #[test]
    fn test_devices_for_urgency_no_filter() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        // Empty filter means accept all urgencies
        let matching = reg.devices_for_urgency("high");
        assert_eq!(matching.len(), 1);
    }

    #[test]
    fn test_devices_for_urgency_with_filter() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device_with_filter("phone", vec!["high", "normal"]));
        reg.add_device(make_device_with_filter("tablet", vec!["high"]));

        let high = reg.devices_for_urgency("high");
        assert_eq!(high.len(), 2);

        let normal = reg.devices_for_urgency("normal");
        assert_eq!(normal.len(), 1);
        assert_eq!(normal[0].name, "phone");

        let low = reg.devices_for_urgency("low");
        assert!(low.is_empty());
    }

    #[test]
    fn test_push_message_serialization() {
        let msg = PushMessage {
            title: "Test".to_string(),
            body: "Hello".to_string(),
            urgency: "normal".to_string(),
            action_url: Some("https://example.com".to_string()),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["title"], "Test");
        assert_eq!(json["action_url"], "https://example.com");
    }

    #[test]
    fn test_registered_device_serialization() {
        let dev = make_device("phone");
        let json = serde_json::to_string(&dev).unwrap();
        let restored: RegisteredDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "phone");
        assert_eq!(restored.provider_type, "ntfy");
    }

    #[test]
    fn test_ntfy_priority_mapping() {
        assert_eq!(urgency_to_ntfy_priority("high"), "urgent");
        assert_eq!(urgency_to_ntfy_priority("urgent"), "urgent");
        assert_eq!(urgency_to_ntfy_priority("low"), "low");
        assert_eq!(urgency_to_ntfy_priority("normal"), "default");
        assert_eq!(urgency_to_ntfy_priority("unknown"), "default");
    }

    #[test]
    fn test_list_devices() {
        let mut reg = DeviceRegistry::new();
        assert!(reg.list_devices().is_empty());
        reg.add_device(make_device("a"));
        reg.add_device(make_device("b"));
        assert_eq!(reg.list_devices().len(), 2);
    }

    #[tokio::test]
    async fn test_email_provider_send() {
        let provider = EmailProvider::new(
            "smtp.example.com".to_string(),
            "hydra@example.com".to_string(),
            "user@example.com".to_string(),
        );
        let msg = PushMessage {
            title: "Test".to_string(),
            body: "Body".to_string(),
            urgency: "normal".to_string(),
            action_url: None,
        };
        // Email provider is a stub — should return Ok
        assert!(provider.send(&msg).await.is_ok());
        assert_eq!(provider.provider_name(), "email");
    }

    #[test]
    fn test_default_registry() {
        let reg = DeviceRegistry::default();
        assert!(reg.devices.is_empty());
    }
}
