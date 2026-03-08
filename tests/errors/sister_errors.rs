//! Category 6: Error Paths — sister errors.

use hydra_core::*;

#[test]
fn test_sister_not_found() {
    let err = HydraError::SisterNotFound("nonexistent-sister".into());
    assert!(!err.is_retryable());
    assert!(err.suggested_action().is_some());
    assert_eq!(err.error_code(), "E301");
}

#[test]
fn test_sister_unreachable() {
    let err = HydraError::SisterUnreachable("memory".into());
    assert!(err.is_retryable());
    assert_eq!(err.error_code(), "E302");
}

#[test]
fn test_sister_timeout() {
    let err = HydraError::Timeout;
    assert!(err.is_retryable());
    assert_eq!(err.error_code(), "E203");
}

#[test]
fn test_io_error_from_std() {
    let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
    let hydra_err: HydraError = io_err.into();
    match &hydra_err {
        HydraError::IoError(msg) => assert!(msg.contains("connection refused")),
        _ => panic!("expected IoError"),
    }
    assert!(hydra_err.is_retryable());
}
