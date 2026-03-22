/// Minimum soul entries required for a valid succession package.
pub const MIN_SOUL_ENTRIES_FOR_SUCCESSION: usize = 1;

/// Minimum genome entries required.
pub const MIN_GENOME_ENTRIES_FOR_SUCCESSION: usize = 1;

/// Package integrity hash label.
pub const SUCCESSION_HASH_LABEL: &str = "sha256-succession";

/// Maximum days a succession package is valid (before expiry).
pub const PACKAGE_VALIDITY_DAYS: i64 = 7;

/// Maximum calibration profiles to export.
pub const MAX_CALIBRATION_PROFILES_EXPORT: usize = 1_000;

/// Maximum genome entries to export.
pub const MAX_GENOME_ENTRIES_EXPORT: usize = 100_000;
