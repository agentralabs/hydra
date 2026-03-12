//! Slash command dispatcher — delegates to category modules.
//!
//! Submodules (all `impl App` extensions):
//!   slash_commands_dev_files    — /files, /open, /edit, /search, /symbols, /impact
//!   slash_commands_dev_project  — /diff, /git, /test, /build, /run, /lint, /fmt, /deps, /bench, /doc, /deploy, /init
//!   slash_commands_system       — /sisters, /fix, /scan, /repair, /memory, /goals, /beliefs, /receipts, /health, /status
//!   slash_commands_session      — /clear, /compact, /history, /export, /context, /resume, /fork, /rewind, /rename, /review
//!   slash_commands_hydra        — /version, /env, /dream, /obstacles, /threat, /autonomy, /implement, /doctor, /help, etc.
//!   slash_commands_integration  — /mcp, /ide, /hooks, /plugin, /agents, /skills, /commands, /plan, /bashes, /tasks, etc.
//!   slash_commands_model        — /usage, /fast, /todos, /add-dir, /terminal-setup, /login, /logout, /keybindings, etc.

use super::app::App;

impl App {
    pub(crate) fn handle_slash_command(&mut self, input: &str, timestamp: &str) {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0];
        let args = parts.get(1).copied().unwrap_or("");

