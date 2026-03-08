//! Hydra Native Desktop — slim entry point.
//!
//! Build with: cargo build -p hydra-native --features desktop
//! Run with:   cargo run -p hydra-native --features desktop

fn main() {
    #[cfg(feature = "desktop")]
    hydra_native::desktop::launch();

    #[cfg(not(feature = "desktop"))]
    {
        eprintln!("hydra-native: desktop feature not enabled.");
        eprintln!("Build with: cargo run -p hydra-native --features desktop");
        std::process::exit(1);
    }
}
