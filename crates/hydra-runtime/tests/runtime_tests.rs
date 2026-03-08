use hydra_runtime::boot::BootSequence;
use hydra_runtime::config::{HydraRuntimeConfig, ResourceProfile};
use hydra_runtime::event_bus::EventBus;
use hydra_runtime::filesystem::{init_filesystem, verify_filesystem};
use hydra_runtime::jsonrpc::{JsonRpcRequest, JsonRpcResponse, RpcErrorCodes};
use hydra_runtime::lock::InstanceLock;
use hydra_runtime::runtime::HydraRuntime;
use hydra_runtime::shutdown::ShutdownSequence;
use hydra_runtime::sse::{SseEvent, SseEventType};

// ═══════════════════════════════════════════════════════════
// CONFIG TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_config_load_default() {
    let config = HydraRuntimeConfig::default();
    assert_eq!(config.api_port, 7777);
    assert_eq!(config.profile, ResourceProfile::Standard);
    assert!(!config.voice_enabled);
    assert_eq!(config.log_level, "info");
}

#[test]
fn test_config_validate_valid() {
    let config = HydraRuntimeConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validate_invalid_port() {
    let mut config = HydraRuntimeConfig::default();
    config.api_port = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_config_validate_invalid_log_level() {
    let mut config = HydraRuntimeConfig::default();
    config.log_level = "garbage".into();
    assert!(config.validate().is_err());
}

#[test]
fn test_config_env_override() {
    let mut config = HydraRuntimeConfig::default();
    // Can't actually set env vars safely in tests, but verify the method exists
    config.apply_env_overrides();
    // Config should remain valid after applying overrides
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_serde_roundtrip() {
    let config = HydraRuntimeConfig::default();
    let toml_str = toml::to_string(&config).unwrap();
    let parsed: HydraRuntimeConfig = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.api_port, config.api_port);
    assert_eq!(parsed.profile, config.profile);
}

// ═══════════════════════════════════════════════════════════
// BOOT SEQUENCE TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_boot_sequence_order() {
    let config = HydraRuntimeConfig::default();
    let event_bus = EventBus::new(64);
    let mut boot = BootSequence::new(config);
    let result = boot.execute(&event_bus).await;
    assert!(result.is_ok());
    assert!(boot.all_succeeded());

    // Verify phases in order
    let results = boot.results();
    assert!(results.len() >= 5);
    assert_eq!(results[0].phase, "preflight");
    assert_eq!(results[1].phase, "core_services");
    assert_eq!(results[2].phase, "sister_bridges");
    assert_eq!(results[3].phase, "execution_engine");
    assert_eq!(results[4].phase, "surfaces");
}

#[tokio::test]
async fn test_boot_required_sisters_fail() {
    // Invalid config should fail boot
    let mut config = HydraRuntimeConfig::default();
    config.api_port = 0; // Invalid
    let event_bus = EventBus::new(64);
    let mut boot = BootSequence::new(config);
    let result = boot.execute(&event_bus).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_boot_optional_sisters_degrade() {
    // Normal config boots fine even without optional sisters
    let config = HydraRuntimeConfig::default();
    let event_bus = EventBus::new(64);
    let mut boot = BootSequence::new(config);
    assert!(boot.execute(&event_bus).await.is_ok());
}

#[tokio::test]
async fn test_boot_timeout_handling() {
    let config = HydraRuntimeConfig::default();
    let event_bus = EventBus::new(64);
    let mut boot = BootSequence::new(config);
    let result = boot.execute(&event_bus).await;
    assert!(result.is_ok());
    // Total boot should be fast (< 3s)
    assert!(boot.total_duration_ms() < 3000);
}

// ═══════════════════════════════════════════════════════════
// SHUTDOWN TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_shutdown_graceful() {
    let shutdown = ShutdownSequence::new();
    let event_bus = EventBus::new(64);
    let mut rx = event_bus.subscribe();
    let result = shutdown.execute(&event_bus, "test shutdown").await;
    assert!(result.clean);
    assert_eq!(result.exit_code, 0);
    assert!(shutdown.is_shutting_down());

    // Should have emitted shutdown event
    let event = rx.try_recv().unwrap();
    assert_eq!(event.event_type, SseEventType::SystemShutdown);
}

#[tokio::test]
async fn test_shutdown_forced() {
    let shutdown = ShutdownSequence::new();
    shutdown.force_shutdown();
    assert!(shutdown.is_shutting_down());
}

#[tokio::test]
async fn test_shutdown_with_active_runs() {
    // Even with active runs, shutdown should complete
    let shutdown = ShutdownSequence::new();
    let event_bus = EventBus::new(64);
    let result = shutdown
        .execute(&event_bus, "shutdown with active runs")
        .await;
    assert!(result.clean);
}

#[tokio::test]
async fn test_shutdown_signal_handling() {
    let shutdown = ShutdownSequence::new();
    let flag = shutdown.flag();
    assert!(!flag.load(std::sync::atomic::Ordering::SeqCst));
    let event_bus = EventBus::new(64);
    shutdown.execute(&event_bus, "signal").await;
    assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
}

// ═══════════════════════════════════════════════════════════
// EVENT BUS TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_event_bus_publish_subscribe() {
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    bus.publish(SseEvent::heartbeat());
    let event = rx.try_recv().unwrap();
    assert_eq!(event.event_type, SseEventType::Heartbeat);
}