        match cmd {
            // ── Developer — file operations ──
            "/files"   => self.slash_cmd_files(args, timestamp),
            "/open"    => self.slash_cmd_open(args, timestamp),
            "/edit"    => self.slash_cmd_edit(args, timestamp),
            "/search"  => self.slash_cmd_search(args, timestamp),
            "/symbols" => self.slash_cmd_symbols(args, timestamp),
            "/impact"  => self.slash_cmd_impact(args, timestamp),

            // ── Developer — project operations ──
            "/diff"    => self.slash_cmd_diff(timestamp),
            "/git"     => self.slash_cmd_git(args, timestamp),
            "/test"    => self.slash_cmd_test(timestamp),
            "/build"   => self.slash_cmd_build(timestamp),
            "/run"     => self.slash_cmd_run(timestamp),
            "/lint"    => self.slash_cmd_lint(timestamp),
            "/fmt"     => self.slash_cmd_fmt(timestamp),
            "/deps"    => self.slash_cmd_deps(timestamp),
            "/bench"   => self.slash_cmd_bench(timestamp),
            "/doc"     => self.slash_cmd_doc(timestamp),
            "/deploy"  => self.slash_cmd_deploy(timestamp),
            "/init"    => self.slash_cmd_init(timestamp),

            // ── System ──
            "/sisters"  => self.slash_cmd_sisters(timestamp),
            "/sister"   => self.slash_cmd_sister(args, timestamp),
            "/fix"      => self.slash_cmd_fix(timestamp),
            "/scan"     => self.slash_cmd_scan(timestamp),
            "/repair"   => self.slash_cmd_repair(timestamp),
            "/memory"   => self.slash_cmd_memory(timestamp),
            "/goals"    => self.slash_cmd_goals(timestamp),
            "/beliefs"  => self.slash_cmd_beliefs(timestamp),
            "/receipts" => self.slash_cmd_receipts(timestamp),
            "/health"   => self.slash_cmd_health(timestamp),
            "/status"   => self.slash_cmd_status(timestamp),

            // ── Session Management (Claude Code Parity) ──
            "/clear"    => self.slash_cmd_clear(),
            "/compact"  => self.slash_cmd_compact(args, timestamp),
            "/history"  => self.slash_cmd_history(args, timestamp),
            "/resume" | "/continue" => self.slash_cmd_resume(args, timestamp),
            "/fork"     => self.slash_cmd_fork(timestamp),
            "/rewind"   => self.slash_cmd_rewind(timestamp),
            "/rename"   => self.slash_cmd_rename(args, timestamp),
            "/export"   => self.slash_cmd_export(args, timestamp),
            "/context"  => self.slash_cmd_context(timestamp),

            // ── Model & Cost ──
            "/model"   => self.slash_cmd_model(timestamp),
            "/cost"    => self.slash_cmd_cost(timestamp),
            "/tokens"  => self.slash_cmd_tokens(timestamp),
            "/usage"   => self.slash_cmd_usage(timestamp),
            "/fast"    => self.slash_cmd_fast(timestamp),

            // ── Code & Review ──
            "/review"  => self.slash_cmd_review(timestamp),
            "/todos"   => self.slash_cmd_todos(timestamp),
            "/add-dir" => self.slash_cmd_add_dir(args, timestamp),

            // ── Config ──
            "/config"         => self.slash_cmd_config(timestamp),
            "/doctor"         => self.slash_cmd_doctor(timestamp),
            "/vim"            => self.slash_cmd_vim(timestamp),
            "/sidebar"        => self.slash_cmd_sidebar(),
            "/voice"          => self.slash_cmd_voice(timestamp),
            "/theme"          => self.slash_cmd_theme(timestamp),
            "/terminal-setup" => self.slash_cmd_terminal_setup(timestamp),
            "/login"          => self.slash_cmd_login(timestamp),
            "/logout"         => self.slash_cmd_logout(timestamp),
            "/keybindings"    => self.slash_cmd_keybindings(timestamp),
            "/output-style"   => self.slash_cmd_output_style(timestamp),

            // ── Integrations (Claude Code Parity) ──
            "/mcp"                => self.slash_cmd_mcp(args, timestamp),
            "/ide"                => self.slash_cmd_ide(timestamp),
            "/install-github-app" => self.slash_cmd_install_github_app(timestamp),
            "/hooks"              => self.slash_cmd_hooks(timestamp),
            "/plugin"             => self.slash_cmd_plugin(args, timestamp),
            "/remote-control"     => self.slash_cmd_remote_control(timestamp),
            "/remote"             => self.slash_cmd_remote(timestamp),

            // ── Agents & Skills (Claude Code Parity) ──
            "/agents"   => self.slash_cmd_agents(timestamp),
            "/skills"   => self.slash_cmd_skills(timestamp),
            "/commands"  => self.slash_cmd_commands(timestamp),
            "/plan"     => self.slash_cmd_plan(timestamp),
            "/bashes"   => self.slash_cmd_bashes(timestamp),
            "/tasks"    => self.slash_cmd_tasks(timestamp),

            // ── Hydra-Exclusive ──
            "/version"     => self.slash_cmd_version(timestamp),
            "/env"         => self.slash_cmd_env(args, timestamp),
            "/dream"       => self.slash_cmd_dream(timestamp),
            "/obstacles"   => self.slash_cmd_obstacles(timestamp),
            "/threat"      => self.slash_cmd_threat(timestamp),
            "/autonomy"    => self.slash_cmd_autonomy(args, timestamp),
            "/implement"   => self.slash_cmd_implement(args, timestamp),
            "/diagnostics" => self.slash_cmd_diagnostics(timestamp),

            // ── Control ──
            "/trust"            => self.slash_cmd_trust(timestamp),
            "/approve" | "/y"   => self.slash_cmd_approve(timestamp),
            "/deny"    | "/n"   => self.slash_cmd_deny(timestamp),
            "/kill"             => self.slash_cmd_kill(),

            // ── Debug ──
            "/log"    => self.slash_cmd_log(timestamp),
            "/debug"  => self.slash_cmd_debug(timestamp),

            // ── Help ──
            "/help" | "/?" => self.slash_cmd_help(timestamp),

            // ── Exit ──
            "/quit" | "/exit" | "/q" => self.slash_cmd_quit(),

            _ => self.slash_cmd_unknown(cmd, timestamp),
        }
        self.scroll_to_bottom();
    }
}
