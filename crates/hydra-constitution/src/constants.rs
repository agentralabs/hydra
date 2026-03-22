//! All constants for hydra-constitution.
//! No magic numbers or strings anywhere else in this crate.

/// Trust tier levels. Lower number = higher authority.
pub const TRUST_TIER_CONSTITUTION: u8 = 0;
pub const TRUST_TIER_HYDRA: u8 = 1;
pub const TRUST_TIER_PRINCIPAL: u8 = 2;
pub const TRUST_TIER_FLEET: u8 = 3;
pub const TRUST_TIER_SKILLS: u8 = 4;
pub const TRUST_TIER_EXTERNAL: u8 = 5;
pub const TRUST_TIER_COUNT: u8 = 6;
pub const TRUST_TIER_MINIMUM: u8 = TRUST_TIER_CONSTITUTION;
pub const TRUST_TIER_MAXIMUM: u8 = TRUST_TIER_EXTERNAL;

/// Animus Prime binary format header.
pub const ANIMUS_MAGIC: &[u8; 4] = b"ANMA";
pub const ANIMUS_VERSION: u32 = 0x00_01_00_00;

/// The root of every causal chain. Every chain must terminate here.
pub const CONSTITUTIONAL_IDENTITY_ID: &str = "00000000-0000-0000-0000-000000000001";

/// Maximum causal chain depth before considered malformed.
pub const CAUSAL_CHAIN_MAX_DEPTH: usize = 10_000;

/// Law 3: memory revision always requires provenance.
pub const MEMORY_REVISION_REQUIRES_PROVENANCE: bool = true;

/// Law 6: exactly one principal permitted at any time.
pub const PRINCIPAL_MAX_COUNT: usize = 1;

/// Maximum receipt chain depth.
pub const RECEIPT_CHAIN_MAX_DEPTH: usize = 100_000;

/// Reserved identity strings that no agent may claim.
pub const RESERVED_IDENTITIES: &[&str] = &[
    "hydra",
    "hydra-kernel",
    "hydra-constitution",
    "hydra-principal",
    "constitutional-identity",
];

/// Action prefixes that are never permitted on receipts.
pub const RECEIPT_MUTATION_PREFIXES: &[&str] = &[
    "receipt.delete",
    "receipt.modify",
    "receipt.suppress",
    "receipt.overwrite",
    "receipt.truncate",
    "receipt.clear",
    "receipt.purge",
];

/// Action prefixes that constitute constitution access attempts.
pub const CONSTITUTION_ACCESS_ACTIONS: &[&str] = &[
    "constitution.modify",
    "constitution.patch",
    "constitution.read_internal",
    "constitution.bypass",
    "constitution.override",
];
