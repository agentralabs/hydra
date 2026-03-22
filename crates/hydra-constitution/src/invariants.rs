//! Compile-time assertions for constitutional invariants.
//! If any of these fail, the crate does not compile.

use crate::constants::*;

/// Verify the trust tier ordering is internally consistent.
const _: () = {
    assert!(TRUST_TIER_CONSTITUTION < TRUST_TIER_HYDRA);
    assert!(TRUST_TIER_HYDRA < TRUST_TIER_PRINCIPAL);
    assert!(TRUST_TIER_PRINCIPAL < TRUST_TIER_FLEET);
    assert!(TRUST_TIER_FLEET < TRUST_TIER_SKILLS);
    assert!(TRUST_TIER_SKILLS < TRUST_TIER_EXTERNAL);
    assert!(TRUST_TIER_MAXIMUM == TRUST_TIER_EXTERNAL);
    assert!(TRUST_TIER_MINIMUM == TRUST_TIER_CONSTITUTION);
};

/// Verify the Animus magic header is exactly 4 bytes.
const _: () = {
    assert!(ANIMUS_MAGIC.len() == 4);
};

/// Verify the principal maximum is 1.
const _: () = {
    assert!(PRINCIPAL_MAX_COUNT == 1);
};

/// Verify causal chain depth is reasonable.
const _: () = {
    assert!(CAUSAL_CHAIN_MAX_DEPTH > 0);
    assert!(CAUSAL_CHAIN_MAX_DEPTH <= 100_000);
};

/// Verify the constitutional identity ID is non-empty.
const _: () = {
    assert!(!CONSTITUTIONAL_IDENTITY_ID.is_empty());
};
