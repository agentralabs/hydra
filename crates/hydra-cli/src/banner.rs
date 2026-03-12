use crate::colors;

const HYDRA_BANNER: &str = r#"
╔══════════════════════════════════════════════════════════════╗
║  ██╗  ██╗██╗   ██╗██████╗ ██████╗  █████╗                  ║
║  ██║  ██║╚██╗ ██╔╝██╔══██╗██╔══██╗██╔══██╗                 ║
║  ███████║ ╚████╔╝ ██║  ██║██████╔╝███████║                 ║
║  ██╔══██║  ╚██╔╝  ██║  ██║██╔══██╗██╔══██║                 ║
║  ██║  ██║   ██║   ██████╔╝██║  ██║██║  ██║                 ║
║  ╚═╝  ╚═╝   ╚═╝   ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝              ║
"#;

pub fn print_banner() {
    let version = env!("CARGO_PKG_VERSION");

    // Print the ASCII art portion
    print!("{}", colors::blue(HYDRA_BANNER.trim_start_matches('\n')));

    // Version line
    println!(
        "{}",
        colors::blue(&format!(
            "║  v{:<55}║",
            format!("{} — PRODUCTION RELEASE", version)
        ))
    );

    // Separator
    println!(
        "{}",
        colors::blue("╠══════════════════════════════════════════════════════════════╣")
    );

    // Info lines
    let info_lines = [
        ("Repository", "github.com/agentralabs/agentic-hydra"),
        ("Architecture", "30+ crates, cognitive loop orchestrator"),
        ("Sisters", "memory · identity · codebase · vision · cognition"),
        ("License", "MIT"),
    ];

    for (label, value) in &info_lines {
        let content = format!("{}: {}", label, value);
        println!(
            "{}",
            colors::blue(&format!("║  {:<60}║", content))
        );
    }

    // Bottom border
    println!(
        "{}",
        colors::blue("╚══════════════════════════════════════════════════════════════╝")
    );
    println!();
}

pub fn print_banner_compact() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "  {} {} {}",
        colors::blue("◉"),
        colors::bold(&format!("Hydra v{}", version)),
        colors::dim("— agentic orchestrator")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_does_not_panic() {
        print_banner();
    }

    #[test]
    fn compact_banner_does_not_panic() {
        print_banner_compact();
    }

    #[test]
    fn banner_contains_hydra_text() {
        assert!(HYDRA_BANNER.contains("██╗  ██╗"));
    }
}
