//! CLI flag parsing for spec §10 (Claude Code parity).
//! Parses flags like -p, -c, -r, --model, --permission-mode, etc.

/// Parsed CLI flags from command-line arguments.
#[derive(Debug, Default)]
pub struct CliFlags {
    /// -p "task" — print mode (non-interactive one-shot)
    pub print_task: Option<String>,
    /// -c — continue last session
    pub continue_session: bool,
    /// -r <id> — resume specific session
    pub resume_session: Option<String>,
    /// --from-pr <number> — resume PR session
    pub from_pr: Option<String>,
    /// --model <name> — use specific model
    pub model: Option<String>,
    /// --max-budget-usd <amount> — cost cap
    pub max_budget_usd: Option<f64>,
    /// --permission-mode <mode> — start in plan/auto-accept mode
    pub permission_mode: Option<String>,
    /// --dangerously-skip-permissions — skip all approvals
    pub skip_permissions: bool,
    /// --verbose — full turn-by-turn logging
    pub verbose: bool,
    /// --output-format json|stream-json — structured output
    pub output_format: Option<String>,
    /// --system-prompt "..." — inline system prompt
    pub system_prompt: Option<String>,
    /// --system-prompt-file <path> — system prompt from file
    pub system_prompt_file: Option<String>,
    /// --append-system-prompt "..." — append to default
    pub append_system_prompt: Option<String>,
    /// --allowedTools "Read,Write" — pre-approve tools
    pub allowed_tools: Option<String>,
    /// --disallowedTools "Bash(rm*)" — block tools
    pub disallowed_tools: Option<String>,
    /// --add-dir <path> — extra working directories
    pub add_dirs: Vec<String>,
    /// --repl — legacy REPL mode
    pub repl: bool,
    /// Remaining positional args (subcommand + args)
    pub remaining: Vec<String>,
}

impl CliFlags {
    /// Parse CLI arguments into structured flags.
    pub fn parse(args: &[String]) -> Self {
        let mut flags = CliFlags::default();
        let mut i = 1; // skip argv[0]
        while i < args.len() {
            match args[i].as_str() {
                "-p" => {
                    i += 1;
                    if i < args.len() {
                        flags.print_task = Some(args[i].clone());
                    }
                }
                "-c" => flags.continue_session = true,
                "-r" => {
                    i += 1;
                    if i < args.len() {
                        flags.resume_session = Some(args[i].clone());
                    }
                }
                "--from-pr" => {
                    i += 1;
                    if i < args.len() {
                        flags.from_pr = Some(args[i].clone());
                    }
                }
                "--model" => {
                    i += 1;
                    if i < args.len() {
                        flags.model = Some(args[i].clone());
                    }
                }
                "--max-budget-usd" => {
                    i += 1;
                    if i < args.len() {
                        flags.max_budget_usd = args[i].parse().ok();
                    }
                }
                "--permission-mode" => {
                    i += 1;
                    if i < args.len() {
                        flags.permission_mode = Some(args[i].clone());
                    }
                }
                "--dangerously-skip-permissions" => flags.skip_permissions = true,
                "--verbose" => flags.verbose = true,
                "--output-format" => {
                    i += 1;
                    if i < args.len() {
                        flags.output_format = Some(args[i].clone());
                    }
                }
                "--system-prompt" => {
                    i += 1;
                    if i < args.len() {
                        flags.system_prompt = Some(args[i].clone());
                    }
                }
                "--system-prompt-file" => {
                    i += 1;
                    if i < args.len() {
                        flags.system_prompt_file = Some(args[i].clone());
                    }
                }
                "--append-system-prompt" => {
                    i += 1;
                    if i < args.len() {
                        flags.append_system_prompt = Some(args[i].clone());
                    }
                }
                "--allowedTools" => {
                    i += 1;
                    if i < args.len() {
                        flags.allowed_tools = Some(args[i].clone());
                    }
                }
                "--disallowedTools" => {
                    i += 1;
                    if i < args.len() {
                        flags.disallowed_tools = Some(args[i].clone());
                    }
                }
                "--add-dir" => {
                    i += 1;
                    if i < args.len() {
                        flags.add_dirs.push(args[i].clone());
                    }
                }
                "--repl" => flags.repl = true,
                _ => {
                    // Everything from here on is remaining args
                    flags.remaining = args[i..].to_vec();
                    break;
                }
            }
            i += 1;
        }
        flags
    }

    /// Whether this invocation should launch the TUI.
    pub fn is_tui_mode(&self) -> bool {
        self.remaining.is_empty()
            && self.print_task.is_none()
            && !self.repl
    }

