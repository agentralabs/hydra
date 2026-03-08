//! Input sanitization — protects against injection attacks

/// Known dangerous shell patterns
const SHELL_PATTERNS: &[&str] = &[
    "$(",
    "`",
    "&&",
    "||",
    ";",
    "|",
    ">",
    "<",
    ">>",
    "rm -rf",
    "rm -f",
    "mkfs",
    "dd if=",
    "chmod 777",
    "curl|sh",
    "wget|sh",
];

/// Known SQL injection patterns
const SQL_PATTERNS: &[&str] = &[
    "'; drop",
    "'; delete",
    "'; update",
    "'; insert",
    "1=1",
    "or 1=1",
    "' or '",
    "union select",
    "--",
    "/*",
    "*/",
];

/// Known prompt injection patterns
const PROMPT_PATTERNS: &[&str] = &[
    "ignore previous",
    "ignore all previous",
    "forget your instructions",
    "you are now",
    "act as",
    "pretend you are",
    "disregard",
    "override",
    "new instructions",
];

/// Check if input contains potential shell injection
pub fn has_shell_injection(text: &str) -> bool {
    let lower = text.to_lowercase();
    SHELL_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Check if input contains potential SQL injection
pub fn has_sql_injection(text: &str) -> bool {
    let lower = text.to_lowercase();
    SQL_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Check if input contains potential prompt injection
pub fn has_prompt_injection(text: &str) -> bool {
    let lower = text.to_lowercase();
    PROMPT_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Check if input contains any dangerous patterns
pub fn has_dangerous_patterns(text: &str) -> bool {
    has_shell_injection(text) || has_sql_injection(text)
}

/// Check if the input is safe (no injection attempts detected)
pub fn is_safe(text: &str) -> bool {
    !has_dangerous_patterns(text) && !has_prompt_injection(text)
}

/// Check if input is ambiguous (too vague to classify)
pub fn is_ambiguous(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    // Very short inputs with no actionable keywords
    if words.len() <= 2 {
        let vague = ["do", "thing", "stuff", "it", "something", "that", "this"];
        return words
            .iter()
            .all(|w| vague.contains(&w.to_lowercase().as_str()));
    }
    false
}

/// Check if input contains contradictory instructions
pub fn has_contradiction(text: &str) -> bool {
    let lower = text.to_lowercase();
    let contradictions = [
        ("create", "delete"),
        ("add", "remove"),
        ("start", "stop"),
        ("open", "close"),
        ("enable", "disable"),
        ("increase", "decrease"),
    ];
    contradictions
        .iter()
        .any(|(a, b)| lower.contains(a) && lower.contains(b))
}

/// Maximum input length (100K chars)
pub const MAX_INPUT_LENGTH: usize = 100_000;

/// Truncate input to safe length
pub fn truncate_if_needed(text: &str) -> &str {
    if text.len() > MAX_INPUT_LENGTH {
        // Find a safe char boundary
        let mut end = MAX_INPUT_LENGTH;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[..end]
    } else {
        text
    }
}
