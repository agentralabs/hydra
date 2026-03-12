//! Category 1: Unit Gap Fill — hydra-runtime edge cases.

use hydra_runtime::hardening::validation::*;
use hydra_runtime::hardening::isolation::ProjectIsolation;
use hydra_runtime::hardening::lock::{LockManager, LockError};
use hydra_runtime::hardening::auth::{AuthManager, AuthError};

// ============================================================
// Phase 24A: Runtime Hardening Tests
// ============================================================

// === Validation: Intent ===

#[test]
fn test_validate_intent_empty() {
    let result = validate_intent("");
    assert_eq!(result.unwrap_err(), ValidationError::Empty);
}

#[test]
fn test_validate_intent_too_long() {
    let long = "x".repeat(10_001);
    let result = validate_intent(&long);
    assert_eq!(
        result.unwrap_err(),
        ValidationError::TooLong {
            len: 10_001,
            max: 10_000
        }
    );
}

#[test]
fn test_validate_intent_null_bytes() {
    let result = validate_intent("hello\0world");
    assert_eq!(result.unwrap_err(), ValidationError::ContainsNullBytes);
}

#[test]
fn test_validate_intent_valid() {
    let result = validate_intent("Fix the bug in auth module");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str(), "Fix the bug in auth module");
}

// === Validation: Run ID ===

#[test]
fn test_validate_run_id_valid() {
    assert!(validate_run_id("abc-123").is_ok());
    assert!(validate_run_id("run-2026-03-07-deadbeef").is_ok());
}

#[test]
fn test_validate_run_id_invalid() {
    assert!(validate_run_id("").is_err());
    assert!(validate_run_id("has spaces").is_err());
    assert!(validate_run_id("has.dots").is_err());
    assert!(validate_run_id(&"x".repeat(65)).is_err());
}

// === Validation: Config Key ===

#[test]
fn test_validate_config_key_valid() {
    assert!(validate_config_key("runtime.limits.token_budget").is_ok());
    assert!(validate_config_key("single").is_ok());
}

#[test]
fn test_validate_config_key_invalid() {
    assert!(validate_config_key("").is_err());
    assert!(validate_config_key(".leading").is_err());
    assert!(validate_config_key("trailing.").is_err());
    assert!(validate_config_key("double..dot").is_err());
    assert!(validate_config_key("has spaces.bad").is_err());
}

// === Project Isolation ===

#[test]
fn test_project_hash_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    let iso1 = ProjectIsolation::new(tmp.path());
    let iso2 = ProjectIsolation::new(tmp.path());
    assert_eq!(iso1.project_hash(), iso2.project_hash());
    assert_eq!(iso1.project_hash().len(), 12);
}

#[test]
fn test_project_isolation_data_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let iso = ProjectIsolation::new(tmp.path());
    let data_dir = iso.data_dir();
    let hash = iso.project_hash();
    assert!(data_dir.ends_with(format!(".hydra/projects/{hash}")));
    // lock_path is inside data_dir
    assert_eq!(iso.lock_path(), data_dir.join("hydra.lock"));
}

// === Lock Manager ===

#[test]
fn test_lock_acquire_release() {
    let tmp = tempfile::tempdir().unwrap();
    let lock_path = tmp.path().join("test.lock");
    let mgr = LockManager::new(lock_path.clone());

    {
        let guard = mgr.acquire().expect("should acquire lock");
        assert!(lock_path.exists());
        drop(guard);
    }
    // After drop, lock file should be removed
    assert!(!lock_path.exists());
}

#[test]
fn test_lock_stale_detection() {
    let tmp = tempfile::tempdir().unwrap();
    let lock_path = tmp.path().join("stale.lock");

    // Write a lock with a fake dead PID and old timestamp
    std::fs::write(&lock_path, "999999999\n0").unwrap();

    let mgr = LockManager::new(lock_path.clone());
    assert!(mgr.is_stale());

    // Should be able to acquire after stale detection
    let guard = mgr.acquire().expect("should recover stale and acquire");
    assert!(lock_path.exists());
    drop(guard);
}

// === Auth Manager ===

#[test]
fn test_auth_missing_token() {
    let mgr = AuthManager::with_token(None);
    let result = mgr.validate_token("anything");
    assert_eq!(result.unwrap_err(), AuthError::MissingToken);
}

#[test]
fn test_auth_invalid_token() {
    let mgr = AuthManager::with_token(Some("correct-token".into()));
    let result = mgr.validate_token("wrong-token");
    assert_eq!(result.unwrap_err(), AuthError::InvalidToken);
}

#[test]
fn test_auth_valid_token() {
    let mgr = AuthManager::with_token(Some("test-secret-42".into()));
    assert!(mgr.validate_token("test-secret-42").is_ok());

    // Also test require_auth with proper header
    let headers = vec![(
        "Authorization".to_string(),
        "Bearer test-secret-42".to_string(),
    )];
    assert!(mgr.require_auth(&headers).is_ok());

    // Missing header
    let empty: Vec<(String, String)> = vec![];
    assert_eq!(mgr.require_auth(&empty).unwrap_err(), AuthError::MissingHeader);
}

