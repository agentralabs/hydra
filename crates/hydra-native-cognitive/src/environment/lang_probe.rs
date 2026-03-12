//! Language probe — detects installed programming languages and versions.

use super::os_probe::run_cmd;

/// A detected programming language.
#[derive(Debug, Clone)]
pub struct Language {
    pub name: String,
    pub version: String,
    pub binary: String,
}

impl Language {
    pub fn display(&self) -> String {
        if self.version.is_empty() {
            self.name.clone()
        } else {
            format!("{} {}", self.name, self.version)
        }
    }
}

/// Probe for all known languages.
pub fn probe_languages() -> Vec<Language> {
    let probes: Vec<(&str, &str, &[&str], fn(&str) -> String)> = vec![
        ("Rust", "rustc", &["--version"], parse_rustc),
        ("Python", "python3", &["--version"], parse_generic),
        ("Node.js", "node", &["--version"], parse_strip_v),
        ("Go", "go", &["version"], parse_go),
        ("Java", "java", &["--version"], parse_java),
        ("Ruby", "ruby", &["--version"], parse_generic),
        ("Swift", "swift", &["--version"], parse_generic),
        ("Deno", "deno", &["--version"], parse_first_line),
        ("Bun", "bun", &["--version"], parse_strip_v),
        ("Zig", "zig", &["version"], parse_first_line),
        ("Elixir", "elixir", &["--version"], parse_first_line),
    ];

    let mut langs = Vec::new();
    for (name, binary, args, parser) in probes {
        if let Some(output) = run_cmd(binary, args) {
            let version = parser(&output);
            langs.push(Language {
                name: name.to_string(),
                version,
                binary: binary.to_string(),
            });
        }
    }
    langs
}

/// Check if a specific tool/binary is available, returning its version.
pub fn check_tool(name: &str) -> Option<String> {
    // Map common names to their version commands
    let (binary, args, parser): (&str, &[&str], fn(&str) -> String) = match name {
        "rustc" | "rust" => ("rustc", &["--version"], parse_rustc),
        "python" | "python3" => ("python3", &["--version"], parse_generic),
        "node" | "nodejs" => ("node", &["--version"], parse_strip_v),
        "go" | "golang" => ("go", &["version"], parse_go),
        "java" => ("java", &["--version"], parse_java),
        "ruby" => ("ruby", &["--version"], parse_generic),
        "swift" => ("swift", &["--version"], parse_generic),
        "deno" => ("deno", &["--version"], parse_first_line),
        "bun" => ("bun", &["--version"], parse_strip_v),
        "git" => ("git", &["--version"], parse_generic),
        "docker" => ("docker", &["--version"], parse_generic),
        "gcc" | "cc" => ("gcc", &["--version"], parse_first_line),
        "clang" => ("clang", &["--version"], parse_first_line),
        "make" => ("make", &["--version"], parse_first_line),
        "cmake" => ("cmake", &["--version"], parse_generic),
        _ => (name, &["--version"], parse_first_line),
    };

    run_cmd(binary, args).map(|out| parser(&out))
}

// --- Parsers ---

fn parse_rustc(output: &str) -> String {
    // "rustc 1.77.0 (aedd173a2 2024-03-17)" → "1.77.0"
    output.split_whitespace().nth(1).unwrap_or("").to_string()
}

fn parse_generic(output: &str) -> String {
    // "Python 3.12.2" → "3.12.2"
    // Take last word that looks like a version
    output
        .split_whitespace()
        .find(|w| w.chars().next().map_or(false, |c| c.is_ascii_digit()))
        .unwrap_or("")
        .to_string()
}

fn parse_strip_v(output: &str) -> String {
    // "v20.11.0" → "20.11.0"
    let first = output.lines().next().unwrap_or("");
    first.trim().trim_start_matches('v').to_string()
}

fn parse_go(output: &str) -> String {
    // "go version go1.22.0 darwin/arm64" → "1.22.0"
    output
        .split_whitespace()
        .find(|w| w.starts_with("go1"))
        .map(|w| w.trim_start_matches("go"))
        .unwrap_or("")
        .to_string()
}

fn parse_java(output: &str) -> String {
    // java --version outputs multi-line, first line: "openjdk 21.0.2 2024-01-16"
    let first = output.lines().next().unwrap_or("");
    first
        .split_whitespace()
        .find(|w| w.chars().next().map_or(false, |c| c.is_ascii_digit()))
        .unwrap_or("")
        .to_string()
}

fn parse_first_line(output: &str) -> String {
    output.lines().next().unwrap_or("").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_languages_finds_rust() {
        let langs = probe_languages();
        assert!(
            langs.iter().any(|l| l.name == "Rust"),
            "Should detect Rust (rustc is required to build this project)"
        );
    }

    #[test]
    fn test_check_tool_rustc() {
        let version = check_tool("rustc");
        assert!(version.is_some(), "rustc should be available");
        assert!(version.unwrap().starts_with('1'), "rustc version should start with 1.x");
    }

    #[test]
    fn test_check_tool_nonexistent() {
        assert!(check_tool("nonexistent_tool_xyz_123").is_none());
    }

    #[test]
    fn test_parse_rustc() {
        assert_eq!(parse_rustc("rustc 1.77.0 (aedd173a2 2024-03-17)"), "1.77.0");
    }

    #[test]
    fn test_parse_strip_v() {
        assert_eq!(parse_strip_v("v20.11.0"), "20.11.0");
    }

    #[test]
    fn test_parse_go() {
        assert_eq!(parse_go("go version go1.22.0 darwin/arm64"), "1.22.0");
    }

    #[test]
    fn test_language_display() {
        let lang = Language { name: "Rust".into(), version: "1.77.0".into(), binary: "rustc".into() };
        assert_eq!(lang.display(), "Rust 1.77.0");
    }
}
