//! `hydra-harness` — Autonomous 20-hour capability test harness.
//!
//! Tests every crate, every capability, every layer.
//! Writes hourly reports. Attempts automated fixes.
//! Does not stop on failure.
//!
//! NOTE: This crate is TEST INFRASTRUCTURE — it is NOT wired into any
//! production runtime path. It provides the harness and harness_v2 binaries
//! used for structural (47/47) and behavioral (V2) testing.
//! DO NOT import hydra-harness from the kernel, TUI, or any production crate.
//! It is intentionally standalone. If test utilities are needed in production,
//! create them in the relevant crate, not here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub crate_name:    String,
    pub capability:    String,
    pub passed:        bool,
    pub duration_ms:   u64,
    pub error:         Option<String>,
    pub fix_attempted: bool,
    pub fix_succeeded: Option<bool>,
    pub fix_notes:     Option<String>,
    pub timestamp:     DateTime<Utc>,
}

impl TestResult {
    pub fn pass(crate_name: &str, capability: &str, duration_ms: u64) -> Self {
        Self {
            crate_name:    crate_name.to_string(),
            capability:    capability.to_string(),
            passed:        true,
            duration_ms,
            error:         None,
            fix_attempted: false,
            fix_succeeded: None,
            fix_notes:     None,
            timestamp:     Utc::now(),
        }
    }

    pub fn fail(crate_name: &str, capability: &str, error: &str, duration_ms: u64) -> Self {
        Self {
            crate_name:    crate_name.to_string(),
            capability:    capability.to_string(),
            passed:        false,
            duration_ms,
            error:         Some(error.to_string()),
            fix_attempted: false,
            fix_succeeded: None,
            fix_notes:     None,
            timestamp:     Utc::now(),
        }
    }
}

pub mod runner;
pub mod reporter;
pub mod fixer;
pub mod layers;
pub mod v2;
