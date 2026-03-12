// ╔══════════════════════════════════════════════════════════════════╗
// ║  WRONG PLACE — This file is NOT used.                         ║
// ║  hydra-native is a LIBRARY crate (autobins = false).          ║
// ║                                                               ║
// ║  The desktop app lives at: crates/hydra-desktop/src/main.rs   ║
// ║  Run with: cargo run --bin hydra-desktop                      ║
// ╚══════════════════════════════════════════════════════════════════╝
//
// This file exists only as a signpost. It is never compiled.

fn main() {
    panic!(
        "WRONG BINARY. hydra-native is a library.\n\
         The desktop app is: cargo run --bin hydra-desktop\n\
         See crates/hydra-desktop/src/main.rs"
    );
}
