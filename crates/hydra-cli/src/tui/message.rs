/// Rich message content types for the TUI conversation.
///
/// Instead of plain strings, messages carry structured segments that the
/// conversation widget renders differently: code blocks with syntax highlighting,
/// diffs with green/red lines, tool-use indicators, change summaries, and
/// live command output.

/// A single segment of message content.
#[derive(Clone, Debug)]
pub enum MessageSegment {
    /// Plain text (the default, backwards-compatible format).
    Text(String),
    /// A code block with optional syntax highlighting.
    CodeBlock {
        path: Option<String>,
        language: String,
        content: String,
        start_line: usize,
    },
    /// A unified diff for a single file.
    Diff {
        path: String,
        hunks: Vec<DiffHunk>,
    },
    /// A sister tool use indicator.
    ToolUse {
        sister: String,
        tool: String,
        args: String,
        status: ToolUseStatus,
    },
    /// Summary of multi-file changes pending approval.
    ChangeSummary {
        files: Vec<FileChange>,
        total_added: usize,
        total_removed: usize,
    },
    /// Live command output (streaming).
    CommandOutput {
        command: String,
        lines: Vec<String>,
        exit_code: Option<i32>,
        running: bool,
        elapsed_secs: f64,
    },
    /// An approval prompt rendered inline.
    ApprovalPrompt {
        action: String,
        detail: String,
    },
}

/// Status of a tool use call.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolUseStatus {
    Running,
    Complete,
    Failed,
}

/// A diff hunk (range of changed lines).
#[derive(Clone, Debug)]
pub struct DiffHunk {
    pub old_start: usize,
    pub new_start: usize,
    pub lines: Vec<DiffLine>,
}

/// A single line in a diff.
#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

/// A file change entry for multi-file edit summaries.
#[derive(Clone, Debug)]
pub struct FileChange {
    pub path: String,
    pub kind: FileChangeKind,
    pub added: usize,
    pub removed: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileChangeKind {
    Modified,
    Added,
    Deleted,
}

impl FileChangeKind {
    pub fn marker(&self) -> &'static str {
        match self {
            Self::Modified => "M",
            Self::Added => "A",
            Self::Deleted => "D",
        }
    }
}

impl MessageSegment {
    /// Create a plain text segment.
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    /// Extract the plain-text representation (for conversation history, etc.)
    pub fn to_plain_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::CodeBlock { path, content, .. } => {
                if let Some(p) = path {
                    format!("```\n// {}\n{}\n```", p, content)
                } else {
                    format!("```\n{}\n```", content)
                }
            }
            Self::Diff { path, hunks } => {
                let mut out = format!("diff {}\n", path);
                for hunk in hunks {
                    for line in &hunk.lines {
                        let prefix = match line.kind {
                            DiffLineKind::Added => "+",
                            DiffLineKind::Removed => "-",
                            DiffLineKind::Context => " ",
                        };
                        out.push_str(&format!("{}{}\n", prefix, line.content));
                    }
                }
                out
            }
            Self::ToolUse { sister, tool, args, .. } => {
                format!("Using {}: {}({})", sister, tool, args)
            }
            Self::ChangeSummary { files, total_added, total_removed } => {
                let mut out = String::from("Changes to apply:\n");
                for f in files {
                    out.push_str(&format!(
                        "  {} {} (+{} -{})\n",
                        f.kind.marker(),
                        f.path,
                        f.added,
                        f.removed
                    ));
                }
                out.push_str(&format!(
                    "Total: {} files, +{} -{}\n",
                    files.len(),
                    total_added,
                    total_removed
                ));
                out
            }
            Self::CommandOutput { command, lines, exit_code, elapsed_secs, .. } => {
                let mut out = format!("$ {}\n", command);
                for line in lines {
                    out.push_str(line);
                    out.push('\n');
                }
                if let Some(code) = exit_code {
                    out.push_str(&format!("\nExit code: {} (took {:.1}s)\n", code, elapsed_secs));
                }
                out
            }
            Self::ApprovalPrompt { action, detail } => {
                format!("{}\n{}\nApprove? [y/n]", action, detail)
            }
        }
    }
}

/// Convert a list of segments to a single plain-text string.
pub fn segments_to_text(segments: &[MessageSegment]) -> String {
    segments
        .iter()
        .map(|s| s.to_plain_text())
        .collect::<Vec<_>>()
        .join("\n")
}
