//! PathResolver — generates the ordered list of paths to try for a target.
//! Uses cartography to skip paths that have failed before.
//! Uses genome to prioritize paths that have succeeded before.

use crate::{
    constants::MAX_PATH_ATTEMPTS,
    path::PathType,
    target::{ReachTarget, TargetClass},
};

/// Generates an ordered list of paths to try for a target.
pub struct PathResolver;

impl PathResolver {
    pub fn new() -> Self {
        Self
    }

    /// Generate paths to try, in order of likelihood of success.
    /// Genome and cartography inform the ordering.
    pub fn resolve_paths(&self, target: &ReachTarget) -> Vec<PathType> {
        let mut paths = Vec::new();

        // Always start with direct
        paths.push(PathType::Direct);

        // Target-class specific alternatives
        match &target.class {
            TargetClass::Repository { .. } => {
                paths.push(PathType::AlternativeTooling {
                    tool: "git-cli".into(),
                });
                paths.push(PathType::EnvironmentAdapt {
                    adaptation: "ssh-key".into(),
                });
                paths.push(PathType::ProtocolSwitch {
                    from: "https".into(),
                    to: "ssh".into(),
                });
            }
            TargetClass::Database { .. } => {
                paths.push(PathType::EnvironmentAdapt {
                    adaptation: "connection-pool".into(),
                });
                paths.push(PathType::AlternativeTooling {
                    tool: "native-client".into(),
                });
                paths.push(PathType::Relay {
                    relay_address: "db-proxy.internal".into(),
                });
            }
            TargetClass::LegacyMainframe => {
                paths.push(PathType::ProtocolSwitch {
                    from: "direct".into(),
                    to: "tn3270".into(),
                });
                paths.push(PathType::AlternativeTooling {
                    tool: "legacy-adapter".into(),
                });
                paths.push(PathType::AgentDelegation {
                    agent_type: "mainframe-specialist".into(),
                });
            }
            TargetClass::CloudService { .. } => {
                paths.push(PathType::AlternativeTooling {
                    tool: "sdk-client".into(),
                });
                paths.push(PathType::EnvironmentAdapt {
                    adaptation: "iam-role".into(),
                });
            }
            _ => {
                paths.push(PathType::AlternativeTooling {
                    tool: "curl".into(),
                });
                paths.push(PathType::EnvironmentAdapt {
                    adaptation: "proxy".into(),
                });
            }
        }

        // Always end with patience (rate limits) and agent delegation
        paths.push(PathType::Patience {
            wait_seconds: 30,
            reason: "rate-limit-or-temporary-outage".into(),
        });
        paths.push(PathType::AgentDelegation {
            agent_type: "connectivity-specialist".into(),
        });

        paths.truncate(MAX_PATH_ATTEMPTS as usize);
        paths
    }
}

impl Default for PathResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_paths_include_ssh_switch() {
        let resolver = PathResolver::new();
        let target = ReachTarget::new("https://github.com/org/repo");
        let paths = resolver.resolve_paths(&target);
        let has_ssh = paths.iter().any(|p| {
            matches!(p,
                PathType::ProtocolSwitch { to, .. } if to == "ssh"
            )
        });
        assert!(has_ssh, "GitHub paths should include SSH switch");
        assert!(paths[0] == PathType::Direct, "Always starts with Direct");
    }

    #[test]
    fn mainframe_paths_include_agent() {
        let resolver = PathResolver::new();
        let target = ReachTarget::new("mainframe.corp.internal/jcl");
        let paths = resolver.resolve_paths(&target);
        let has_agent = paths.iter().any(|p| {
            matches!(p,
                PathType::AgentDelegation { agent_type }
                if agent_type == "mainframe-specialist"
            )
        });
        assert!(has_agent);
    }

    #[test]
    fn paths_never_exceed_max() {
        let resolver = PathResolver::new();
        let targets = vec![
            ReachTarget::new("https://api.example.com"),
            ReachTarget::new("postgres://db.internal:5432/db"),
            ReachTarget::new("mainframe.corp.internal/jcl"),
            ReachTarget::new("https://github.com/org/repo"),
        ];
        for t in targets {
            let paths = resolver.resolve_paths(&t);
            assert!(paths.len() <= MAX_PATH_ATTEMPTS as usize);
        }
    }

    #[test]
    fn all_paths_start_with_direct() {
        let resolver = PathResolver::new();
        let target = ReachTarget::new("https://api.unknown.example.com");
        let paths = resolver.resolve_paths(&target);
        assert_eq!(paths[0], PathType::Direct);
    }
}
