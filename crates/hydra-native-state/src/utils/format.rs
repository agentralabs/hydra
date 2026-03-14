//! Formatting utilities — language detection, byte formatting, safe string truncation.

/// Safely truncate a string at `max_bytes`, ensuring the cut falls on a UTF-8 char boundary.
/// Returns the original string if it fits within `max_bytes`.
pub fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

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

/// Strip emoji characters from text — code-level enforcement.
///
/// Removes Unicode emoji ranges so LLM responses never contain emojis,
/// regardless of what the model generates. This is more reliable than
/// prompt-level instructions which models sometimes ignore.
pub fn strip_emojis(s: &str) -> String {
    s.chars().filter(|c| !is_emoji(*c)).collect()
}

/// Check if a character is an emoji.
fn is_emoji(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        // Emoticons, Dingbats, Symbols, Transport/Map, Misc Symbols
        0x2600..=0x27BF |
        // CJK Symbols (some emoji live here)
        0x2B50..=0x2B55 |
        // Misc Symbols & Pictographs
        0x1F300..=0x1F5FF |
        // Emoticons
        0x1F600..=0x1F64F |
        // Transport & Map Symbols
        0x1F680..=0x1F6FF |
        // Supplemental Symbols & Pictographs
        0x1F900..=0x1F9FF |
        // Symbols & Pictographs Extended-A
        0x1FA00..=0x1FA6F |
        // Symbols & Pictographs Extended-B
        0x1FA70..=0x1FAFF |
        // Flags
        0x1F1E0..=0x1F1FF |
        // Variation selectors (emoji modifiers)
        0xFE00..=0xFE0F |
        // Zero-width joiner (used in compound emojis)
        0x200D |
        // Skin tone modifiers
        0x1F3FB..=0x1F3FF
    )
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
    fn test_strip_emojis() {
        assert_eq!(strip_emojis("Hello world"), "Hello world");
        assert_eq!(strip_emojis("Hello 🌍 world"), "Hello  world");
        assert_eq!(strip_emojis("Great job! 🎉👍"), "Great job! ");
        assert_eq!(strip_emojis("No emojis here."), "No emojis here.");
        assert_eq!(strip_emojis("🚀 Launch"), " Launch");
        assert_eq!(strip_emojis("Code ✨ clean"), "Code  clean");
        // Preserves non-emoji unicode
        assert_eq!(strip_emojis("café résumé"), "café résumé");
    }

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

/// Cross-platform home directory. Checks HOME (Unix), USERPROFILE (Windows), falls back to ".".
/// Use this instead of `std::env::var("HOME")` everywhere.
pub fn home_dir() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into())
}

/// Cross-platform hydra data directory (~/.hydra). Auto-creates if missing.
pub fn hydra_data_dir() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(home_dir()).join(".hydra");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
