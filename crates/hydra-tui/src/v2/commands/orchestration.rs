//! Orchestration commands — /code (O9 Supreme Coder).
//! Separated from agent.rs to stay under 400-line limit.

use super::registry::{sys, Command, CommandCategory, CommandContext};
use crate::stream_types::StreamItem;

pub fn commands() -> Vec<Command> {
    vec![
        Command {
            name: "code",
            aliases: &["coder"],
            description: "Generate code via O9 Supreme Coder pipeline",
            args_help: "<goal description>",
            category: CommandCategory::System,
            handler: cmd_code,
        },
    ]
}

fn cmd_code(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let goal = args.trim();
    if goal.is_empty() {
        return vec![sys("Usage: /code <description of what to build>")];
    }
    let working_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .to_string_lossy()
        .to_string();
    let mut genome = hydra_genome::GenomeStore::open();
    let result = hydra_kernel::coder::code(goal, &working_dir, &mut genome);
    let mut items = vec![
        sys(&format!("Code generation complete (score: {:.1}/10)", result.score)),
        sys(&format!("  files created: {}", result.files_created)),
        sys(&format!("  tests: {}/{} passed", result.tests_passed, result.tests_passed + result.tests_failed)),
    ];
    if !result.review_issues.is_empty() {
        items.push(sys(&format!("  review issues: {}", result.review_issues.len())));
        for issue in result.review_issues.iter().take(5) {
            items.push(sys(&format!("    - {issue:?}")));
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_count() {
        assert_eq!(commands().len(), 1);
    }
}
