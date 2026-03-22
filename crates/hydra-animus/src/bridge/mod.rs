//! The script bridge: converts between human-readable text and Animus Prime.

pub mod from_prime;
pub mod to_prime;

pub use from_prime::{graph_to_text, HumanReadable};
pub use to_prime::{text_to_signal, ResolvedIntent};
