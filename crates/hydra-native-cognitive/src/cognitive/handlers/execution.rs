//! Plan execution and project deepening logic.

use std::sync::Arc;
use tokio::sync::mpsc;

use super::super::loop_runner::CognitiveUpdate;
use hydra_native_state::utils::{detect_language, format_bytes, safe_truncate};
use hydra_runtime::undo::{UndoStack, FileCreateAction};

// Re-export deepening types/functions so existing callers are unaffected.
pub(crate) use super::execution_deepen::{DeepenResult, maybe_deepen_project};

/// Execute a JSON plan (create dirs, files, run commands).
pub(crate) async fn execute_json_plan(
    plan: &serde_json::Value,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
    undo_stack: &Option<Arc<parking_lot::Mutex<UndoStack>>>,
) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let project_dir_name = plan["project_dir"].as_str().unwrap_or("hydra-project");
    let base_dir = format!("{}/projects/{}", home, project_dir_name);
    let _ = tokio::fs::create_dir_all(&base_dir).await;

    let steps = plan["steps"].as_array();
    let total_steps = steps.map(|s| s.len()).unwrap_or(0);

    let mut files_created: Vec<(String, usize, u64)> = Vec::new();
    let mut dirs_created = 0u32;
    let mut commands_run: Vec<(String, bool)> = Vec::new();
    let mut total_lines = 0usize;
    let mut total_bytes = 0u64;
    let mut languages: std::collections::HashMap<String, (u32, usize)> = std::collections::HashMap::new();

    if let Some(steps) = steps {
        for (i, step) in steps.iter().enumerate() {
            let step_type = step["type"].as_str().unwrap_or("");

            match step_type {
                "create_dir" => {
                    let path = step["path"].as_str().unwrap_or("");
                    let full_path = format!("{}/{}", base_dir, path);
                    let _ = tokio::fs::create_dir_all(&full_path).await;
                    dirs_created += 1;
                }
                "create_file" | "modify_file" => {
                    let path = step["path"].as_str().unwrap_or("");
                    let content = step["content"].as_str().unwrap_or("");
                    let full_path = format!("{}/{}", base_dir, path);
                    if let Some(parent) = std::path::Path::new(&full_path).parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    let _ = tokio::fs::write(&full_path, content).await;

                    // Track file creation in undo stack
                    if let Some(undo) = undo_stack {
                        let action = FileCreateAction::new(&full_path, content.as_bytes().to_vec());
                        undo.lock().push(Box::new(action));
                        let stack = undo.lock();
                        let _ = tx.send(CognitiveUpdate::UndoStatus {
                            can_undo: stack.can_undo(),
                            can_redo: stack.can_redo(),
                            last_action: stack.last_action_description().map(String::from),
                        });
                    }

                    let line_count = content.lines().count();
                    let byte_count = content.len() as u64;
                    total_lines += line_count;
                    total_bytes += byte_count;
                    files_created.push((path.to_string(), line_count, byte_count));

                    let lang = detect_language(path);
                    let entry = languages.entry(lang.to_string()).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += line_count;

                    let _ = tx.send(CognitiveUpdate::EvidenceCode {
                        title: format!("{} ({} lines, {})", path, line_count, format_bytes(byte_count)),
                        content: safe_truncate(&content, 500).to_string(),
                        language: Some(lang.to_string()),
                        file_path: Some(path.to_string()),
                    });
                }
                "run_command" => {
                    let cmd = step["command"].as_str().unwrap_or("");
                    let cwd = step["cwd"].as_str().unwrap_or(".");
                    let work_dir = if cwd == "." { base_dir.clone() } else { format!("{}/{}", base_dir, cwd) };

                    let output = tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd)
                        .current_dir(&work_dir)
                        .output()
                        .await;

                    let success = output.as_ref().map(|o| o.status.success()).unwrap_or(false);
                    commands_run.push((cmd.to_string(), success));

                    if let Ok(out) = output {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        let display = if !stdout.is_empty() { stdout.to_string() } else { stderr.to_string() };
                        if !display.is_empty() {
                            let _ = tx.send(CognitiveUpdate::EvidenceCode {
                                title: format!("$ {} {}", cmd, if success { "✓" } else { "✗" }),
                                content: safe_truncate(&display, 300).to_string(),
                                language: Some("bash".to_string()),
                                file_path: None,
                            });
                        }
                    }
                }
                _ => {}
            }

            // Report plan step progress
            if total_steps > 0 {
                let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: i, duration_ms: None });
                if i + 1 < total_steps {
                    let _ = tx.send(CognitiveUpdate::PlanStepStart(i + 1));
                }
            }
        }
    }

    // Build rich metrics response
    let mut lang_list: Vec<_> = languages.iter().collect();
    lang_list.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));

    let completion_msg = plan["completion_message"].as_str().unwrap_or("");
    let summary = plan["summary"].as_str().unwrap_or("Project created");
    let commands_ok = commands_run.iter().filter(|(_, s)| *s).count();

    let mut metrics = format!(
        "## {}\n\n\
         ### Project Metrics\n\
         | Metric | Value |\n\
         |--------|-------|\n\
         | Location | `~/projects/{}` |\n\
         | Files created | **{}** |\n\
         | Directories | **{}** |\n\
         | Total lines of code | **{}** |\n\
         | Total size | **{}** |\n\
         | Commands executed | **{}/{}** passed |\n\n",
        summary, project_dir_name,
        files_created.len(), dirs_created,
        total_lines, format_bytes(total_bytes),
        commands_ok, commands_run.len(),
    );

    if !lang_list.is_empty() {
        metrics.push_str("### Languages\n| Language | Files | Lines |\n|----------|-------|-------|\n");
        for (lang, (count, lines)) in &lang_list {
            metrics.push_str(&format!("| {} | {} | {} |\n", lang, count, lines));
        }
        metrics.push('\n');
    }

    metrics.push_str("### Files\n| File | Lines | Size |\n|------|-------|------|\n");
    for (path, lines, bytes) in &files_created {
        metrics.push_str(&format!("| `{}` | {} | {} |\n", path, lines, format_bytes(*bytes)));
    }
    metrics.push('\n');

    if !commands_run.is_empty() {
        metrics.push_str("### Commands\n");
        for (cmd, success) in &commands_run {
            metrics.push_str(&format!("- `{}` {}\n", cmd, if *success { "✓" } else { "✗" }));
        }
        metrics.push('\n');
    }

    if !completion_msg.is_empty() {
        metrics.push_str(&format!("### Getting Started\n{}\n", completion_msg));
    }

    metrics
}
