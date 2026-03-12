//! Missing tool detector — parses error output to identify missing tools/dependencies.

/// A detected missing tool or dependency.
#[derive(Debug, Clone, PartialEq)]
pub struct MissingTool {
    pub name: String,
    pub source: DetectionSource,
    pub ecosystem: ToolEcosystem,
}

/// How the missing tool was detected.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetectionSource {
    /// Detected from error output (command not found, etc.)
    ErrorOutput,
    /// Detected from build/compile output
    BuildOutput,
    /// Explicitly requested by user
    UserRequest,
}

/// Which ecosystem a tool belongs to (determines package manager).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolEcosystem {
    System,     // brew/apt — general system tools
    Rust,       // cargo install
    Python,     // pip install
    JavaScript, // npm install -g
    Go,         // go install
    Ruby,       // gem install
    Unknown,
}

impl MissingTool {
    /// Get the correct package name for a given package manager.
    pub fn package_name(&self, pm_name: &str) -> String {
        package_name_for(&self.name, pm_name)
    }
}

/// Detect a missing tool from error output.
pub fn detect_missing(error: &str) -> Option<MissingTool> {
    let lower = error.to_lowercase();

    // "command not found: jq" or "jq: command not found"
    if let Some(name) = detect_command_not_found(&lower) {
        return Some(MissingTool {
            ecosystem: classify_tool_ecosystem(&name),
            name,
            source: DetectionSource::ErrorOutput,
        });
    }

    // "error: linker `cc` not found" (Rust build)
    if let Some(name) = detect_linker_missing(&lower, error) {
        return Some(MissingTool {
            name,
            source: DetectionSource::BuildOutput,
            ecosystem: ToolEcosystem::System,
        });
    }

    // "No such file or directory: python3" or "python3: No such file"
    if let Some(name) = detect_no_such_file(&lower) {
        return Some(MissingTool {
            ecosystem: classify_tool_ecosystem(&name),
            name,
            source: DetectionSource::ErrorOutput,
        });
    }

    // Rust: "could not find crate `serde`"
    if let Some(name) = detect_missing_crate(&lower) {
        return Some(MissingTool {
            name,
            source: DetectionSource::BuildOutput,
            ecosystem: ToolEcosystem::Rust,
        });
    }

    // Python: "ModuleNotFoundError: No module named 'requests'"
    if let Some(name) = detect_python_module(&lower) {
        return Some(MissingTool {
            name,
            source: DetectionSource::BuildOutput,
            ecosystem: ToolEcosystem::Python,
        });
    }

    // Node: "Cannot find module 'express'"
    if let Some(name) = detect_node_module(&lower, error) {
        return Some(MissingTool {
            name,
            source: DetectionSource::BuildOutput,
            ecosystem: ToolEcosystem::JavaScript,
        });
    }

    None
}

fn detect_command_not_found(lower: &str) -> Option<String> {
    // "command not found: toolname" (zsh)
    if let Some(idx) = lower.find("command not found:") {
        let rest = &lower[idx + 18..];
        return extract_first_word(rest);
    }
    // "toolname: command not found" (bash)
    if lower.contains(": command not found") {
        let parts: Vec<&str> = lower.split(": command not found").collect();
        if let Some(before) = parts.first() {
            return extract_last_word(before);
        }
    }
    None
}

fn detect_linker_missing(lower: &str, _original: &str) -> Option<String> {
    if lower.contains("linker") && lower.contains("not found") {
        // Common linker tools
        if lower.contains("`cc`") || lower.contains("'cc'") {
            return Some("gcc".to_string());
        }
        if lower.contains("`ld`") || lower.contains("'ld'") {
            return Some("binutils".to_string());
        }
        return Some("build-essential".to_string());
    }
    None
}

fn detect_no_such_file(lower: &str) -> Option<String> {
    if lower.contains("no such file or directory") {
        for line in lower.lines() {
            if line.contains("no such file or directory") {
                // "bash: python3: no such file or directory"
                // "bash: /usr/bin/python3: no such file or directory"
                let before = line.split("no such file or directory").next()?.trim();
                // Split by ':' and take the last non-empty part before the error
                let parts: Vec<&str> = before.split(':')
                    .map(|p| p.trim())
                    .filter(|p| !p.is_empty())
                    .collect();
                if let Some(last) = parts.last() {
                    let name = last.rsplit('/').next().unwrap_or(last).trim();
                    if !name.is_empty() && name.len() < 30 {
                        return Some(name.to_string());
                    }
                }
            }
        }
    }
    None
}

fn detect_missing_crate(lower: &str) -> Option<String> {
    // "could not find `serde` in the registry"
    if lower.contains("could not find") && (lower.contains("crate") || lower.contains("registry")) {
        return extract_backtick_content(lower);
    }
    // "unresolved import `foo`"
    if lower.contains("unresolved import") {
        if let Some(name) = extract_backtick_content(lower) {
            // Take just the top-level crate name
            return Some(name.split("::").next().unwrap_or(&name).to_string());
        }
    }
    None
}

fn detect_python_module(lower: &str) -> Option<String> {
    // "ModuleNotFoundError: No module named 'requests'"
    if lower.contains("no module named") {
        return extract_quoted_content(lower);
    }
    None
}

fn detect_node_module(lower: &str, original: &str) -> Option<String> {
    // "Cannot find module 'express'"
    if lower.contains("cannot find module") {
        return extract_quoted_content(original);
    }
    None
}

