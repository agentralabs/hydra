//! Sister delegation methods — thin wrappers that dispatch tool calls to
//! individual sister MCP servers. Separated from `cognitive.rs` for file size.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    /// THINK: Forge blueprint generation for complex tasks (before LLM)
    pub async fn think_forge(&self, text: &str) -> Option<String> {
        if let Some(s) = &self.forge {
            let result = s.call_tool("forge_blueprint_create", serde_json::json!({
                "intent": text,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// THINK: Veritas intent compilation (structured intent from user text)
    pub async fn think_veritas(&self, text: &str) -> Option<String> {
        if let Some(s) = &self.veritas {
            let result = s.call_tool("intent_compile", serde_json::json!({
                "input": text,
                "include_ambiguity": true,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// DECIDE: Contract policy validation (check if action is allowed)
    pub async fn decide_contract(&self, action: &str, risk_level: &str) -> Option<String> {
        if let Some(s) = &self.contract {
            let result = s.call_tool("policy_validate", serde_json::json!({
                "action": action,
                "risk_level": risk_level,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// DECIDE: Veritas uncertainty quantification
    pub async fn decide_veritas(&self, action: &str) -> Option<String> {
        if let Some(s) = &self.veritas {
            let result = s.call_tool("uncertainty_quantify", serde_json::json!({
                "action": action,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// ACT: Vision web capture (capture and parse a web page)
    pub async fn act_vision_capture(&self, url: &str) -> Option<String> {
        if let Some(s) = &self.vision {
            let result = s.call_tool("vision_web_map", serde_json::json!({
                "url": url,
                "extract_text": true,
                "extract_links": true,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// ACT: Aegis shadow execution (validate command safety)
    pub async fn act_aegis_validate(&self, command: &str) -> Option<(bool, String)> {
        if let Some(s) = &self.aegis {
            let result = s.call_tool("aegis_shadow_execute", serde_json::json!({
                "command": command,
                "dry_run": true,
            })).await.ok();
            if let Some(v) = result {
                let safe = v.get("safe").and_then(|s| s.as_bool()).unwrap_or(true);
                let rec = extract_text(&v);
                Some((safe, rec))
            } else {
                None
            }
        } else { None }
    }

    /// ACT: Planning goal checkpoint (update goal progress after action)
    pub async fn act_planning_checkpoint(&self, action: &str, status: &str) {
        if let Some(s) = &self.planning {
            if let Err(e) = s.call_tool("goal_checkpoint", serde_json::json!({"action": action, "status": status})).await {
                eprintln!("[hydra:delegation] goal_checkpoint FAILED: {}", e);
            }
        }
    }

    /// ACT: Identity receipt for command execution
    pub async fn act_receipt(&self, command: &str, risk_level: &str, success: bool) {
        if let Some(s) = &self.identity {
            if let Err(e) = s.call_tool("receipt_create", serde_json::json!({
                "action": format!("command_execution: {}", command), "risk_level": risk_level, "success": success,
            })).await { eprintln!("[hydra:delegation] receipt_create FAILED: {}", e); }
        }
    }

    /// LEARN: Planning goal progress update
    pub async fn learn_planning(&self, user_msg: &str, outcome: &str) {
        if let Some(s) = &self.planning {
            if let Err(e) = s.call_tool("goal_progress", serde_json::json!({"interaction": user_msg, "outcome": outcome})).await {
                eprintln!("[hydra:delegation] goal_progress FAILED: {}", e);
            }
        }
    }

    /// LEARN: Comm share learnings with federated peers
    pub async fn learn_comm_share(&self, insight: &str) {
        if let Some(s) = &self.comm {
            if let Err(e) = s.call_tool("broadcast_insight", serde_json::json!({"insight": insight, "source": "cognitive_loop"})).await {
                eprintln!("[hydra:delegation] broadcast_insight FAILED: {}", e);
            }
        }
    }

    /// LEARN: Memory capture file modifications
    pub async fn learn_capture_files(&self, files: &[String]) {
        if let Some(mem) = &self.memory {
            for file in files {
                if let Err(e) = mem.call_tool("memory_capture_file", serde_json::json!({"path": file, "source": "hydra_native"})).await {
                    eprintln!("[hydra:delegation] capture_file FAILED: {}", e);
                }
            }
        }
    }

    /// LEARN: Memory capture command execution
    pub async fn learn_capture_command(&self, command: &str, output: &str, success: bool) {
        if let Some(mem) = &self.memory {
            if let Err(e) = mem.call_tool("memory_capture_tool", serde_json::json!({
                "tool_name": "shell", "input": command, "output": safe_truncate(&output, 500), "success": success,
            })).await { eprintln!("[hydra:delegation] capture_tool FAILED: {}", e); }
        }
    }

    /// DECIDE: Aegis shadow_validate — validate an action plan against expected outcomes
    /// before execution. Returns (safe, recommendation). Used in the DECIDE phase for
    /// medium+ risk actions alongside contract policy and veritas uncertainty checks.
    pub async fn shadow_validate(&self, action: &str, expected: &std::collections::HashMap<String, String>) -> Option<(bool, String)> {
        if let Some(s) = &self.aegis {
            let result = s.call_tool("shadow_validate", serde_json::json!({
                "action": action,
                "expected_outcomes": expected,
                "dry_run": true,
            })).await.ok();
            if let Some(v) = result {
                let safe = v.get("safe").and_then(|s| s.as_bool()).unwrap_or(true);
                let rec = extract_text(&v);
                Some((safe, rec))
            } else {
                None
            }
        } else { None }
    }

    /// LEARN: Evolve record_action — record an action pattern for skill crystallization.
    /// When repeated patterns are detected, the Evolve sister can emit a SkillCrystallized
    /// event indicating a new reusable skill has been extracted from behavior.
    pub async fn record_action(&self, action: &str, patterns: &[String], success: bool) -> Option<String> {
        if let Some(s) = &self.evolve {
            let result = s.call_tool("evolve_record_action", serde_json::json!({
                "action": action,
                "patterns": patterns,
                "success": success,
            })).await.ok();
            // If Evolve detects a repeated pattern, it returns a SkillCrystallized name
            result.and_then(|v| v.get("skill_name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        } else { None }
    }

    /// PERCEIVE: Fallback screencapture when Vision sister is offline (macOS only).
    /// Uses the macOS `screencapture` command to capture screen state for context.
    pub(crate) async fn screencapture_fallback() -> Option<serde_json::Value> {
        #[cfg(target_os = "macos")]
        {
            let tmp = std::env::temp_dir().join("hydra-screencapture.png");
            let output = tokio::process::Command::new("screencapture")
                .args(["-x", "-t", "png", &tmp.display().to_string()])
                .output()
                .await
                .ok();
            if let Some(o) = output {
                if o.status.success() {
                    return Some(serde_json::json!({
                        "source": "screencapture_fallback",
                        "path": tmp.display().to_string(),
                    }));
                }
            }
            None
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }
}
