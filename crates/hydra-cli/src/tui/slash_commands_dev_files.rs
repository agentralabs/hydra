//! Slash commands — developer file operations (/files, /open, /edit, /search, /symbols, /impact).

use crate::tui::project;
use super::app::{App, Message, MessageRole};

impl App {
    pub(crate) fn slash_cmd_files(&mut self, args: &str, timestamp: &str) {
        let dir = std::path::Path::new(&self.working_dir);
        let depth: usize = args.parse().unwrap_or(2);
        let entries = project::list_files(dir, depth);
        let mut content = format!("Project files (depth {}):\n\n", depth);
        if entries.is_empty() {
            content.push_str("  (no files found)");
        } else {
            for entry in entries.iter().take(200) {
                content.push_str(entry);
                content.push('\n');
            }
            if entries.len() > 200 {
                content.push_str(&format!("  ... and {} more", entries.len() - 200));
            }
        }
        self.messages.push(Message {
            role: MessageRole::System,
            content,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_open(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /open <file_path>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            let file_path = if args.starts_with('/') {
                std::path::PathBuf::from(args)
            } else {
                std::path::Path::new(&self.working_dir).join(args)
            };
            match project::read_file_with_lines(&file_path) {
                Ok((content, language)) => {
                    let line_count = content.lines().count();
                    let display: String = content.lines()
                        .take(100)
                        .enumerate()
                        .map(|(i, l)| format!("{:>4} | {}", i + 1, l))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let mut msg = format!(
                        "--- {} ({}, {} lines) ---\n{}",
                        file_path.display(), language, line_count, display
                    );
                    if line_count > 100 {
                        msg.push_str(&format!("\n\n... {} more lines (use /open {} <offset>)", line_count - 100, args));
                    }
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: msg,
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                Err(e) => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: e,
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
        }
    }

    pub(crate) fn slash_cmd_edit(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /edit <file_path>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| "vim".to_string());
            let file_path = if args.starts_with('/') {
                args.to_string()
            } else {
                format!("{}/{}", self.working_dir, args)
            };
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Opening {} in {}...", file_path, editor),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            // Spawn the editor outside the TUI context
            // The TUI will need to suspend — for now just show instruction
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Run: {} {}", editor, file_path),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_search(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /search <term>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            // Fast grep search
            let dir = &self.working_dir;
            let output = std::process::Command::new("grep")
                .args(["-rn", "--include=*.rs", "--include=*.ts", "--include=*.tsx",
                       "--include=*.js", "--include=*.py", "--include=*.go",
                       "--include=*.toml", "--include=*.json",
                       "-I", args, dir])
                .output();
            match output {
                Ok(o) if o.status.success() => {
                    let results = String::from_utf8_lossy(&o.stdout);
                    let lines: Vec<&str> = results.lines().take(50).collect();
                    let mut content = format!("Search results for \"{}\":\n\n", args);
                    for line in &lines {
                        // Strip the working dir prefix for cleaner display
                        let display = line.strip_prefix(dir).unwrap_or(line);
                        let display = display.strip_prefix('/').unwrap_or(display);
                        content.push_str(&format!("  {}\n", display));
                    }
                    let total: usize = results.lines().count();
                    if total > 50 {
                        content.push_str(&format!("\n  ... and {} more matches", total - 50));
                    } else {
                        content.push_str(&format!("\n  {} match{}", total, if total == 1 { "" } else { "es" }));
                    }
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content,
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                Ok(_) => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("No results for \"{}\"", args),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                Err(e) => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Search failed: {}", e),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
        }
    }

    pub(crate) fn slash_cmd_symbols(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /symbols <file_path>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            let file_path = if args.starts_with('/') {
                std::path::PathBuf::from(args)
            } else {
                std::path::Path::new(&self.working_dir).join(args)
            };
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    let lang = project::detect_language(&file_path);
                    let mut symbols = Vec::new();
                    for (i, line) in content.lines().enumerate() {
                        let trimmed = line.trim();
                        let is_symbol = match lang.as_str() {
                            "Rust" => trimmed.starts_with("pub fn ")
                                || trimmed.starts_with("fn ")
                                || trimmed.starts_with("pub struct ")
                                || trimmed.starts_with("struct ")
                                || trimmed.starts_with("pub enum ")
                                || trimmed.starts_with("enum ")
                                || trimmed.starts_with("pub trait ")
                                || trimmed.starts_with("trait ")
                                || trimmed.starts_with("impl ")
                                || trimmed.starts_with("pub type ")
                                || trimmed.starts_with("pub const ")
                                || trimmed.starts_with("pub mod ")
                                || trimmed.starts_with("mod "),
                            "TypeScript" | "JavaScript" => trimmed.starts_with("function ")
                                || trimmed.starts_with("export function ")
                                || trimmed.starts_with("export const ")
                                || trimmed.starts_with("export class ")
                                || trimmed.starts_with("class ")
                                || trimmed.starts_with("interface ")
                                || trimmed.starts_with("export interface ")
                                || trimmed.starts_with("type ")
                                || trimmed.starts_with("export type "),
                            "Python" => trimmed.starts_with("def ")
                                || trimmed.starts_with("class ")
                                || trimmed.starts_with("async def "),
                            "Go" => trimmed.starts_with("func ")
                                || trimmed.starts_with("type "),
                            _ => false,
                        };
                        if is_symbol {
                            symbols.push(format!("{:>4} | {}", i + 1, trimmed));
                        }
                    }
                    let mut msg = format!("Symbols in {} ({}):\n\n", args, lang);
                    if symbols.is_empty() {
                        msg.push_str("  (no symbols found)");
                    } else {
                        for s in &symbols {
                            msg.push_str(&format!("  {}\n", s));
                        }
                        msg.push_str(&format!("\n  {} symbol{}", symbols.len(), if symbols.len() == 1 { "" } else { "s" }));
                    }
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: msg,
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                Err(e) => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Cannot read {}: {}", args, e),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
        }
    }

    pub(crate) fn slash_cmd_impact(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /impact <file_path>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            // Find what imports/uses this file
            let basename = std::path::Path::new(args)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(args);
            let dir = &self.working_dir;
            let output = std::process::Command::new("grep")
                .args(["-rn", "--include=*.rs", "--include=*.ts", "--include=*.tsx",
                       "--include=*.js", "--include=*.py", "--include=*.go",
                       "-I", basename, dir])
                .output();
            match output {
                Ok(o) if o.status.success() => {
                    let results = String::from_utf8_lossy(&o.stdout);
                    // Filter to only import/use lines
                    let imports: Vec<&str> = results.lines()
                        .filter(|l| {
                            let lower = l.to_lowercase();
                            lower.contains("use ") || lower.contains("mod ")
                                || lower.contains("import ") || lower.contains("require(")
                                || lower.contains("from ")
                        })
                        .take(30)
                        .collect();
                    let mut msg = format!("Impact analysis for \"{}\":\n\n", args);
                    if imports.is_empty() {
                        msg.push_str("  No imports/references found.");
                    } else {
                        for line in &imports {
                            let display = line.strip_prefix(dir).unwrap_or(line);
                            let display = display.strip_prefix('/').unwrap_or(display);
                            msg.push_str(&format!("  {}\n", display));
                        }
                        msg.push_str(&format!("\n  {} reference{}", imports.len(), if imports.len() == 1 { "" } else { "s" }));
                    }
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: msg,
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                _ => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("No references found for \"{}\"", args),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
        }
    }
}
