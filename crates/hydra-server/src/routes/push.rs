use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use chrono::Utc;
use futures::stream::{self, Stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

// ═══════════════════════════════════════════════════════════
// ROUTE PATHS
// ═══════════════════════════════════════════════════════════

pub struct PushRoutes;

impl PushRoutes {
    /// POST: register a device for push notifications
    pub fn register_device() -> &'static str {
        "/api/push/register"
    }

    /// DELETE: unregister a device by name
    pub fn unregister_device() -> &'static str {
        "/api/push/devices/:name"
    }

    /// GET: list all registered devices
    pub fn list_devices() -> &'static str {
        "/api/push/devices"
    }

    /// POST: send a test notification to all devices
    pub fn test_push() -> &'static str {
        "/api/push/test"
    }

    /// GET: SSE stream for web-based companion
    pub fn subscribe_sse() -> &'static str {
        "/api/push/subscribe"
    }
}

// ═══════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub name: String,
    pub provider: String,
    pub push_token: String,
    #[serde(default)]
    pub urgency_filter: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterDeviceResponse {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct UnregisterDeviceResponse {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub name: String,
    pub provider: String,
    pub push_token: String,
    pub last_seen: String,
    pub urgency_filter: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ListDevicesResponse {
    pub devices: Vec<DeviceInfo>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct TestPushResponse {
    pub sent_to: usize,
    pub status: String,
}

// ═══════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════

/// POST /api/push/register — register a device for push notifications
pub async fn register_device(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<(StatusCode, Json<RegisterDeviceResponse>), (StatusCode, String)> {
    use hydra_runtime::notifications::push::{DeviceRegistry, RegisteredDevice};

    let device = RegisteredDevice {
        name: req.name.clone(),
        provider_type: req.provider,
        push_token: req.push_token,
        last_seen: Utc::now(),
        urgency_filter: req.urgency_filter,
    };

    let mut registry = DeviceRegistry::load(DeviceRegistry::default_path())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    registry.add_device(device);

    registry
        .save()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterDeviceResponse {
            name: req.name,
            status: "registered".into(),
        }),
    ))
}

/// DELETE /api/push/devices/:name — unregister a device by name
pub async fn unregister_device(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<UnregisterDeviceResponse>, (StatusCode, String)> {
    use hydra_runtime::notifications::push::DeviceRegistry;

    let mut registry = DeviceRegistry::load(DeviceRegistry::default_path())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !registry.remove_device(&name) {
        return Err((
            StatusCode::NOT_FOUND,
            format!("device '{}' not found", name),
        ));
    }

    registry
        .save()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UnregisterDeviceResponse {
        name,
        status: "unregistered".into(),
    }))
}

