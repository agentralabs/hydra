use hydra_model::executor;

// ═══════════════════════════════════════════════════════════
// TIMEOUT CONSTANTS TEST
// ═══════════════════════════════════════════════════════════

#[test]
fn test_timeout_constants() {
    assert_eq!(
        executor::LLM_COMPLETION_TIMEOUT,
        std::time::Duration::from_secs(30)
    );
    assert_eq!(
        executor::LLM_FIRST_TOKEN_TIMEOUT,
        std::time::Duration::from_secs(10)
    );
    assert_eq!(
        executor::LLM_STREAMING_TIMEOUT,
        std::time::Duration::from_secs(60)
    );
    assert_eq!(
        executor::HEALTH_CHECK_TIMEOUT,
        std::time::Duration::from_secs(5)
    );
    assert_eq!(executor::MAX_RETRY_ATTEMPTS, 3);
}
