//! Cognitive inventions — Dream State, Shadow Self, Future Echo.
//!
//! Wraps hydra-inventions types into a unified engine that the cognitive
//! loop can call at the right phase.
//!
//! Split into submodules for maintainability:
//! - `inventions_core`: struct definition, constructor, and primary methods
//! - `inventions_extras`: session momentum tracking and intelligence upgrade helpers
//! - `inventions_tests`: unit tests

#[path = "inventions_core.rs"]
mod inventions_core;

#[path = "inventions_extras.rs"]
mod inventions_extras;

#[cfg(test)]
#[path = "inventions_tests.rs"]
mod inventions_tests;

pub use inventions_core::InventionEngine;
