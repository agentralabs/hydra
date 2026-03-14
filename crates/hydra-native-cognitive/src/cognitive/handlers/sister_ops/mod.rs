//! Sister operations and self-repair dispatch handlers — extracted from loop_runner.rs.
//!
//! Contains the larger intent handlers: self-repair, omniscience scan, self-implement,
//! sister diagnostics, and sister repair.

mod scan_repair;
mod implement_diagnose;
mod diagnostics;
mod improve_threat;
mod sister_repair_diagnosis;
mod sister_repair_handler;
mod build_system;
mod build_phases;
mod implement_new_project;

pub(crate) use scan_repair::{handle_self_repair, handle_omniscience_scan};
pub(crate) use implement_diagnose::handle_self_implement;
pub(crate) use diagnostics::handle_sister_diagnose;
pub(crate) use improve_threat::{handle_sister_improve, handle_threat_query};
pub(crate) use sister_repair_handler::handle_sister_repair;
pub(crate) use build_system::handle_build_system;
