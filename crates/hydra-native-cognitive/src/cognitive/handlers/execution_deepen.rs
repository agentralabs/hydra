//! Multi-pass deepening system — expands shallow stub files into full implementations.

use tokio::sync::mpsc;

use super::super::loop_runner::CognitiveUpdate;
use hydra_native_state::utils::{detect_language, format_bytes, safe_truncate};

// ═══════════════════════════════════════════════════════════════════
// Multi-pass deepening system
// ═══════════════════════════════════════════════════════════════════

/// Result of a deepening pass.
pub(crate) struct DeepenResult {
    pub modules_deepened: usize,
    pub files_expanded: usize,
    pub total_lines: usize,
    pub total_bytes: u64,
}

/// Scan all files under `base_dir` and return (relative_path, line_count, byte_count).
async fn scan_project_files(base_dir: &str) -> Vec<(String, usize, u64)> {
    let mut files = Vec::new();
    let base = std::path::Path::new(base_dir);
    if !base.is_dir() {
        return files;
    }
    let mut stack = vec![base.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden dirs and node_modules
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if !name.starts_with('.') && name != "node_modules" && name != "target" {
                    stack.push(path);
                }
            } else if path.is_file() {
                let rel = path.strip_prefix(base)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if rel.is_empty() {
                    continue;
                }
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    let line_count = content.lines().count();
                    let byte_count = content.len() as u64;
                    files.push((rel, line_count, byte_count));
                }
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// Check if a file is a source file that should be deepened (not config/data files).
fn is_deepenable_source(path: &str) -> bool {
    let ext = path.rsplit('.').next().unwrap_or("");
    matches!(ext,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "kt" |
        "swift" | "c" | "cpp" | "h" | "hpp" | "cs" | "rb" | "php" | "vue" |
        "svelte" | "dart" | "zig" | "ex" | "exs" | "lua" | "scala"
    )
}

/// Group files by their first directory segment (module).
fn group_by_module(files: &[(String, usize, u64)]) -> std::collections::HashMap<String, Vec<(String, usize, u64)>> {
    let mut groups: std::collections::HashMap<String, Vec<(String, usize, u64)>> = std::collections::HashMap::new();
    for (path, lines, bytes) in files {
        if !is_deepenable_source(path) {
            continue;
        }
        let module = if let Some(idx) = path.find('/') {
            let first = &path[..idx];
            // Use two levels if first is "src" or "lib"
            if (first == "src" || first == "lib" || first == "app") && path[idx + 1..].contains('/') {
                let rest = &path[idx + 1..];
                if let Some(idx2) = rest.find('/') {
                    format!("{}/{}", first, &rest[..idx2])
                } else {
                    first.to_string()
                }
            } else {
                first.to_string()
            }
        } else {
            "root".to_string()
        };
        groups.entry(module).or_default().push((path.clone(), *lines, *bytes));
    }
    groups
}

/// Build a deepening prompt for a specific module group.
fn build_deepen_prompt(project_summary: &str, module: &str, files: &[(String, usize, u64)]) -> String {
    let mut file_listing = String::new();
    for (path, lines, _) in files {
        file_listing.push_str(&format!("- {} ({} lines)\n", path, lines));
    }
    format!(
        "You are expanding shallow stub files into full, production-quality implementations.\n\n\
         Project: {}\n\
         Module: {}\n\n\
         These files were generated as stubs and need to be fully implemented:\n{}\n\
         For EACH file listed above, output a complete, production-ready implementation.\n\
         Use real logic, proper error handling, documentation, and tests where appropriate.\n\
         Do NOT output placeholder comments like \"// TODO\" or \"// implement here\".\n\n\
         Output format — for each file, use exactly this format:\n\
         === FILE: <relative_path> ===\n\
         <full file content>\n\
         === END FILE ===\n\n\
         Expand ALL files listed above. Make them substantial and correct.",
        project_summary, module, file_listing
    )
}

/// Parse the LLM deepening response into file path -> content pairs.
fn parse_deepen_response(response: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut remaining = response;
    while let Some(start_marker) = remaining.find("=== FILE: ") {
        let after_marker = &remaining[start_marker + 10..];
        let line_end = after_marker.find(" ===").or_else(|| after_marker.find('\n'));
        if let Some(end) = line_end {
            let path = after_marker[..end].trim().to_string();
            let content_start = after_marker[end..].find('\n').map(|i| end + i + 1).unwrap_or(end);
            let after_path = &after_marker[content_start..];
            let content_end = after_path.find("=== END FILE ===").unwrap_or(after_path.len());
            let content = after_path[..content_end].trim_end().to_string();
            if !path.is_empty() && !content.is_empty() {
                results.push((path, content));
            }
            remaining = &after_path[content_end..];
        } else {
            break;
        }
    }
    results
}

/// Call the LLM provider and return the response content.
async fn call_llm_for_deepening(
    prompt: &str,
    llm_config: &hydra_model::LlmConfig,
    provider: &str,
    model: &str,
) -> Result<String, String> {
    let request = hydra_model::CompletionRequest {
        model: model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        max_tokens: {
            // Use actual model limits for deepening calls
            match model {
                m if m.contains("opus") => 32_768,
                m if m.contains("sonnet") => 16_384,
                m if m.contains("haiku") => 8_192,
                m if m.contains("gpt-4o") => 16_384,
                m if m.contains("gpt-4") => 8_192,
                m if m.contains("ollama") | m.contains("llama") | m.contains("phi") | m.contains("mistral") => 4_096,
                _ => 16_384,
            }
        },
        temperature: Some(0.2),
        system: Some("You are a senior software engineer. Expand stub files into full implementations. Output ONLY the file contents in the specified format.".to_string()),
    };

    match provider {
        "anthropic" => {
            let client = hydra_model::providers::anthropic::AnthropicClient::new(llm_config)
                .map_err(|e| format!("{}", e))?;
            client.complete(request).await
                .map(|r| r.content)
                .map_err(|e| format!("{}", e))
        }
        "openai" | "google" => {
            let client = hydra_model::providers::openai::OpenAiClient::new(llm_config)
                .map_err(|e| format!("{}", e))?;
            client.complete(request).await
                .map(|r| r.content)
                .map_err(|e| format!("{}", e))
        }
        "ollama" => {
            let mut ollama_config = llm_config.clone();
            ollama_config.openai_api_key = Some("ollama".into());
            ollama_config.openai_base_url = "http://localhost:11434".into();
            let client = hydra_model::providers::openai::OpenAiClient::new(&ollama_config)
                .map_err(|e| format!("{}", e))?;
            client.complete(request).await
                .map(|r| r.content)
                .map_err(|e| format!("{}", e))
        }
        _ => Err("Unsupported provider".into()),
    }
}

/// Multi-pass deepening: if average lines per source file < 25, expand modules iteratively.
///
/// Scans the project on disk, groups shallow files by module, and makes targeted LLM calls
/// to replace stub files with full implementations.
pub(crate) async fn maybe_deepen_project(
    base_dir: &str,
    project_summary: &str,
    llm_config: &hydra_model::LlmConfig,
    provider: &str,
    model: &str,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Option<DeepenResult> {
    let files = scan_project_files(base_dir).await;
    if files.is_empty() {
        return None;
    }

    // Only consider source files for shallowness check
    let source_files: Vec<_> = files.iter()
        .filter(|(p, _, _)| is_deepenable_source(p))
        .collect();

    if source_files.is_empty() {
        return None;
    }

    let total_source_lines: usize = source_files.iter().map(|(_, l, _)| l).sum();
    let avg_lines = total_source_lines / source_files.len();

    // Threshold: if average source file has >= 25 lines, no deepening needed
    if avg_lines >= 25 {
        return None;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Deepening project...".into()));

    let modules = group_by_module(&files);
    let mut modules_deepened = 0usize;
    let mut files_expanded = 0usize;

    for (module_name, module_files) in &modules {
        // Only deepen modules where average is shallow
        let module_avg: usize = module_files.iter().map(|(_, l, _)| l).sum::<usize>()
            / module_files.len().max(1);
        if module_avg >= 25 {
            continue;
        }

        let display_module = if module_name == "root" {
            "root files".to_string()
        } else {
            format!("{} module", module_name)
        };
        let _ = tx.send(CognitiveUpdate::Phase(format!("Deepening {}...", display_module)));

        let prompt = build_deepen_prompt(project_summary, module_name, module_files);

        let deepen_result = tokio::time::timeout(
            std::time::Duration::from_secs(90),
            call_llm_for_deepening(&prompt, llm_config, provider, model),
        ).await.unwrap_or_else(|_| Err("Deepening LLM call timed out after 90s".into()));
        match deepen_result {
            Ok(response) => {
                let expanded = parse_deepen_response(&response);
                for (rel_path, content) in &expanded {
                    let full_path = format!("{}/{}", base_dir, rel_path);
                    if let Some(parent) = std::path::Path::new(&full_path).parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    let _ = tokio::fs::write(&full_path, content).await;
                    files_expanded += 1;

                    let line_count = content.lines().count();
                    let byte_count = content.len() as u64;
                    let lang = detect_language(rel_path);
                    let _ = tx.send(CognitiveUpdate::EvidenceCode {
                        title: format!("Deepened: {} ({} lines, {})", rel_path, line_count, format_bytes(byte_count)),
                        content: safe_truncate(&content, 500).to_string(),
                        language: Some(lang.to_string()),
                        file_path: Some(rel_path.to_string()),
                    });
                }
                modules_deepened += 1;
            }
            Err(err) => {
                let _ = tx.send(CognitiveUpdate::EvidenceCode {
                    title: format!("Deepening {} failed", display_module),
                    content: err,
                    language: None,
                    file_path: None,
                });
            }
        }
    }

    if modules_deepened == 0 {
        return None;
    }

    // Re-scan to get final totals
    let final_files = scan_project_files(base_dir).await;
    let total_lines: usize = final_files.iter().map(|(_, l, _)| l).sum();
    let total_bytes: u64 = final_files.iter().map(|(_, _, b)| b).sum();

    Some(DeepenResult {
        modules_deepened,
        files_expanded,
        total_lines,
        total_bytes,
    })
}
