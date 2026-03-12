use std::path::PathBuf;

use hydra_runtime::{
    boot::BootSequence, shutdown::ShutdownSequence, EventBus, HydraRuntimeConfig, KillSwitch,
    ResourceProfile, TaskRegistry,
};

// ═══════════════════════════════════════════════════════════
// CONFIG TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_config_load_from_toml() {
    let toml_str = r#"
        data_dir = "/tmp/test-hydra"
        profile = "performance"
        voice_enabled = true
        wake_word = "hey test"
        api_port = 8888
        log_level = "debug"
        server_mode = true

        [llm]
        anthropic_api_key = "sk-ant-test"
        default_provider = "anthropic"
        perception_model = "claude-haiku"
        thinking_model = "claude-opus"

        [limits]
        token_budget = 50000
        max_concurrent_runs = 5
        approval_timeout_secs = 120
    "#;

    let config: HydraRuntimeConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.data_dir, PathBuf::from("/tmp/test-hydra"));
    assert_eq!(config.profile, ResourceProfile::Performance);
    assert!(config.voice_enabled);
    assert_eq!(config.api_port, 8888);
    assert_eq!(config.llm.anthropic_api_key, Some("sk-ant-test".into()));
    assert_eq!(config.llm.perception_model, Some("claude-haiku".into()));
    assert_eq!(config.limits.token_budget, 50000);
    assert_eq!(config.limits.max_concurrent_runs, 5);
    assert_eq!(config.limits.approval_timeout_secs, 120);
}

#[test]
fn test_config_env_override() {
    let mut config = HydraRuntimeConfig::default();
    assert_eq!(config.profile, ResourceProfile::Standard);
    assert!(config.llm.anthropic_api_key.is_none());

    // Env override would set keys, but we test the method exists
    // (Can't set env vars safely in parallel tests)
    config.apply_env_overrides();

    // Default values should remain if env vars not set
    assert_eq!(config.api_port, 7777);
}

#[test]
fn test_config_to_llm_config() {
    let mut config = HydraRuntimeConfig::default();
    config.llm.anthropic_api_key = Some("sk-ant-test".into());
    config.llm.openai_api_key = Some("sk-test".into());

    let llm_config = config.to_llm_config();
    assert_eq!(llm_config.anthropic_api_key, Some("sk-ant-test".into()));
    assert_eq!(llm_config.openai_api_key, Some("sk-test".into()));
    assert!(llm_config.has_anthropic());
    assert!(llm_config.has_openai());
}

#[test]
fn test_config_validate_limits() {
    let mut config = HydraRuntimeConfig::default();
    config.limits.token_budget = 0;
    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err()[0].contains("Token budget"));
}

#[test]
fn test_config_checkpoint_path() {
    let config = HydraRuntimeConfig::default();
    let path = config.checkpoint_path();
    assert!(path.to_str().unwrap().ends_with("checkpoint.json"));
}

#[test]
fn test_config_default_values() {
    let config = HydraRuntimeConfig::default();
    assert_eq!(config.limits.token_budget, 100_000);
    assert_eq!(config.limits.max_concurrent_runs, 10);
    assert_eq!(config.limits.approval_timeout_secs, 300);
    assert!(config.llm.anthropic_api_key.is_none());
    assert!(config.llm.openai_api_key.is_none());
}

// ═══════════════════════════════════════════════════════════
// BOOT + RECOVERY TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_boot_loads_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = HydraRuntimeConfig::default();
    config.data_dir = dir.path().to_path_buf();

    // Write a fake checkpoint
    let checkpoint_path = config.checkpoint_path();
    std::fs::write(
        &checkpoint_path,
        r#"{"type":"graceful_shutdown","phase":"decide"}"#,
    )
    .unwrap();

    let event_bus = EventBus::new(64);
    let mut boot = BootSequence::new(config);
    let result = boot.execute(&event_bus).await;
    assert!(result.is_ok());

    // Checkpoint should have been loaded
    assert!(boot.last_checkpoint().is_some());
    let checkpoint_data = boot.last_checkpoint().unwrap();
    assert!(checkpoint_data.contains("graceful_shutdown"));
}

#[tokio::test]
async fn test_boot_detects_incomplete_runs() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = HydraRuntimeConfig::default();
    config.data_dir = dir.path().to_path_buf();

    let event_bus = EventBus::new(64);
    let mut boot = BootSequence::new(config);
    let result = boot.execute(&event_bus).await;
    assert!(result.is_ok());

    // No orphaned runs in a fresh boot
    assert!(boot.orphaned_runs().is_empty());
}

// ═══════════════════════════════════════════════════════════
// SHUTDOWN + CHECKPOINT SAVE TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_graceful_shutdown_saves_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let checkpoint_path = dir.path().join("checkpoint.json");

    let event_bus = EventBus::new(64);
    let shutdown = ShutdownSequence::new();
    let kill_switch = KillSwitch::new();
    let task_registry = TaskRegistry::new();

    let result = shutdown
        .execute_with_registry(
            &event_bus,
            "test shutdown",
            Some(&kill_switch),
            Some(&task_registry),
            Some(&checkpoint_path),
        )
        .await;

    assert!(result.clean);
    assert_eq!(result.reason, "test shutdown");

    // Checkpoint file should exist
    assert!(checkpoint_path.exists());
    let contents = std::fs::read_to_string(&checkpoint_path).unwrap();
    assert!(contents.contains("graceful_shutdown"));

    // Kill switch should be active after shutdown
    assert!(kill_switch.is_active());
}

#[tokio::test]
async fn test_config_reload_runtime() {
    // Test that config can be re-parsed at runtime
    let toml1 = r#"
        data_dir = "/tmp/test1"
        profile = "minimal"
        voice_enabled = false
        wake_word = "hey hydra"
        api_port = 7777
        log_level = "info"
        server_mode = false
    "#;

    let toml2 = r#"
        data_dir = "/tmp/test2"
        profile = "performance"
        voice_enabled = true
        wake_word = "hey hydra"
        api_port = 9999
        log_level = "debug"
        server_mode = true
    "#;

    let config1: HydraRuntimeConfig = toml::from_str(toml1).unwrap();
    let config2: HydraRuntimeConfig = toml::from_str(toml2).unwrap();

    assert_eq!(config1.profile, ResourceProfile::Minimal);
    assert_eq!(config2.profile, ResourceProfile::Performance);
    assert_eq!(config1.api_port, 7777);
    assert_eq!(config2.api_port, 9999);
}
