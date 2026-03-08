use serde::{Deserialize, Serialize};

/// Hard boundary enforcement — actions that are NEVER allowed regardless of risk score.
/// This runs BEFORE risk assessment in the ExecutionGate pipeline.
#[derive(Debug, Clone)]
pub struct BoundaryEnforcer {
    blocked_paths: Vec<String>,
    blocked_patterns: Vec<BlockedPattern>,
}

/// A pattern that is unconditionally blocked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedPattern {
    pub name: &'static str,
    pub description: &'static str,
    pub check: BlockedCheck,
}

/// What kind of check to perform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockedCheck {
    PathPrefix(String),
    TargetContains(String),
    ActionType(String),
}

/// Result of boundary check
#[derive(Debug, Clone)]
pub enum BoundaryResult {
    Allowed,
    Blocked(BoundaryViolation),
}

/// Details of a boundary violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryViolation {
    pub rule_name: String,
    pub reason: String,
    pub target: String,
}

impl std::fmt::Display for BoundaryViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Boundary violation [{}]: {} (target: {})",
            self.rule_name, self.reason, self.target
        )
    }
}

impl BoundaryEnforcer {
    pub fn new() -> Self {
        Self {
            blocked_paths: vec![
                "/etc/".into(),
                "/System/".into(),
                "/usr/bin/".into(),
                "/usr/sbin/".into(),
                "/sbin/".into(),
                "/boot/".into(),
                "/proc/".into(),
                "/sys/".into(),
                "~/.ssh/".into(),
                ".ssh/".into(),
                "~/.gnupg/".into(),
                ".gnupg/".into(),
            ],
            blocked_patterns: vec![
                BlockedPattern {
                    name: "email_without_confirmation",
                    description: "Email sending requires explicit user confirmation",
                    check: BlockedCheck::TargetContains("send_email".into()),
                },
                BlockedPattern {
                    name: "money_without_approval",
                    description: "Financial transactions require explicit approval",
                    check: BlockedCheck::TargetContains("payment".into()),
                },
                BlockedPattern {
                    name: "delete_without_recovery",
                    description: "Permanent deletion requires recovery path",
                    check: BlockedCheck::TargetContains("rm -rf /".into()),
                },
                BlockedPattern {
                    name: "self_modification",
                    description: "Hydra cannot modify its own binaries or core config",
                    check: BlockedCheck::TargetContains("hydra-gate/src".into()),
                },
                BlockedPattern {
                    name: "self_modification_kernel",
                    description: "Hydra cannot modify its own kernel",
                    check: BlockedCheck::TargetContains("hydra-kernel/src".into()),
                },
                BlockedPattern {
                    name: "self_modification_core",
                    description: "Hydra cannot modify its own core types",
                    check: BlockedCheck::TargetContains("hydra-core/src".into()),
                },
            ],
        }
    }

    /// Check if an action target is allowed through the boundary.
    /// Returns `BoundaryResult::Blocked` if the target hits a hard block.
    pub fn check(&self, target: &str) -> BoundaryResult {
        let lower = target.to_lowercase();

        // Check blocked paths
        for path in &self.blocked_paths {
            let path_lower = path.to_lowercase();
            if lower.starts_with(&path_lower) || lower.contains(&path_lower) {
                return BoundaryResult::Blocked(BoundaryViolation {
                    rule_name: "blocked_path".into(),
                    reason: format!("Path '{}' is in a protected system directory", path),
                    target: target.to_string(),
                });
            }
        }

        // Check blocked patterns
        for pattern in &self.blocked_patterns {
            let matched = match &pattern.check {
                BlockedCheck::PathPrefix(prefix) => {
                    let prefix_lower = prefix.to_lowercase();
                    lower.starts_with(&prefix_lower)
                }
                BlockedCheck::TargetContains(substring) => {
                    let sub_lower = substring.to_lowercase();
                    lower.contains(&sub_lower)
                }
                BlockedCheck::ActionType(action_type) => lower == action_type.to_lowercase(),
            };

            if matched {
                return BoundaryResult::Blocked(BoundaryViolation {
                    rule_name: pattern.name.to_string(),
                    reason: pattern.description.to_string(),
                    target: target.to_string(),
                });
            }
        }

        BoundaryResult::Allowed
    }

