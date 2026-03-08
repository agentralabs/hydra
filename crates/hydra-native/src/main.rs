//! Hydra Native Desktop — Dioxus entry point.
//!
//! Build with: cargo build -p hydra-native --features desktop
//! Run with:   cargo run -p hydra-native --features desktop

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::prelude::*;
        println!("Hydra Desktop v0.1.0 — starting...");
        // Dioxus desktop launch would go here:
        // dioxus::LaunchBuilder::desktop().launch(App);
        println!("Hydra Desktop requires Dioxus desktop launch. Build with --features desktop.");
    }

    #[cfg(not(feature = "desktop"))]
    {
        eprintln!("hydra-native: desktop feature not enabled.");
        eprintln!("Build with: cargo run -p hydra-native --features desktop");
        std::process::exit(1);
    }
}