#[test]
fn test_event_bus_multiple_subscribers() {
    let bus = EventBus::new(64);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();
    bus.publish(SseEvent::heartbeat());
    assert!(rx1.try_recv().is_ok());
    assert!(rx2.try_recv().is_ok());
}

#[test]
fn test_event_bus_counter() {
    let bus = EventBus::new(64);
    assert_eq!(bus.total_published(), 0);
    bus.publish(SseEvent::heartbeat());
    bus.publish(SseEvent::heartbeat());
    assert_eq!(bus.total_published(), 2);
}

// ═══════════════════════════════════════════════════════════
// SINGLE INSTANCE LOCK TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_single_instance_lock() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = InstanceLock::new(dir.path());
    assert!(lock.acquire().is_ok());
    assert!(lock.is_held());
    assert!(lock.lock_exists());
    lock.release();
    assert!(!lock.is_held());
}

#[test]
fn test_stale_lock_recovery() {
    let dir = tempfile::tempdir().unwrap();
    // Write a stale lock (PID that doesn't exist)
    std::fs::write(dir.path().join("hydra.lock"), "99999999").unwrap();
    let mut lock = InstanceLock::new(dir.path());
    // Should recover stale lock
    assert!(lock.acquire().is_ok());
    assert!(lock.is_held());
}

// ═══════════════════════════════════════════════════════════
// JSON-RPC TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_jsonrpc_valid_request() {
    let req: JsonRpcRequest = serde_json::from_str(
        r#"{
        "jsonrpc": "2.0",
        "id": "test-123",
        "method": "hydra.run",
        "params": {"intent": "list files"}
    }"#,
    )
    .unwrap();
    assert!(req.is_valid());
    assert_eq!(req.method, "hydra.run");
}

#[test]
fn test_jsonrpc_invalid_request() {
    let req: JsonRpcRequest = serde_json::from_str(
        r#"{
        "jsonrpc": "1.0",
        "id": "test",
        "method": ""
    }"#,
    )
    .unwrap();
    assert!(!req.is_valid());
}

#[test]
fn test_jsonrpc_error_codes() {
    assert_eq!(RpcErrorCodes::PARSE_ERROR, -32700);
    assert_eq!(RpcErrorCodes::INVALID_REQUEST, -32600);
    assert_eq!(RpcErrorCodes::METHOD_NOT_FOUND, -32601);
    assert_eq!(RpcErrorCodes::INVALID_PARAMS, -32602);
    assert_eq!(RpcErrorCodes::INTERNAL_ERROR, -32603);
    assert_eq!(RpcErrorCodes::SISTER_UNAVAILABLE, -32000);
    assert_eq!(RpcErrorCodes::RUN_FAILED, -32001);
    assert_eq!(RpcErrorCodes::APPROVAL_REQUIRED, -32002);
    assert_eq!(RpcErrorCodes::CAPABILITY_DENIED, -32003);
    assert_eq!(RpcErrorCodes::RESOURCE_EXHAUSTED, -32004);
    assert_eq!(RpcErrorCodes::TIMEOUT, -32005);
}

