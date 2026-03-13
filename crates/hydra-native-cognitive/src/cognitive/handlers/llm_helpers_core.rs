//! LLM core utilities — error diagnosis, retry logic, dependency detection.

use hydra_native_state::utils::safe_truncate;

/// Phase 2, A4: Recognize common error patterns and suggest fixes without LLM call.
/// Returns (diagnosis, suggested_fix_command) if a pattern matches.
pub(crate) fn recognize_error_pattern(command: &str, stderr: &str) -> Option<(String, String)> {
    let lower = stderr.to_lowercase();
    let cmd_lower = command.to_lowercase();

    // "command not found"
    if lower.contains("command not found") || lower.contains("not found") {
        let cmd_name = command.split_whitespace().next().unwrap_or(command);
        return Some((
            format!("'{}' is not installed", cmd_name),
            format!("which {} || echo 'Not installed — try: brew install {} or npm install -g {}'", cmd_name, cmd_name, cmd_name),
        ));
    }

    // "permission denied"
    if lower.contains("permission denied") {
        if cmd_lower.starts_with("./") {
            return Some((
                "Script is not executable".to_string(),
                format!("chmod +x {}", command.split_whitespace().next().unwrap_or(command)),
            ));
        }
        return Some((
            "Permission denied — may need elevated privileges".to_string(),
            String::new(), // Don't auto-suggest sudo
        ));
    }

    // "address already in use" / EADDRINUSE
    if lower.contains("eaddrinuse") || lower.contains("address already in use") {
        // Try to extract port number
        let port = stderr.split(':').last()
            .and_then(|s| s.trim().parse::<u16>().ok())
            .unwrap_or(3000);
        return Some((
            format!("Port {} is already in use", port),
            format!("lsof -ti :{} | xargs kill -9 2>/dev/null; echo 'Port {} freed'", port, port),
        ));
    }

    // "No such file or directory"
    if lower.contains("no such file or directory") {
        // Extract the path
        let path = stderr.split("No such file or directory")
            .next()
            .and_then(|s| s.rsplit(':').next())
            .map(|s| s.trim().trim_matches('\'').trim_matches('"'))
            .unwrap_or("");
        if !path.is_empty() {
            let dir = path.rsplit('/').skip(1).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/");
            if !dir.is_empty() {
                return Some((
                    format!("Path '{}' does not exist", path),
                    format!("mkdir -p {}", dir),
                ));
            }
        }
        return Some((
            "File or directory not found".to_string(),
            String::new(),
        ));
    }

    // "EACCES" / npm permission issues
    if lower.contains("eacces") {
        return Some((
            "npm/node permission issue".to_string(),
            "npm config set prefix '~/.npm-global' && export PATH=~/.npm-global/bin:$PATH".to_string(),
        ));
    }

    // "MODULE_NOT_FOUND" / "Cannot find module"
    if lower.contains("module_not_found") || lower.contains("cannot find module") {
        return Some((
            "Missing Node.js module".to_string(),
            "npm install".to_string(),
        ));
    }

    // Compilation errors (Rust)
    if lower.contains("error[e") && lower.contains("aborting due to") {
        return Some((
            "Rust compilation error".to_string(),
            String::new(), // Can't auto-fix compilation errors
        ));
    }

    // Python import errors
    if lower.contains("modulenotfounderror") || lower.contains("no module named") {
        let module = stderr.lines()
            .find(|l| l.contains("No module named"))
            .and_then(|l| l.split("No module named").last())
            .map(|s| s.trim().trim_matches('\'').trim_matches('"'))
            .unwrap_or("unknown");
        return Some((
            format!("Python module '{}' not found", module),
            format!("pip install {}", module),
        ));
    }

    None
}