fn classify_tool_ecosystem(name: &str) -> ToolEcosystem {
    match name {
        "cargo" | "rustc" | "rustup" | "cargo-watch" | "cargo-edit" => ToolEcosystem::Rust,
        "python" | "python3" | "pip" | "pip3" | "pipenv" | "poetry" => ToolEcosystem::Python,
        "node" | "npm" | "npx" | "yarn" | "pnpm" | "bun" | "deno" => ToolEcosystem::JavaScript,
        "go" | "gofmt" => ToolEcosystem::Go,
        "ruby" | "gem" | "bundle" | "bundler" => ToolEcosystem::Ruby,
        _ => ToolEcosystem::System,
    }
}

/// Extract content between backticks: `foo` → "foo"
fn extract_backtick_content(s: &str) -> Option<String> {
    let start = s.find('`')? + 1;
    let end = s[start..].find('`')? + start;
    let content = s[start..end].trim();
    if content.is_empty() { None } else { Some(content.to_string()) }
}

/// Extract content between single quotes: 'foo' → "foo"
fn extract_quoted_content(s: &str) -> Option<String> {
    let start = s.find('\'')? + 1;
    let end = s[start..].find('\'')? + start;
    let content = s[start..end].trim();
    if content.is_empty() { None } else { Some(content.to_string()) }
}

fn extract_first_word(s: &str) -> Option<String> {
    let word = s.trim().split_whitespace().next()?;
    let clean: String = word.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
    if clean.is_empty() { None } else { Some(clean) }
}

fn extract_last_word(s: &str) -> Option<String> {
    let word = s.trim().rsplit_once(|c: char| c.is_whitespace() || c == ':' || c == '\n')
        .map(|(_, w)| w)
        .unwrap_or(s.trim());
    let clean: String = word.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
    if clean.is_empty() { None } else { Some(clean) }
}

/// Map a tool name to the correct package name for a specific package manager.
pub fn package_name_for(tool: &str, pm: &str) -> String {
    match (tool, pm) {
        // System tools with different names across PMs
        ("gcc", "apt") => "build-essential".to_string(),
        ("gcc", _) => "gcc".to_string(),
        ("build-essential", "brew") => "gcc".to_string(),
        ("python3", "brew") => "python@3".to_string(),
        ("python3", _) => "python3".to_string(),
        ("python", "brew") => "python@3".to_string(),
        ("node", "apt") => "nodejs".to_string(),
        ("node", _) => "node".to_string(),
        ("java", "brew") => "openjdk".to_string(),
        ("java", "apt") => "default-jdk".to_string(),
        ("binutils", "brew") => "binutils".to_string(),
        // Rust tools → always cargo
        ("cargo-watch", _) => "cargo-watch".to_string(),
        ("cargo-edit", _) => "cargo-edit".to_string(),
        // Default: same name
        _ => tool.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_command_not_found_zsh() {
        let tool = detect_missing("zsh: command not found: jq").unwrap();
        assert_eq!(tool.name, "jq");
        assert_eq!(tool.source, DetectionSource::ErrorOutput);
    }

    #[test]
    fn test_detect_command_not_found_bash() {
        let tool = detect_missing("jq: command not found").unwrap();
        assert_eq!(tool.name, "jq");
    }

    #[test]
    fn test_detect_linker_missing() {
        let tool = detect_missing("error: linker `cc` not found").unwrap();
        assert_eq!(tool.name, "gcc");
        assert_eq!(tool.ecosystem, ToolEcosystem::System);
    }

    #[test]
    fn test_detect_python_missing() {
        let tool = detect_missing("bash: python3: No such file or directory").unwrap();
        assert_eq!(tool.name, "python3");
    }

    #[test]
    fn test_detect_missing_crate() {
        let tool = detect_missing("could not find `serde` in the registry").unwrap();
        assert_eq!(tool.name, "serde");
        assert_eq!(tool.ecosystem, ToolEcosystem::Rust);
    }

    #[test]
    fn test_detect_python_module() {
        let tool = detect_missing("ModuleNotFoundError: No module named 'requests'").unwrap();
        assert_eq!(tool.name, "requests");
        assert_eq!(tool.ecosystem, ToolEcosystem::Python);
    }

    #[test]
    fn test_detect_node_module() {
        let tool = detect_missing("Error: Cannot find module 'express'").unwrap();
        assert_eq!(tool.name, "express");
        assert_eq!(tool.ecosystem, ToolEcosystem::JavaScript);
    }

    #[test]
    fn test_detect_none_for_unknown() {
        assert!(detect_missing("something went wrong").is_none());
    }

    #[test]
    fn test_package_name_mapping() {
        assert_eq!(package_name_for("gcc", "apt"), "build-essential");
        assert_eq!(package_name_for("gcc", "brew"), "gcc");
        assert_eq!(package_name_for("python3", "brew"), "python@3");
        assert_eq!(package_name_for("node", "apt"), "nodejs");
        assert_eq!(package_name_for("jq", "brew"), "jq"); // default
    }

    #[test]
    fn test_classify_ecosystem() {
        assert_eq!(classify_tool_ecosystem("cargo"), ToolEcosystem::Rust);
        assert_eq!(classify_tool_ecosystem("python3"), ToolEcosystem::Python);
        assert_eq!(classify_tool_ecosystem("npm"), ToolEcosystem::JavaScript);
        assert_eq!(classify_tool_ecosystem("jq"), ToolEcosystem::System);
    }

    #[test]
    fn test_tool_package_name() {
        let tool = MissingTool {
            name: "gcc".into(),
            source: DetectionSource::BuildOutput,
            ecosystem: ToolEcosystem::System,
        };
        assert_eq!(tool.package_name("apt"), "build-essential");
        assert_eq!(tool.package_name("brew"), "gcc");
    }
}
