//! /learn — parse markdown documents into genome entries + operational skills.
//! Any structured MD becomes a skill folder: genome.toml + operations.toml + assumptions.toml.

use std::path::Path;

/// Minimum text length for a bullet item to be considered knowledge.
const MIN_KNOWLEDGE_LEN: usize = 15;
/// Minimum text length for a paragraph to be considered knowledge.
const MIN_PARAGRAPH_LEN: usize = 20;
/// Default confidence for auto-learned genome entries.
const AUTO_LEARN_CONFIDENCE: f64 = 0.6;
/// Default severity for auto-learned assumption rules.
const AUTO_LEARN_SEVERITY: f64 = 0.7;
/// Maximum file size for /learn input (1MB).
const MAX_LEARN_FILE_BYTES: u64 = 1_048_576;

/// Result of learning from a markdown document.
#[derive(Debug)]
pub struct LearnResult {
    pub domain: String,
    pub knowledge_count: usize,
    pub step_count: usize,
    pub rule_count: usize,
    pub skill_dir: String,
    pub conflicts: Vec<String>,
}

/// Parse a markdown file and generate a skill from its contents.
// TODO: template variable extraction ({{name}}) from code blocks — spec aspirational
pub fn learn_from_markdown(path: &str) -> Result<LearnResult, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("Can't stat {path}: {e}"))?;
    if meta.len() > MAX_LEARN_FILE_BYTES {
        return Err(format!("File too large ({} bytes, max {})", meta.len(), MAX_LEARN_FILE_BYTES));
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Can't read {path}: {e}"))?;
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() { return Err("Empty file".into()); }

    let domain = extract_domain(&lines);
    let mut knowledge = Vec::new();
    let mut steps = Vec::new();
    let mut rules = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();

    for line in &lines {
        let trimmed = line.trim();

        // Code block handling
        if trimmed.starts_with("```") {
            if in_code_block {
                if !code_buf.trim().is_empty() {
                    steps.push(format_step(&code_lang, &code_buf));
                }
                code_buf.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                code_lang = trimmed.trim_start_matches('`').to_string();
                in_code_block = true;
            }
            continue;
        }
        if in_code_block { code_buf.push_str(line); code_buf.push('\n'); continue; }

        // Skip headings (used for domain extraction)
        if trimmed.starts_with('#') { continue; }
        // Skip empty lines
        if trimmed.is_empty() { continue; }

        // Numbered lists → operational steps
        if is_numbered_item(trimmed) {
            let text = strip_number(trimmed);
            steps.push(format!("type = \"shell\"\ncommand = \"{}\"\ndescription = \"{}\"\nneeds_review = true", escape_toml(&text), &text));
        }
        // Bullet lists → knowledge entries
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            if has_rule_language(text) {
                rules.push(text.to_string());
            } else if text.len() > MIN_KNOWLEDGE_LEN {
                knowledge.push(text.to_string());
            }
        }
        // Paragraphs → knowledge or rules
        else if trimmed.len() > MIN_PARAGRAPH_LEN {
            if has_rule_language(trimmed) {
                rules.push(trimmed.to_string());
            } else {
                knowledge.push(trimmed.to_string());
            }
        }
    }

    // Generate skill folder
    let skill_dir = format!("skills/auto-{}", domain.replace(' ', "-"));
    let dir_path = Path::new(&skill_dir);
    std::fs::create_dir_all(dir_path).map_err(|e| format!("mkdir: {e}"))?;

    // Merge: deduplicate against existing genome entries
    let conflicts = Vec::new();
    let existing_genome = if dir_path.join("genome.toml").exists() {
        std::fs::read_to_string(dir_path.join("genome.toml")).unwrap_or_default()
    } else { String::new() };
    if !existing_genome.is_empty() {
        knowledge.retain(|k| {
            let escaped = escape_toml(k);
            if existing_genome.contains(&escaped) {
                false // exact duplicate — skip
            } else {
                true
            }
        });
        if knowledge.is_empty() && steps.is_empty() && rules.is_empty() {
            return Err("No new content to learn (all entries already exist)".into());
        }
    }

    // Write genome.toml (append if existing, create if new)
    if !knowledge.is_empty() {
        let mut genome = if existing_genome.is_empty() {
            String::from("# Auto-generated from markdown\n\n")
        } else {
            format!("{existing_genome}\n")
        };
        for k in &knowledge {
            genome.push_str(&format!(
                "[[entries]]\nsituation = \"{}\"\napproach = \"{}\"\nconfidence = {}\nobservations = 1\n\n",
                escape_toml(&domain), escape_toml(k), AUTO_LEARN_CONFIDENCE
            ));
        }
        std::fs::write(dir_path.join("genome.toml"), &genome).map_err(|e| format!("write genome: {e}"))?;
    }

    // Write operations.toml
    if !steps.is_empty() {
        let mut ops = format!(
            "[[operation]]\nname = \"auto-{}\"\ntrigger = \"{}\"\nconfidence = {}\n\n",
            escape_toml(&domain), escape_toml(&domain), AUTO_LEARN_CONFIDENCE
        );
        for s in &steps {
            ops.push_str(&format!("[[operation.steps]]\n{s}\n\n"));
        }
        std::fs::write(dir_path.join("operations.toml"), &ops).map_err(|e| format!("write ops: {e}"))?;
    }

    // Write assumptions.toml
    if !rules.is_empty() {
        let mut assumptions = String::from("# Auto-generated rules\n\n");
        assumptions.push_str(&format!(
            "[[assumptions]]\ntrigger = \"{}\"\nchecks = [\n", escape_toml(&domain)
        ));
        for r in &rules {
            assumptions.push_str(&format!("  {{ statement = \"{}\", severity = {} }},\n", escape_toml(r), AUTO_LEARN_SEVERITY));
        }
        assumptions.push_str("]\n");
        std::fs::write(dir_path.join("assumptions.toml"), &assumptions).map_err(|e| format!("write assumptions: {e}"))?;
    }

    eprintln!("hydra-learn: {path} → {skill_dir} ({} knowledge, {} steps, {} rules, {} conflicts)",
        knowledge.len(), steps.len(), rules.len(), conflicts.len());

    Ok(LearnResult {
        domain, knowledge_count: knowledge.len(), step_count: steps.len(),
        rule_count: rules.len(), skill_dir, conflicts,
    })
}

