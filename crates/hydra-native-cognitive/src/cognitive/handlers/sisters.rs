//! Sister-related handler helpers.

/// Get sister binary info: (display_name, binary_name, args).
pub(crate) fn get_sister_bin_info() -> Vec<(&'static str, &'static str, &'static [&'static str])> {
    vec![
        ("Memory", "agentic-memory-mcp", &["serve"] as &[&str]),
        ("Identity", "agentic-identity-mcp", &["serve"]),
        ("Codebase", "agentic-codebase-mcp", &["serve"]),
        ("Vision", "agentic-vision-mcp", &["serve"]),
        ("Comm", "agentic-comm-mcp", &["serve"]),
        ("Contract", "agentic-contract-mcp", &[]),
        ("Time", "agentic-time-mcp", &["serve"]),
        ("Planning", "agentic-planning-mcp", &["serve"]),
        ("Cognition", "agentic-cognition-mcp", &[]),
        ("Reality", "agentic-reality-mcp", &[]),
        ("Forge", "agentic-forge-mcp", &[]),
        ("Aegis", "agentic-aegis-mcp", &[]),
        ("Veritas", "agentic-veritas-mcp", &[]),
        ("Evolve", "agentic-evolve-mcp", &[]),
    ]
}
