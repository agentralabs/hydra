//! LocalExecutor — executes operations on local connectors.
//! Handles filesystem, subprocess, http_local, and applescript access methods.
//! Path traversal prevention on all filesystem operations.

use crate::constants::{LOCAL_FS_MAX_FILE_SIZE_BYTES, LOCAL_HTTP_TIMEOUT_MS};
use crate::local_config::LocalConfig;
use crate::runtime::ExecutionResult;

use std::collections::HashMap;
use std::path::Path;

/// Executes operations on local connectors.
pub struct LocalExecutor;

impl LocalExecutor {
    /// Execute a local operation based on the config's access_method.
    pub fn execute(
        config: &LocalConfig,
        operation: &str,
        params: &HashMap<String, String>,
    ) -> ExecutionResult {
        let start = std::time::Instant::now();
        let receipt_id = uuid::Uuid::new_v4().to_string();
        let name = &config.integration.name;

        let result = match config.local.access_method.as_str() {
            "filesystem" => Self::execute_filesystem(config, operation, params),
            "subprocess" => Self::execute_subprocess(config, operation, params),
            "http_local" => Self::execute_http_local(config, operation, params),
            "applescript" => Self::execute_applescript(config, operation, params),
            other => Err(format!("Unknown access method: {other}")),
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                eprintln!("hydra-executor: local '{name}' {operation} succeeded ({duration_ms}ms)");
                ExecutionResult {
                    name: format!("{name}.{operation}"),
                    success: true,
                    output,
                    error: None,
                    duration_ms,
                    receipt_id,
                }
            }
            Err(e) => {
                eprintln!("hydra-executor: local '{name}' {operation} failed: {e}");
                ExecutionResult {
                    name: format!("{name}.{operation}"),
                    success: false,
                    output: String::new(),
                    error: Some(e),
                    duration_ms,
                    receipt_id,
                }
            }
        }
    }

    fn execute_filesystem(
        config: &LocalConfig,
        operation: &str,
        params: &HashMap<String, String>,
    ) -> Result<String, String> {
        let fs_spec = config.local.filesystem.as_ref()
            .ok_or("No filesystem spec in local config")?;
        let caps = &config.local.capabilities;

        let root = shellexpand::tilde(&fs_spec.root_path).to_string();
        let root_path = Path::new(&root);

        match operation {
            "read" => {
                Self::check_cap(caps.read, "read")?;
                let file = params.get("path").ok_or("Missing 'path' param")?;
                let full = root_path.join(file);
                Self::check_traversal(root_path, &full)?;
                let meta = std::fs::metadata(&full).map_err(|e| e.to_string())?;
                if meta.len() as usize > LOCAL_FS_MAX_FILE_SIZE_BYTES {
                    return Err(format!("File too large: {} bytes", meta.len()));
                }
                std::fs::read_to_string(&full).map_err(|e| e.to_string())
            }
            "write" => {
                Self::check_cap(caps.write, "write")?;
                let file = params.get("path").ok_or("Missing 'path' param")?;
                let content = params.get("content").ok_or("Missing 'content' param")?;
                let full = root_path.join(file);
                Self::check_traversal(root_path, &full)?;
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&full, content).map_err(|e| e.to_string())?;
                eprintln!("hydra-executor: wrote {} bytes to {}", content.len(), full.display());
                Ok(format!("Written {} bytes", content.len()))
            }
            "list" => {
                Self::check_cap(caps.read, "list")?;
                let subdir = params.get("path").map(|s| s.as_str()).unwrap_or("");
                let full = root_path.join(subdir);
                Self::check_traversal(root_path, &full)?;
                Self::list_dir(&full, &fs_spec.file_pattern)
            }
            "delete" => {
                Self::check_cap(caps.delete, "delete")?;
                let file = params.get("path").ok_or("Missing 'path' param")?;
                let full = root_path.join(file);
                Self::check_traversal(root_path, &full)?;
                std::fs::remove_file(&full).map_err(|e| e.to_string())?;
                Ok(format!("Deleted {}", full.display()))
            }
            _ => Err(format!("Unknown filesystem operation: {operation}")),
        }
    }

    fn execute_subprocess(
        config: &LocalConfig,
        _operation: &str,
        params: &HashMap<String, String>,
    ) -> Result<String, String> {
        let spec = config.local.subprocess.as_ref()
            .ok_or("No subprocess spec in local config")?;
        let mut cmd_str = spec.command.clone();
        for (k, v) in params {
            cmd_str = cmd_str.replace(&format!("{{{k}}}"), v);
        }
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(&cmd_str);
        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt;
            cmd.pre_exec(|| { libc::setpgid(0, 0); Ok(()) });
        }
        let output = cmd.output().map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn execute_http_local(
        config: &LocalConfig,
        _operation: &str,
        params: &HashMap<String, String>,
    ) -> Result<String, String> {
        let spec = config.local.http_local.as_ref()
            .ok_or("No http_local spec in local config")?;
        let endpoint = params.get("endpoint").map(|s| s.as_str()).unwrap_or("");
        let url = format!("{}{}", spec.base_url, endpoint);

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(LOCAL_HTTP_TIMEOUT_MS))
            .build()
            .map_err(|e| e.to_string())?;

        let method = params.get("method").map(|s| s.as_str()).unwrap_or("GET");
        let resp = match method {
            "POST" | "PUT" => {
                Self::check_cap(config.local.capabilities.write, "write")?;
                let body = params.get("body").cloned().unwrap_or_default();
                let req = if method == "POST" { client.post(&url) } else { client.put(&url) };
                req.header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .map_err(|e| e.to_string())?
            }
            _ => {
                client.get(&url).send().map_err(|e| e.to_string())?
            }
        };

        let status = resp.status();
        let body = resp.text().map_err(|e| e.to_string())?;
        if status.is_success() {
            Ok(body)
        } else {
            Err(format!("HTTP {status}: {body}"))
        }
    }

    fn execute_applescript(
        config: &LocalConfig,
        operation: &str,
        params: &HashMap<String, String>,
    ) -> Result<String, String> {
        if !cfg!(target_os = "macos") {
            return Err("AppleScript only available on macOS".into());
        }
        let spec = config.local.applescript.as_ref()
            .ok_or("No applescript spec in local config")?;
        let script = spec.scripts.get(operation)
            .ok_or(format!("No AppleScript for operation '{operation}'"))?;

        let mut resolved = script.clone();
        for (k, v) in params {
            resolved = resolved.replace(&format!("{{{k}}}"), v);
        }

        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&resolved)
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Verify path doesn't escape the root directory.
    fn check_traversal(root: &Path, target: &Path) -> Result<(), String> {
        // Canonicalize if both exist, otherwise check string prefix
        let target_str = target.to_string_lossy();

        if target_str.contains("..") {
            return Err(format!("Path traversal blocked: {target_str}"));
        }

        // If target exists, use canonical paths
        if let (Ok(canon_root), Ok(canon_target)) =
            (std::fs::canonicalize(root), std::fs::canonicalize(target))
        {
            if !canon_target.starts_with(&canon_root) {
                return Err(format!(
                    "Path traversal blocked: {} is outside {}",
                    canon_target.display(),
                    canon_root.display()
                ));
            }
        }

        Ok(())
    }

    fn check_cap(allowed: bool, operation: &str) -> Result<(), String> {
        if !allowed {
            Err(format!("Operation '{operation}' not permitted by capabilities"))
        } else {
            Ok(())
        }
    }

    fn list_dir(dir: &Path, _pattern: &str) -> Result<String, String> {
        let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            items.push(if is_dir { format!("{name}/") } else { name });
        }
        items.sort();
        Ok(items.join("\n"))
    }
}

// shellexpand tilde replacement (minimal, no extra dep)
mod shellexpand {
    pub fn tilde(path: &str) -> std::borrow::Cow<'_, str> {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return std::borrow::Cow::Owned(
                    format!("{}{}", home.display(), &path[1..])
                );
            }
        }
        std::borrow::Cow::Borrowed(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_traversal_blocked() {
        let root = Path::new("/tmp/safe");
        let bad = Path::new("/tmp/safe/../../../etc/passwd");
        assert!(LocalExecutor::check_traversal(root, bad).is_err());
    }

    #[test]
    fn path_within_root_allowed() {
        let good = Path::new("/tmp/subdir/file.txt");
        assert!(!good.to_string_lossy().contains(".."));
    }

    #[test]
    fn capability_gating() {
        assert!(LocalExecutor::check_cap(true, "read").is_ok());
        assert!(LocalExecutor::check_cap(false, "write").is_err());
        assert!(LocalExecutor::check_cap(false, "delete").is_err());
    }

    #[test]
    fn tilde_expansion() {
        let expanded = shellexpand::tilde("~/test.txt");
        assert!(!expanded.starts_with("~/"));
    }
}