fn extract_domain(lines: &[&str]) -> String {
    for line in lines {
        if let Some(heading) = line.strip_prefix("# ") {
            let raw = heading.trim().to_lowercase().replace(' ', "-");
            // Sanitize: only alphanumeric + dash to prevent path traversal
            let sanitized: String = raw.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            let trimmed = sanitized.trim_matches('-');
            return if trimmed.is_empty() { "unknown".into() } else { trimmed.to_string() };
        }
    }
    "unknown".into()
}

fn is_numbered_item(s: &str) -> bool {
    s.len() > 2 && s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && s.contains(". ")
}

fn strip_number(s: &str) -> String {
    if let Some(pos) = s.find(". ") { s[pos + 2..].to_string() } else { s.to_string() }
}

/// Detect rule/constraint language in markdown text.
/// NOTE: This is structural markdown parsing (imperative verbs, warnings),
/// NOT user intent classification. Acceptable under CLAUDE.md exception:
/// "purely in-memory, no I/O" — same pattern any markdown linter uses.
fn has_rule_language(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("always ") || lower.contains("never ") || lower.contains("must ")
        || lower.contains("don't ") || lower.contains("make sure") || lower.contains("important")
        || lower.contains("warning") || lower.contains("caution")
}

fn format_step(lang: &str, code: &str) -> String {
    match lang {
        "bash" | "sh" | "shell" | "" => format!("type = \"shell\"\ncommand = \"{}\"\ndescription = \"from markdown\"\nneeds_review = true", escape_toml(code.trim())),
        _ => format!("type = \"code_gen\"\ndescription = \"{}\"\nlanguage = \"{}\"", escape_toml(code.trim()), lang),
    }
}

fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_domain_from_heading() {
        assert_eq!(extract_domain(&["# Deploy to AWS", "some content"]), "deploy-to-aws");
    }

    #[test]
    fn detect_numbered_item() {
        assert!(is_numbered_item("1. Build the Docker image"));
        assert!(!is_numbered_item("This is a paragraph"));
    }

    #[test]
    fn detect_rule_language() {
        assert!(has_rule_language("Always use multi-stage builds"));
        assert!(has_rule_language("Never deploy on Fridays"));
        assert!(!has_rule_language("Docker is a container runtime"));
    }

    #[test]
    fn escape_toml_special_chars() {
        assert_eq!(escape_toml("hello \"world\""), "hello \\\"world\\\"");
    }

    #[test]
    fn extract_domain_sanitizes_traversal() {
        assert_eq!(extract_domain(&["# ../../../etc/passwd"]), "etcpasswd");
    }

    #[test]
    fn extract_domain_strips_special_chars() {
        assert_eq!(extract_domain(&["# Hello World! @#$"]), "hello-world");
    }
}