    /// Add a custom blocked path
    pub fn add_blocked_path(&mut self, path: impl Into<String>) {
        self.blocked_paths.push(path.into());
    }

    /// Add a custom blocked pattern
    pub fn add_blocked_pattern(&mut self, pattern: BlockedPattern) {
        self.blocked_patterns.push(pattern);
    }

    /// Get all blocked paths (for inspection/testing)
    pub fn blocked_paths(&self) -> &[String] {
        &self.blocked_paths
    }

    /// Get all blocked patterns (for inspection/testing)
    pub fn blocked_patterns(&self) -> &[BlockedPattern] {
        &self.blocked_patterns
    }
}

impl Default for BoundaryEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_paths_blocked() {
        let enforcer = BoundaryEnforcer::new();
        let cases = vec![
            "/etc/passwd",
            "/System/Library/config",
            "/usr/bin/rm",
            "~/.ssh/id_rsa",
            ".ssh/authorized_keys",
        ];
        for path in cases {
            match enforcer.check(path) {
                BoundaryResult::Blocked(v) => {
                    assert_eq!(
                        v.rule_name, "blocked_path",
                        "Expected blocked_path for {path}"
                    );
                }
                BoundaryResult::Allowed => panic!("{path} should be blocked"),
            }
        }
    }

    #[test]
    fn test_safe_paths_allowed() {
        let enforcer = BoundaryEnforcer::new();
        let cases = vec!["src/main.rs", "/home/user/project/lib.rs", "/tmp/test.txt"];
        for path in cases {
            assert!(
                matches!(enforcer.check(path), BoundaryResult::Allowed),
                "{path} should be allowed"
            );
        }
    }

    #[test]
    fn test_self_modification_blocked() {
        let enforcer = BoundaryEnforcer::new();
        let cases = vec![
            "hydra-gate/src/gate.rs",
            "hydra-kernel/src/cognitive_loop.rs",
            "hydra-core/src/types.rs",
        ];
        for path in cases {
            assert!(
                matches!(enforcer.check(path), BoundaryResult::Blocked(_)),
                "{path} should be blocked"
            );
        }
    }

    #[test]
    fn test_email_blocked() {
        let enforcer = BoundaryEnforcer::new();
        assert!(matches!(
            enforcer.check("send_email"),
            BoundaryResult::Blocked(_)
        ));
    }

    #[test]
    fn test_payment_blocked() {
        let enforcer = BoundaryEnforcer::new();
        assert!(matches!(
            enforcer.check("process_payment"),
            BoundaryResult::Blocked(_)
        ));
    }

    #[test]
    fn test_destructive_rm_blocked() {
        let enforcer = BoundaryEnforcer::new();
        assert!(matches!(
            enforcer.check("rm -rf /"),
            BoundaryResult::Blocked(_)
        ));
    }

    #[test]
    fn test_custom_blocked_path() {
        let mut enforcer = BoundaryEnforcer::new();
        enforcer.add_blocked_path("/custom/protected/");
        assert!(matches!(
            enforcer.check("/custom/protected/file.txt"),
            BoundaryResult::Blocked(_)
        ));
    }

    #[test]
    fn test_case_insensitive() {
        let enforcer = BoundaryEnforcer::new();
        assert!(matches!(
            enforcer.check("/ETC/passwd"),
            BoundaryResult::Blocked(_)
        ));
        assert!(matches!(
            enforcer.check("/SYSTEM/Library"),
            BoundaryResult::Blocked(_)
        ));
    }
}
