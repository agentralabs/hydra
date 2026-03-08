//! Sandbox isolation for skill execution.

use crate::definition::SandboxLevel;

/// Sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub allow_network: bool,
    pub allow_filesystem: bool,
    pub temp_dir_only: bool,
    pub max_memory_mb: u64,
    pub max_duration_secs: u64,
}

/// Sandbox enforcement
pub struct Sandbox {
    level: SandboxLevel,
    config: SandboxConfig,
}

impl Sandbox {
    /// Create a sandbox for the given level
    pub fn for_level(level: SandboxLevel) -> Self {
        let config = match level {
            SandboxLevel::None => SandboxConfig {
                allow_network: true,
                allow_filesystem: true,
                temp_dir_only: false,
                max_memory_mb: 0, // unlimited
                max_duration_secs: 300,
            },
            SandboxLevel::Basic => SandboxConfig {
                allow_network: true,
                allow_filesystem: true,
                temp_dir_only: true,
                max_memory_mb: 512,
                max_duration_secs: 60,
            },
            SandboxLevel::Strict => SandboxConfig {
                allow_network: false,
                allow_filesystem: false,
                temp_dir_only: false,
                max_memory_mb: 128,
                max_duration_secs: 30,
            },
        };

        Self { level, config }
    }

    pub fn level(&self) -> SandboxLevel {
        self.level
    }

    pub fn allows_network(&self) -> bool {
        self.config.allow_network
    }

    pub fn allows_filesystem(&self) -> bool {
        self.config.allow_filesystem
    }

    pub fn temp_dir_only(&self) -> bool {
        self.config.temp_dir_only
    }

    pub fn max_memory_mb(&self) -> u64 {
        self.config.max_memory_mb
    }

    pub fn max_duration_secs(&self) -> u64 {
        self.config.max_duration_secs
    }

    /// Check if an operation is allowed
    pub fn check_operation(&self, op: &SandboxOp) -> bool {
        match op {
            SandboxOp::Network => self.config.allow_network,
            SandboxOp::ReadFile(path) => {
                if !self.config.allow_filesystem {
                    return false;
                }
                if self.config.temp_dir_only {
                    return path.starts_with("/tmp") || path.starts_with("/var/tmp");
                }
                true
            }
            SandboxOp::WriteFile(path) => {
                if !self.config.allow_filesystem {
                    return false;
                }
                if self.config.temp_dir_only {
                    return path.starts_with("/tmp") || path.starts_with("/var/tmp");
                }
                true
            }
            SandboxOp::Execute => self.level == SandboxLevel::None,
        }
    }
}

/// Operations that can be sandboxed
#[derive(Debug)]
pub enum SandboxOp {
    Network,
    ReadFile(String),
    WriteFile(String),
    Execute,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_none() {
        let sandbox = Sandbox::for_level(SandboxLevel::None);
        assert!(sandbox.allows_network());
        assert!(sandbox.allows_filesystem());
        assert!(sandbox.check_operation(&SandboxOp::Network));
        assert!(sandbox.check_operation(&SandboxOp::ReadFile("/etc/passwd".into())));
        assert!(sandbox.check_operation(&SandboxOp::Execute));
    }

    #[test]
    fn test_sandbox_basic() {
        let sandbox = Sandbox::for_level(SandboxLevel::Basic);
        assert!(sandbox.allows_network());
        assert!(sandbox.allows_filesystem());
        assert!(sandbox.temp_dir_only());
        // Can read temp
        assert!(sandbox.check_operation(&SandboxOp::ReadFile("/tmp/data.txt".into())));
        // Cannot read outside temp
        assert!(!sandbox.check_operation(&SandboxOp::ReadFile("/etc/passwd".into())));
        // Cannot execute
        assert!(!sandbox.check_operation(&SandboxOp::Execute));
    }

    #[test]
    fn test_sandbox_strict() {
        let sandbox = Sandbox::for_level(SandboxLevel::Strict);
        assert!(!sandbox.allows_network());
        assert!(!sandbox.allows_filesystem());
        assert!(!sandbox.check_operation(&SandboxOp::Network));
        assert!(!sandbox.check_operation(&SandboxOp::ReadFile("/tmp/data.txt".into())));
        assert!(!sandbox.check_operation(&SandboxOp::Execute));
        assert_eq!(sandbox.max_memory_mb(), 128);
        assert_eq!(sandbox.max_duration_secs(), 30);
    }

    #[test]
    fn test_sandbox_none_level() {
        let sandbox = Sandbox::for_level(SandboxLevel::None);
        assert_eq!(sandbox.level(), SandboxLevel::None);
    }

    #[test]
    fn test_sandbox_basic_level() {
        let sandbox = Sandbox::for_level(SandboxLevel::Basic);
        assert_eq!(sandbox.level(), SandboxLevel::Basic);
    }

    #[test]
    fn test_sandbox_strict_level() {
        let sandbox = Sandbox::for_level(SandboxLevel::Strict);
        assert_eq!(sandbox.level(), SandboxLevel::Strict);
    }

    #[test]
    fn test_sandbox_none_unlimited_memory() {
        let sandbox = Sandbox::for_level(SandboxLevel::None);
        assert_eq!(sandbox.max_memory_mb(), 0); // unlimited
    }

    #[test]
    fn test_sandbox_basic_memory() {
        let sandbox = Sandbox::for_level(SandboxLevel::Basic);
        assert_eq!(sandbox.max_memory_mb(), 512);
    }

    #[test]
    fn test_sandbox_none_duration() {
        let sandbox = Sandbox::for_level(SandboxLevel::None);
        assert_eq!(sandbox.max_duration_secs(), 300);
    }

    #[test]
    fn test_sandbox_basic_write_temp() {
        let sandbox = Sandbox::for_level(SandboxLevel::Basic);
        assert!(sandbox.check_operation(&SandboxOp::WriteFile("/tmp/output.txt".into())));
    }

    #[test]
    fn test_sandbox_basic_write_outside_temp() {
        let sandbox = Sandbox::for_level(SandboxLevel::Basic);
        assert!(!sandbox.check_operation(&SandboxOp::WriteFile("/home/user/data.txt".into())));
    }

    #[test]
    fn test_sandbox_basic_var_tmp() {
        let sandbox = Sandbox::for_level(SandboxLevel::Basic);
        assert!(sandbox.check_operation(&SandboxOp::ReadFile("/var/tmp/data.txt".into())));
    }

    #[test]
    fn test_sandbox_strict_write_denied() {
        let sandbox = Sandbox::for_level(SandboxLevel::Strict);
        assert!(!sandbox.check_operation(&SandboxOp::WriteFile("/tmp/file.txt".into())));
    }

    #[test]
    fn test_sandbox_none_not_temp_only() {
        let sandbox = Sandbox::for_level(SandboxLevel::None);
        assert!(!sandbox.temp_dir_only());
    }
}
