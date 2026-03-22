//! All constants for hydra-trust.
//! No magic numbers or strings anywhere else in this crate.

/// Minimum trust score (absolute floor).
pub const TRUST_SCORE_MIN: f64 = 0.0;

/// Maximum trust score (absolute ceiling).
pub const TRUST_SCORE_MAX: f64 = 1.0;

/// Default trust score for new agents.
pub const TRUST_SCORE_DEFAULT: f64 = 0.5;

/// Fleet trust threshold: below this, agent is considered unreliable.
pub const T_FLEET_UNRELIABLE: f64 = 0.3;

/// Fleet trust threshold: above this, agent is considered trusted.
pub const T_FLEET_TRUSTED: f64 = 0.7;

/// Fleet trust threshold: above this, agent is considered highly trusted.
pub const T_FLEET_HIGHLY_TRUSTED: f64 = 0.9;

/// Spike magnitude applied on constitutional violation.
pub const CONSTITUTIONAL_VIOLATION_SPIKE: f64 = 0.5;

/// Trust recovery rate per success event.
pub const TRUST_RECOVERY_RATE: f64 = 0.02;

/// Trust penalty rate per failure event.
pub const TRUST_PENALTY_RATE: f64 = 0.05;

/// Boltzmann constant for trust thermodynamics.
pub const BOLTZMANN_K: f64 = 1.0;

/// Energy for the External tier.
pub const ENERGY_EXTERNAL: f64 = 5.0;

/// Energy for the Skills tier.
pub const ENERGY_SKILLS: f64 = 4.0;

/// Energy for the Fleet tier.
pub const ENERGY_FLEET: f64 = 3.0;

/// Energy for the Principal tier.
pub const ENERGY_PRINCIPAL: f64 = 2.0;

/// Energy for the Hydra tier.
pub const ENERGY_HYDRA: f64 = 1.0;

/// Energy for the Constitution tier.
pub const ENERGY_CONSTITUTION: f64 = 0.0;

/// Maximum number of agents in a trust field.
pub const MAX_AGENTS: usize = 1024;

/// Number of consecutive failures before auto-quarantine.
pub const QUARANTINE_FAILURE_THRESHOLD: u32 = 5;

/// Temperature for Boltzmann computation (fleet default).
pub const DEFAULT_TEMPERATURE: f64 = 1.0;
