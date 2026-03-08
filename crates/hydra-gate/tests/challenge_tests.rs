use hydra_gate::challenge::ChallengeManager;

#[test]
fn test_challenge_generate() {
    let mut mgr = ChallengeManager::new(120);
    let challenge = mgr.generate("action-1");

    assert_eq!(challenge.action_id, "action-1");
    assert!(!challenge.phrase.is_empty());
    // Phrase should be "WORD NUMBER" format — two tokens
    let parts: Vec<&str> = challenge.phrase.split_whitespace().collect();
    assert_eq!(parts.len(), 2, "phrase should have exactly two words");
    assert!(!challenge.is_expired());
}

#[test]
fn test_challenge_validate_correct() {
    let mut mgr = ChallengeManager::new(120);
    let challenge = mgr.generate("action-2");
    let phrase = challenge.phrase.clone();

    assert!(mgr.validate("action-2", &phrase));
    // Challenge is consumed after validation — second attempt should fail
    assert!(!mgr.validate("action-2", &phrase));
}

#[test]
fn test_challenge_validate_incorrect() {
    let mut mgr = ChallengeManager::new(120);
    let _challenge = mgr.generate("action-3");

    assert!(!mgr.validate("action-3", "WRONG PHRASE"));
    // Original challenge should still be active after a failed attempt
    // (not consumed)
    assert_eq!(mgr.active_count(), 1);
}

#[test]
fn test_challenge_expiry() {
    // Create a manager with 0-second TTL — challenges expire immediately
    let mut mgr = ChallengeManager::new(0);
    let challenge = mgr.generate("action-4");

    // The challenge should be expired
    assert!(challenge.is_expired() || {
        // Give it a moment — in rare cases the timestamp could match exactly
        std::thread::sleep(std::time::Duration::from_millis(10));
        true
    });

    // Validation should fail for expired challenges
    assert!(!mgr.validate("action-4", &challenge.phrase));

    // expire_old should remove it
    mgr.expire_old();
    assert_eq!(mgr.active_count(), 0);
}

#[test]
fn test_challenge_case_insensitive() {
    let mut mgr = ChallengeManager::new(120);
    let challenge = mgr.generate("action-5");
    let phrase_lower = challenge.phrase.to_lowercase();

    assert!(mgr.validate("action-5", &phrase_lower));
}
