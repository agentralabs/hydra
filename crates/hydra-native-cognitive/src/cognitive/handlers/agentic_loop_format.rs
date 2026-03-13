//! Agentic loop formatting — tool result formatting and completion detection.

use hydra_native_state::utils::safe_truncate;

/// Format tool results + exec results into a follow-up message for the LLM.
///
/// This message is appended to the conversation so the LLM can see what
/// its tool calls returned and decide what to do next.
pub(crate) fn format_tool_results_message(
    tool_results: &[(String, String)],
    exec_results: &[(String, String, bool)],
) -> String {
    let mut msg = String::with_capacity(2048);

    if !tool_results.is_empty() {
        msg.push_str("## Tool Results\n\n");
        for (name, output) in tool_results {
            msg.push_str(&format!(
                "### {}\n```\n{}\n```\n\n",
                name, safe_truncate(output, 2000)
            ));
        }
    }

    if !exec_results.is_empty() {
        msg.push_str("## Command Results\n\n");
        for (cmd, output, success) in exec_results {
            let status = if *success { "OK" } else { "FAILED" };
            msg.push_str(&format!(
                "### `{}` [{}]\n```\n{}\n```\n\n",
                safe_truncate(cmd, 200),
                status,
                safe_truncate(output, 2000)
            ));
        }
    }

    msg.push_str(
        "Based on these results, continue your work. \
         If you need more tool calls, use <hydra-tool> or <hydra-exec> tags. \
         If the task is complete, include <hydra-done/> at the end of your response."
    );

    msg
}

/// Check if LLM response contains actionable tags (<hydra-tool> or <hydra-exec>).
pub(crate) fn has_actionable_tags(response: &str) -> bool {
    response.contains("<hydra-tool") || response.contains("<hydra-exec>")
}

/// Detect if the LLM signals task completion.
pub(crate) fn is_task_complete(response: &str) -> bool {
    response.contains("<hydra-done")
        || response.contains("<hydra-done/>")
        || response.contains("</hydra-done>")
}

/// Strip <hydra-done/> tags from response for clean display.
pub(crate) fn strip_done_tag(text: &str) -> String {
    text.replace("<hydra-done/>", "")
        .replace("<hydra-done>", "")
        .replace("</hydra-done>", "")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tool_results() {
        let tool_results = vec![
            ("memory_query".into(), "Found 3 memories".into()),
        ];
        let exec_results = vec![
            ("ls -la".into(), "total 42\nfoo.rs".into(), true),
        ];
        let msg = format_tool_results_message(&tool_results, &exec_results);
        assert!(msg.contains("memory_query"));
        assert!(msg.contains("Found 3 memories"));
        assert!(msg.contains("ls -la"));
        assert!(msg.contains("[OK]"));
        assert!(msg.contains("<hydra-done/>"));
    }

    #[test]
    fn test_has_actionable_tags() {
        assert!(has_actionable_tags(r#"<hydra-tool name="x">{}</hydra-tool>"#));
        assert!(has_actionable_tags("<hydra-exec>ls</hydra-exec>"));
        assert!(!has_actionable_tags("plain text response"));
    }

    #[test]
    fn test_is_task_complete() {
        assert!(is_task_complete("Done! <hydra-done/>"));
        assert!(!is_task_complete("Still working..."));
    }

    #[test]
    fn test_strip_done_tag() {
        assert_eq!(strip_done_tag("All done <hydra-done/>"), "All done");
    }

    #[test]
    fn test_format_empty_results() {
        let msg = format_tool_results_message(&[], &[]);
        assert!(msg.contains("Based on these results"));
    }

    #[test]
    fn test_format_failed_command() {
        let exec = vec![("cargo test".into(), "error[E0308]".into(), false)];
        let msg = format_tool_results_message(&[], &exec);
        assert!(msg.contains("[FAILED]"));
        assert!(msg.contains("error[E0308]"));
    }
}