/// Diagnose a failed command and attempt a retry with a fix.
pub(crate) async fn diagnose_and_retry(
    failed_cmd: &str,
    stderr: &str,
    llm_config: &hydra_model::LlmConfig,
    decide_engine: &crate::cognitive::decide::DecideEngine,
) -> Option<(String, String, bool)> {
    // Step 1: Try error pattern recognition (no LLM cost)
    if let Some((diagnosis, fix_cmd)) = recognize_error_pattern(failed_cmd, stderr) {
        eprintln!("[hydra:retry] Pattern match: {} → fix: {}", diagnosis, fix_cmd);
        if fix_cmd.is_empty() {
            return None; // Pattern recognized but no auto-fix available
        }
        // Security gate check on the fix command
        let gate = decide_engine.evaluate_command(&fix_cmd);
        if !gate.allowed {
            eprintln!("[hydra:retry] Fix command blocked by security gate: {}", gate.reason);
            return None;
        }
        // Execute the fix
        match tokio::process::Command::new("sh")
            .arg("-c").arg(&fix_cmd)
            .output().await
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let success = output.status.success();
                return Some((fix_cmd, stdout, success));
            }
            Err(e) => {
                eprintln!("[hydra:retry] Fix command failed: {}", e);
                return None;
            }
        }
    }

    // Step 2: Micro-LLM diagnosis (cheapest model, 150 tokens, 10s timeout)
    if !llm_config.has_anthropic() && !llm_config.has_openai() {
        return None; // No LLM available
    }
    let model = hydra_model::pick_cheapest_model(llm_config);

    let prompt = format!(
        "This shell command failed:\n```\n$ {}\n```\nError:\n```\n{}\n```\n\
         Reply with ONLY the fixed shell command. No explanation. Just the command.",
        failed_cmd, safe_truncate(stderr, 300)
    );

    let request = hydra_model::CompletionRequest {
        model: model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: prompt,
        }],
        max_tokens: 150,
        temperature: Some(0.0),
        system: Some("You are a shell command fixer. Given a failed command and its error, output ONLY the corrected command. No markdown, no explanation, no backticks.".to_string()),
    };

    let llm_future = async {
        if llm_config.anthropic_api_key.is_some() {
            match hydra_model::providers::anthropic::AnthropicClient::new(llm_config) {
                Ok(client) => client.complete(request).await.ok().map(|r| r.content),
                Err(_) => None,
            }
        } else if llm_config.openai_api_key.is_some() {
            match hydra_model::providers::openai::OpenAiClient::new(llm_config) {
                Ok(client) => client.complete(request).await.ok().map(|r| r.content),
                Err(_) => None,
            }
        } else {
            None
        }
    };

    // 10-second timeout
    let result = match tokio::time::timeout(std::time::Duration::from_secs(10), llm_future).await {
        Ok(r) => r,
        Err(_) => {
            eprintln!("[hydra:retry] LLM diagnosis timed out after 10s");
            return None;
        }
    };

    if let Some(fix_cmd) = result {
        let fix_cmd = fix_cmd.trim().trim_matches('`').trim().to_string();
        if fix_cmd.is_empty() || fix_cmd == failed_cmd {
            return None; // No useful fix
        }
        eprintln!("[hydra:retry] LLM suggests: {}", fix_cmd);

        // Security gate check
        let gate = decide_engine.evaluate_command(&fix_cmd);
        if !gate.allowed {
            eprintln!("[hydra:retry] Fix command blocked by security gate: {}", gate.reason);
            return None;
        }

        match tokio::process::Command::new("sh")
            .arg("-c").arg(&fix_cmd)
            .output().await
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let success = output.status.success();
                return Some((fix_cmd, stdout, success));
            }
            Err(e) => {
                eprintln!("[hydra:retry] Retried command failed: {}", e);
            }
        }
    }

    None
}

/// Phase 2, A3: Check if a command depends on a previously failed command.
/// Uses heuristics to detect common dependency patterns.
pub(crate) fn commands_are_dependent(failed_cmd_output: &str, next_cmd: &str) -> bool {
    let next_lower = next_cmd.to_lowercase();

    // cd into a directory that was supposed to be created
    if next_lower.starts_with("cd ") {
        return true; // cd always depends on prior context
    }

    // npm/yarn/pip commands that depend on prior install
    if (next_lower.contains("npm start") || next_lower.contains("npm run")
        || next_lower.contains("yarn start") || next_lower.contains("yarn run"))
        && failed_cmd_output.contains("npm install")
    {
        return true;
    }

    // Commands that reference files/directories from failed output
    // e.g., "cat output.txt" after a command that was supposed to create output.txt
    if failed_cmd_output.contains("No such file") || failed_cmd_output.contains("not found") {
        return true; // If previous failed due to missing files, next likely depends on it
    }

    // Sequential pipe-like patterns: if the failed command was supposed to produce output
    // that the next command needs
    if next_lower.starts_with("grep ") || next_lower.starts_with("awk ")
        || next_lower.starts_with("sed ") || next_lower.starts_with("sort ")
        || next_lower.starts_with("wc ")
    {
        // These are typically downstream processors
        return true;
    }

    false
}
