//! Priority 5: Deep Aegis Integration — input/output validation,
//! shadow execution for high-risk commands.
//!
//! Adds OUTPUT validation (new capability) and pre-execution shadow testing.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

/// Aegis validation result.
#[derive(Debug, Clone)]
pub struct AegisValidation {
    pub safe: bool,
    pub reason: String,
    pub severity: AegisSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AegisSeverity {
    Safe,
    Suspicious,
    Blocked,
}

impl Sisters {
    /// ACT: Validate a command BEFORE execution via Aegis sister.
    /// Falls back to None if sister is offline (caller uses local blocked list).
    pub async fn aegis_validate_input(&self, command: &str) -> Option<AegisValidation> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_validate_input", serde_json::json!({
            "command": command,
            "context": "shell_execution",
        })).await.ok()?;

        let safe = result.get("safe").and_then(|v| v.as_bool()).unwrap_or(true);
        let reason = result.get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let severity_str = result.get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("safe");
        let severity = match severity_str {
            "blocked" => AegisSeverity::Blocked,
            "suspicious" => AegisSeverity::Suspicious,
            _ => AegisSeverity::Safe,
        };

        Some(AegisValidation { safe, reason, severity })
    }

    /// ACT: Validate command OUTPUT after execution via Aegis sister.
    /// Catches leaked secrets, unexpected binary data, injection in output.
    /// This is a NEW capability — Hydra doesn't have local output validation.
    pub async fn aegis_validate_output(
        &self,
        command: &str,
        output: &str,
    ) -> Option<AegisValidation> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_validate_output", serde_json::json!({
            "command": command,
            "output": safe_truncate(output, 2000),
        })).await.ok()?;

        let safe = result.get("safe").and_then(|v| v.as_bool()).unwrap_or(true);
        let reason = result.get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let severity = if safe { AegisSeverity::Safe } else { AegisSeverity::Suspicious };

        Some(AegisValidation { safe, reason, severity })
    }

    /// SESSION: Create a security session with Aegis — initializes threat tracking.
    pub async fn aegis_session_create(&self, user_name: &str) {
        if let Some(aegis) = &self.aegis {
            let _ = aegis.call_tool("aegis_session_create", serde_json::json!({
                "agent_id": user_name,
                "session_type": "conversation",
            })).await;
        }
    }

    /// SESSION: End Aegis security session — flushes threat log and audit trail.
    pub async fn aegis_session_end(&self, summary: &str) {
        if let Some(aegis) = &self.aegis {
            let _ = aegis.call_tool("aegis_session_end", serde_json::json!({
                "summary": safe_truncate(summary, 500),
            })).await;
        }
    }

    /// DECIDE: Shadow execute a high-risk command in sandbox.
    /// Returns predicted outcome before real execution.
    pub async fn aegis_shadow_execute(&self, command: &str) -> Option<ShadowResult> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_shadow_execute", serde_json::json!({
            "command": command,
            "dry_run": true,
            "timeout_ms": 5000,
        })).await.ok()?;

        let safe = result.get("safe").and_then(|v| v.as_bool()).unwrap_or(true);
        let predicted_outcome = extract_text(&result);
        let files_affected = result.get("files_affected")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        Some(ShadowResult {
            safe,
            predicted_outcome,
            files_affected,
        })
    }
}

/// Result from Aegis shadow execution.
#[derive(Debug, Clone)]
pub struct ShadowResult {
    pub safe: bool,
    pub predicted_outcome: String,
    pub files_affected: u32,
}

/// Local fast-path validation — blocked commands that skip sister call entirely.
/// These are instantly blocked; no point asking Aegis about obviously dangerous commands.
pub fn is_locally_blocked(command: &str) -> bool {
    let lower = command.to_lowercase();
    let blocked = [
        "rm -rf /", "dd if=/dev/zero", "mkfs",
        "shutdown", "reboot", "halt",
        ":(){ :|:& };:", // fork bomb
        "chmod 777 /", "iptables -f",
    ];
    blocked.iter().any(|b| lower.contains(b))
}

/// Check if output might contain leaked secrets.
pub fn output_contains_secrets(output: &str) -> bool {
    let patterns = [
        "AKIA",           // AWS access key
        "sk-",            // OpenAI/Stripe key prefix
        "ghp_",           // GitHub PAT
        "-----BEGIN RSA",  // Private key
        "password=",
        "secret_key=",
        "api_key=",
    ];
    patterns.iter().any(|p| output.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locally_blocked() {
        assert!(is_locally_blocked("rm -rf /"));
        assert!(is_locally_blocked("sudo dd if=/dev/zero of=/dev/sda"));
        assert!(!is_locally_blocked("ls -la"));
        assert!(!is_locally_blocked("cargo test"));
    }

    #[test]
    fn test_output_secrets() {
        assert!(output_contains_secrets("Found key: AKIAIOSFODNN7EXAMPLE"));
        assert!(output_contains_secrets("api_key=sk-abc123"));
        assert!(output_contains_secrets("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(!output_contains_secrets("Build succeeded"));
    }

    #[test]
    fn test_aegis_validation_struct() {
        let v = AegisValidation {
            safe: false,
            reason: "Destructive command".into(),
            severity: AegisSeverity::Blocked,
        };
        assert!(!v.safe);
        assert_eq!(v.severity, AegisSeverity::Blocked);
    }
}
