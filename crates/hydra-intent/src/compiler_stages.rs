use hydra_core::types::{ActionType, Entity, EntityType};
use uuid::Uuid;

use crate::compiler_types::Complexity;

/// Stage 2: Extract entities from tokens
pub(crate) fn extract_entities(text: &str, tokens: &[&str]) -> Vec<Entity> {
    let mut entities = Vec::new();

    for token in tokens {
        // URLs (check before file paths)
        if token.starts_with("http://") || token.starts_with("https://") {
            entities.push(Entity {
                id: Uuid::new_v4(),
                entity_type: EntityType::Url,
                value: token.to_string(),
                resolved_path: None,
                confidence: 0.95,
            });
            continue;
        }

        // File paths (after URLs)
        if token.contains('/') || token.contains('\\') || token.starts_with('.') {
            if token.contains('.') || token.ends_with('/') || token.starts_with("src") {
                entities.push(Entity {
                    id: Uuid::new_v4(),
                    entity_type: EntityType::FilePath,
                    value: token.to_string(),
                    resolved_path: None,
                    confidence: 0.9,
                });
                continue;
            }
        }

        // Package names (contains :: or -)
        if token.contains("::") {
            entities.push(Entity {
                id: Uuid::new_v4(),
                entity_type: EntityType::ModuleName,
                value: token.to_string(),
                resolved_path: None,
                confidence: 0.85,
            });
            continue;
        }

        // Function-like names (contains parentheses or camelCase/snake_case)
        if token.contains('(') || token.contains(')') {
            let name = token.trim_end_matches("()").trim_end_matches('(');
            entities.push(Entity {
                id: Uuid::new_v4(),
                entity_type: EntityType::FunctionName,
                value: name.to_string(),
                resolved_path: None,
                confidence: 0.8,
            });
            continue;
        }

        // File extensions (e.g., .rs, .py, .ts)
        if token.starts_with("*.") || (token.starts_with('.') && token.len() <= 5) {
            entities.push(Entity {
                id: Uuid::new_v4(),
                entity_type: EntityType::Other("file_extension".into()),
                value: token.to_string(),
                resolved_path: None,
                confidence: 0.7,
            });
            continue;
        }

        // Branch names (common git branch patterns)
        if token.starts_with("feature/")
            || token.starts_with("fix/")
            || token.starts_with("release/")
        {
            entities.push(Entity {
                id: Uuid::new_v4(),
                entity_type: EntityType::BranchName,
                value: token.to_string(),
                resolved_path: None,
                confidence: 0.9,
            });
            continue;
        }

        // Class names (PascalCase, at least 2 capital letters)
        if token.len() > 3
            && token.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && token.chars().filter(|c| c.is_uppercase()).count() >= 2
            && token.chars().any(|c| c.is_lowercase())
            && token.chars().all(|c| c.is_alphanumeric())
        {
            entities.push(Entity {
                id: Uuid::new_v4(),
                entity_type: EntityType::ClassName,
                value: token.to_string(),
                resolved_path: None,
                confidence: 0.6,
            });
        }
    }

    // Also extract file paths from the full text using regex-like patterns
    let words: Vec<&str> = text.split_whitespace().collect();
    for window in words.windows(1) {
        let w = window[0];
        // Detect paths like "src/main.rs" even without leading ./
        if w.contains('/')
            && (w.ends_with(".rs")
                || w.ends_with(".ts")
                || w.ends_with(".py")
                || w.ends_with(".js")
                || w.ends_with(".go")
                || w.ends_with(".java"))
            && !entities.iter().any(|e| e.value == w)
        {
            entities.push(Entity {
                id: Uuid::new_v4(),
                entity_type: EntityType::FilePath,
                value: w.to_string(),
                resolved_path: None,
                confidence: 0.9,
            });
        }
    }

    entities
}

/// Stage 3: Classify the primary action type
pub(crate) fn classify_action(text: &str, _tokens: &[&str], entities: &[Entity]) -> ActionType {
    let lower = text.to_lowercase();

    // Destructive actions
    if lower.contains("delete") || lower.contains("remove") || lower.contains("drop") {
        return ActionType::FileDelete;
    }

    // System commands
    if lower.contains("deploy")
        || lower.contains("restart")
        || lower.contains("shutdown")
        || lower.contains("install")
    {
        return ActionType::System;
    }

    // Network actions
    if lower.contains("send") || lower.contains("email") || lower.contains("webhook") {
        return ActionType::Network;
    }

    // Git operations
    if lower.contains("commit")
        || lower.contains("push")
        || lower.contains("merge")
        || lower.contains("rebase")
    {
        return ActionType::GitOperation;
    }

    // Execute/run
    if lower.contains("run") || lower.contains("execute") || lower.contains("test") {
        return ActionType::Execute;
    }

    // Code generation/modification
    if lower.contains("create") || lower.contains("generate") || lower.contains("implement") {
        if entities
            .iter()
            .any(|e| matches!(e.entity_type, EntityType::FilePath))
        {
            return ActionType::FileCreate;
        }
        return ActionType::Write;
    }

    // Modification
    if lower.contains("edit")
        || lower.contains("modify")
        || lower.contains("refactor")
        || lower.contains("fix")
    {
        return ActionType::FileModify;
    }

    // Default: Read (safest)
    ActionType::Read
}

/// Stage 4: Assess complexity of the intent
pub(crate) fn assess_complexity(
    text: &str,
    tokens: &[&str],
    entities: &[Entity],
    action_type: &ActionType,
) -> Complexity {
    let mut score: u32 = 0;

    // Token count contributes to complexity
    if tokens.len() > 20 {
        score += 2;
    } else if tokens.len() > 10 {
        score += 1;
    }

    // Multiple entities = more complex
    if entities.len() > 3 {
        score += 2;
    } else if entities.len() > 1 {
        score += 1;
    }

    // Destructive/system actions are inherently more complex
    match action_type {
        ActionType::System => score += 3,
        ActionType::FileDelete => score += 2,
        ActionType::Network => score += 2,
        ActionType::Execute | ActionType::ShellExecute => score += 1,
        ActionType::Composite => score += 3,
        _ => {}
    }

    // Multi-step indicators
    let lower = text.to_lowercase();
    if lower.contains(" and ") || lower.contains(" then ") || lower.contains(" also ") {
        score += 2;
    }
    if lower.contains("all") || lower.contains("every") || lower.contains("each") {
        score += 1;
    }

    // Conditional logic
    if lower.contains(" if ") || lower.contains("unless") || lower.contains("when") {
        score += 1;
    }

    match score {
        0..=1 => Complexity::Simple,
        2..=3 => Complexity::Moderate,
        4..=6 => Complexity::Complex,
        _ => Complexity::Critical,
    }
}

/// Estimate number of execution steps from complexity
pub(crate) fn estimate_steps(complexity: &Complexity) -> usize {
    match complexity {
        Complexity::Simple => 1,
        Complexity::Moderate => 2,
        Complexity::Complex => 4,
        Complexity::Critical => 8,
    }
}