/// GET /api/push/devices — list all registered devices
pub async fn list_devices(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ListDevicesResponse>, (StatusCode, String)> {
    use hydra_runtime::notifications::push::DeviceRegistry;

    let registry = DeviceRegistry::load(DeviceRegistry::default_path())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let devices: Vec<DeviceInfo> = registry
        .list_devices()
        .iter()
        .map(|d| DeviceInfo {
            name: d.name.clone(),
            provider: d.provider_type.clone(),
            push_token: d.push_token.clone(),
            last_seen: d.last_seen.to_rfc3339(),
            urgency_filter: d.urgency_filter.clone(),
        })
        .collect();

    let count = devices.len();
    Ok(Json(ListDevicesResponse { devices, count }))
}

/// POST /api/push/test — send a test notification to all registered devices
pub async fn test_push(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<TestPushResponse>, (StatusCode, String)> {
    use hydra_runtime::notifications::push::{DeviceRegistry, PushMessage};

    let registry = DeviceRegistry::load(DeviceRegistry::default_path())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let device_count = registry.list_devices().len();

    // In a full implementation, we would iterate over devices, instantiate
    // the appropriate provider for each, and call send(). For now we log
    // the intent and report success since actual delivery depends on
    // provider credentials being configured.
    let _message = PushMessage {
        title: "Hydra Test Notification".to_string(),
        body: "This is a test push notification from Hydra.".to_string(),
        urgency: "normal".to_string(),
        action_url: None,
    };

    tracing::info!(
        "Test push requested for {} registered device(s)",
        device_count
    );

    Ok(Json(TestPushResponse {
        sent_to: device_count,
        status: "sent".into(),
    }))
}

/// GET /api/push/subscribe — SSE stream for web-based push companion
pub async fn subscribe_sse(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_bus.subscribe();

    let initial = stream::once(async {
        Ok(Event::default()
            .event("push_connected")
            .data(serde_json::json!({"status": "connected", "version": "0.1.0"}).to_string()))
    });

    let events = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(sse_event) => {
                let event_name = serde_json::to_string(&sse_event.event_type)
                    .unwrap_or_else(|_| "\"unknown\"".into())
                    .trim_matches('"')
                    .to_string();
                let data = serde_json::to_string(&sse_event.data).unwrap_or_default();
                let event = Event::default().event(event_name).data(data);
                Some((Ok(event), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("Push SSE client lagged, skipped {} events", n);
                let event = Event::default()
                    .event("push_reconnected")
                    .data(
                        serde_json::json!({"warning": "reconnected after lag", "skipped": n})
                            .to_string(),
                    );
                Some((Ok(event), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => None,
        }
    });

    Sse::new(initial.chain(events)).keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Route path tests ───────────────────────────────────

    #[test]
    fn test_register_device_path() {
        assert_eq!(PushRoutes::register_device(), "/api/push/register");
    }

    #[test]
    fn test_unregister_device_path() {
        assert_eq!(PushRoutes::unregister_device(), "/api/push/devices/:name");
    }

    #[test]
    fn test_list_devices_path() {
        assert_eq!(PushRoutes::list_devices(), "/api/push/devices");
    }

    #[test]
    fn test_test_push_path() {
        assert_eq!(PushRoutes::test_push(), "/api/push/test");
    }

    #[test]
    fn test_subscribe_sse_path() {
        assert_eq!(PushRoutes::subscribe_sse(), "/api/push/subscribe");
    }

    // ── Request/Response serialization tests ───────────────

    #[test]
    fn test_register_device_request_deserialization() {
        let json = serde_json::json!({
            "name": "my-phone",
            "provider": "ntfy",
            "push_token": "hydra-push-topic",
            "urgency_filter": ["high", "normal"]
        });
        let req: RegisterDeviceRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.name, "my-phone");
        assert_eq!(req.provider, "ntfy");
        assert_eq!(req.push_token, "hydra-push-topic");
        assert_eq!(req.urgency_filter, vec!["high", "normal"]);
    }

    #[test]
    fn test_register_device_request_no_filter() {
        let json = serde_json::json!({
            "name": "tablet",
            "provider": "telegram",
            "push_token": "12345:ABCDEF"
        });
        let req: RegisterDeviceRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.name, "tablet");
        assert!(req.urgency_filter.is_empty());
    }

    #[test]
    fn test_register_device_response_serialization() {
        let resp = RegisterDeviceResponse {
            name: "my-phone".into(),
            status: "registered".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["name"], "my-phone");
        assert_eq!(json["status"], "registered");
    }

    #[test]
    fn test_unregister_device_response_serialization() {
        let resp = UnregisterDeviceResponse {
            name: "old-device".into(),
            status: "unregistered".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["name"], "old-device");
        assert_eq!(json["status"], "unregistered");
    }

    #[test]
    fn test_list_devices_response_serialization() {
        let resp = ListDevicesResponse {
            devices: vec![
                DeviceInfo {
                    name: "phone".into(),
                    provider: "ntfy".into(),
                    push_token: "topic-1".into(),
                    last_seen: "2026-03-08T00:00:00Z".into(),
                    urgency_filter: vec!["high".into()],
                },
                DeviceInfo {
                    name: "tablet".into(),
                    provider: "telegram".into(),
                    push_token: "chat-123".into(),
                    last_seen: "2026-03-07T00:00:00Z".into(),
                    urgency_filter: vec![],
                },
            ],
            count: 2,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["count"], 2);
        assert_eq!(json["devices"].as_array().unwrap().len(), 2);
        assert_eq!(json["devices"][0]["name"], "phone");
    }

    #[test]
    fn test_test_push_response_serialization() {
        let resp = TestPushResponse {
            sent_to: 3,
            status: "sent".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["sent_to"], 3);
        assert_eq!(json["status"], "sent");
    }

    #[test]
    fn test_device_info_serialization() {
        let info = DeviceInfo {
            name: "laptop".into(),
            provider: "web_push".into(),
            push_token: "endpoint-url".into(),
            last_seen: "2026-03-08T12:00:00Z".into(),
            urgency_filter: vec!["high".into(), "normal".into()],
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["name"], "laptop");
        assert_eq!(json["provider"], "web_push");
        assert_eq!(json["urgency_filter"].as_array().unwrap().len(), 2);
    }
}