    /// Apply environment overrides from flags.
    pub fn apply_env(&self) {
        if let Some(ref model) = self.model {
            std::env::set_var("HYDRA_MODEL", model);
        }
        if self.verbose {
            std::env::set_var("HYDRA_VERBOSE", "1");
        }
        if let Some(ref budget) = self.max_budget_usd {
            std::env::set_var("HYDRA_MAX_BUDGET_USD", budget.to_string());
        }
        if let Some(ref fmt) = self.output_format {
            std::env::set_var("HYDRA_OUTPUT_FORMAT", fmt);
        }
        if self.skip_permissions {
            std::env::set_var("HYDRA_SKIP_PERMISSIONS", "1");
        }
        if let Some(ref tools) = self.allowed_tools {
            std::env::set_var("HYDRA_ALLOWED_TOOLS", tools);
        }
        if let Some(ref tools) = self.disallowed_tools {
            std::env::set_var("HYDRA_DISALLOWED_TOOLS", tools);
        }
        for dir in &self.add_dirs {
            // Append to HYDRA_EXTRA_DIRS (colon-separated)
            let existing = std::env::var("HYDRA_EXTRA_DIRS").unwrap_or_default();
            let new_val = if existing.is_empty() {
                dir.clone()
            } else {
                format!("{}:{}", existing, dir)
            };
            std::env::set_var("HYDRA_EXTRA_DIRS", new_val);
        }
        // System prompt handling
        if let Some(ref sp) = self.system_prompt {
            std::env::set_var("HYDRA_SYSTEM_PROMPT", sp);
        }
        if let Some(ref path) = self.system_prompt_file {
            if let Ok(content) = std::fs::read_to_string(path) {
                std::env::set_var("HYDRA_SYSTEM_PROMPT", content);
            }
        }
        if let Some(ref sp) = self.append_system_prompt {
            std::env::set_var("HYDRA_APPEND_SYSTEM_PROMPT", sp);
        }
        if let Some(ref mode) = self.permission_mode {
            std::env::set_var("HYDRA_PERMISSION_MODE", mode);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_args() {
        let args = vec!["hydra".to_string()];
        let flags = CliFlags::parse(&args);
        assert!(flags.is_tui_mode());
        assert!(!flags.verbose);
    }

    #[test]
    fn parse_print_mode() {
        let args = vec!["hydra".into(), "-p".into(), "fix the auth bug".into()];
        let flags = CliFlags::parse(&args);
        assert_eq!(flags.print_task.as_deref(), Some("fix the auth bug"));
        assert!(!flags.is_tui_mode());
    }

    #[test]
    fn parse_continue_and_resume() {
        let args = vec!["hydra".into(), "-c".into()];
        let flags = CliFlags::parse(&args);
        assert!(flags.continue_session);

        let args2 = vec!["hydra".into(), "-r".into(), "abc123".into()];
        let flags2 = CliFlags::parse(&args2);
        assert_eq!(flags2.resume_session.as_deref(), Some("abc123"));
    }

    #[test]
    fn parse_model_and_budget() {
        let args = vec![
            "hydra".into(), "--model".into(), "opus".into(),
            "--max-budget-usd".into(), "5.00".into(),
        ];
        let flags = CliFlags::parse(&args);
        assert_eq!(flags.model.as_deref(), Some("opus"));
        assert_eq!(flags.max_budget_usd, Some(5.0));
    }

    #[test]
    fn parse_permission_flags() {
        let args = vec![
            "hydra".into(), "--permission-mode".into(), "plan".into(),
            "--dangerously-skip-permissions".into(),
        ];
        let flags = CliFlags::parse(&args);
        assert_eq!(flags.permission_mode.as_deref(), Some("plan"));
        assert!(flags.skip_permissions);
    }

    #[test]
    fn parse_add_dirs() {
        let args = vec![
            "hydra".into(), "--add-dir".into(), "./services".into(),
            "--add-dir".into(), "./shared".into(),
        ];
        let flags = CliFlags::parse(&args);
        assert_eq!(flags.add_dirs, vec!["./services", "./shared"]);
    }

    #[test]
    fn parse_system_prompt() {
        let args = vec![
            "hydra".into(), "--system-prompt".into(), "Be concise".into(),
        ];
        let flags = CliFlags::parse(&args);
        assert_eq!(flags.system_prompt.as_deref(), Some("Be concise"));
    }

    #[test]
    fn parse_output_format() {
        let args = vec![
            "hydra".into(), "--output-format".into(), "json".into(),
            "--verbose".into(),
        ];
        let flags = CliFlags::parse(&args);
        assert_eq!(flags.output_format.as_deref(), Some("json"));
        assert!(flags.verbose);
    }

    #[test]
    fn parse_tools_flags() {
        let args = vec![
            "hydra".into(),
            "--allowedTools".into(), "Read,Write".into(),
            "--disallowedTools".into(), "Bash(rm*)".into(),
        ];
        let flags = CliFlags::parse(&args);
        assert_eq!(flags.allowed_tools.as_deref(), Some("Read,Write"));
        assert_eq!(flags.disallowed_tools.as_deref(), Some("Bash(rm*)"));
    }

    #[test]
    fn parse_remaining_as_subcommand() {
        let args = vec![
            "hydra".into(), "--verbose".into(), "run".into(), "fix bug".into(),
        ];
        let flags = CliFlags::parse(&args);
        assert!(flags.verbose);
        assert_eq!(flags.remaining, vec!["run", "fix bug"]);
        assert!(!flags.is_tui_mode());
    }

    #[test]
    fn parse_from_pr() {
        let args = vec!["hydra".into(), "--from-pr".into(), "446".into()];
        let flags = CliFlags::parse(&args);
        assert_eq!(flags.from_pr.as_deref(), Some("446"));
    }

    #[test]
    fn parse_repl_flag() {
        let args = vec!["hydra".into(), "--repl".into()];
        let flags = CliFlags::parse(&args);
        assert!(flags.repl);
        assert!(!flags.is_tui_mode());
    }
}
