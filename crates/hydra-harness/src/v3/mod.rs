//! Harness V3 — Orchestration Capability Testing.
//! Part A: 32 operational calibration tests (6 categories).
//! Part B: 28 real-user-day tests (7 categories).
//! Combined: 60 tests across 13 categories.

pub mod bank;
pub mod bank_ops;
pub mod bank_day;
pub mod bank_orch;
pub mod runner;
pub mod runner_direct;
pub mod runner_output;
pub mod runner_orch;
pub mod runner_orch_amm;
pub mod evaluator;
pub mod analyzer;
pub mod reporter;