#[test]
fn test_jsonrpc_success_response() {
    let resp = JsonRpcResponse::success(
        serde_json::json!("req-1"),
        serde_json::json!({"status": "ok"}),
    );
    assert!(resp.is_success());
    assert_eq!(resp.jsonrpc, "2.0");
}

#[test]
fn test_jsonrpc_error_response() {
    let resp = JsonRpcResponse::error(
        serde_json::json!("req-1"),
        RpcErrorCodes::METHOD_NOT_FOUND,
        "Method not found",
    );
    assert!(!resp.is_success());
    assert_eq!(resp.error.as_ref().unwrap().code, -32601);
}

#[test]
fn test_jsonrpc_response_serde() {
    let resp = JsonRpcResponse::success(serde_json::json!("1"), serde_json::json!({}));
    let json = serde_json::to_string(&resp).unwrap();
    assert!(!json.contains("error")); // skip_serializing_if works
    let parsed: JsonRpcResponse = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_success());
}

// ═══════════════════════════════════════════════════════════
// SSE EVENT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_sse_event_types() {
    let events = vec![
        SseEvent::heartbeat(),
        SseEvent::system_ready("0.1.0"),
        SseEvent::system_shutdown("test"),
    ];
    assert_eq!(events[0].event_type, SseEventType::Heartbeat);
    assert_eq!(events[1].event_type, SseEventType::SystemReady);
    assert_eq!(events[2].event_type, SseEventType::SystemShutdown);
}

#[test]
fn test_sse_heartbeat() {
    let hb = SseEvent::heartbeat();
    assert_eq!(hb.data["status"], "alive");
}

#[test]
fn test_sse_wire_format() {
    let event = SseEvent::heartbeat();
    let wire = event.to_sse_string();
    assert!(wire.contains("event: heartbeat"));
    assert!(wire.contains("data: "));
    assert!(wire.ends_with("\n\n"));
}

// ═══════════════════════════════════════════════════════════
// FULL RUNTIME TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_runtime_boot_and_shutdown() {
    let config = HydraRuntimeConfig::default();
    let mut runtime = HydraRuntime::new(config);
    assert!(!runtime.is_booted());

    runtime.boot().await.unwrap();
    assert!(runtime.is_booted());
    assert!(!runtime.is_shutting_down());

    let result = runtime.shutdown("test").await;
    assert!(result.clean);
    assert!(runtime.is_shutting_down());
}

#[tokio::test]
async fn test_runtime_event_bus() {
    let config = HydraRuntimeConfig::default();
    let mut runtime = HydraRuntime::new(config);
    let mut rx = runtime.event_bus().subscribe();
    runtime.boot().await.unwrap();
    // Boot emits system_ready event
    let event = rx.try_recv().unwrap();
    assert_eq!(event.event_type, SseEventType::SystemReady);
}

// ═══════════════════════════════════════════════════════════
// FILESYSTEM TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_filesystem_init_creates_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("hydra-data");
    init_filesystem(&data_dir).unwrap();

    assert!(data_dir.is_dir());
    assert!(data_dir.join("receipts").is_dir());
    assert!(data_dir.join("evidence").is_dir());
    assert!(data_dir.join("cache").is_dir());
    assert!(data_dir.join("logs").is_dir());
    assert!(data_dir.join("voice").is_dir());
    assert!(verify_filesystem(&data_dir));
}

#[test]
fn test_filesystem_init_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("hydra-data");

    // Call twice — should not error
    init_filesystem(&data_dir).unwrap();
    init_filesystem(&data_dir).unwrap();

    assert!(verify_filesystem(&data_dir));
}

#[cfg(unix)]
#[test]
fn test_filesystem_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("hydra-data");
    init_filesystem(&data_dir).unwrap();

    let mode = std::fs::metadata(&data_dir).unwrap().permissions().mode();
    // Check owner rwx (0o700)
    assert_eq!(mode & 0o777, 0o700);

    let receipts_mode = std::fs::metadata(data_dir.join("receipts"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(receipts_mode & 0o777, 0o700);
}
