//! Formatting utilities — language detection, byte formatting.

/// Detect programming language from file extension.
pub fn detect_language(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "rs" => "Rust",
        "js" => "JavaScript",
        "ts" => "TypeScript",
        "tsx" => "TypeScript (React)",
        "jsx" => "JavaScript (React)",
        "py" => "Python",
        "rb" => "Ruby",
        "go" => "Go",
        "java" => "Java",
        "kt" => "Kotlin",
        "swift" => "Swift",
        "c" => "C",
        "cpp" | "cc" | "cxx" => "C++",
        "h" | "hpp" => "C/C++ Header",
        "cs" => "C#",
        "php" => "PHP",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "SCSS",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "xml" => "XML",
        "sql" => "SQL",
        "sh" | "bash" | "zsh" => "Shell",
        "md" | "markdown" => "Markdown",
        "dockerfile" => "Dockerfile",
        "proto" => "Protobuf",
        "graphql" | "gql" => "GraphQL",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "dart" => "Dart",
        "r" => "R",
        "lua" => "Lua",
        "ex" | "exs" => "Elixir",
        "erl" => "Erlang",
        "zig" => "Zig",
        "nim" => "Nim",
        "lock" => "Lock File",
        "env" => "Environment",
        "gitignore" => "Git Config",
        _ => "Other",
    }
}

/// Format byte count as human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("main.rs"), "Rust");
        assert_eq!(detect_language("index.ts"), "TypeScript");
        assert_eq!(detect_language("App.tsx"), "TypeScript (React)");
        assert_eq!(detect_language("style.css"), "CSS");
        assert_eq!(detect_language("unknown.xyz"), "Other");
        assert_eq!(detect_language("Dockerfile"), "Other"); // no extension
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }
}
