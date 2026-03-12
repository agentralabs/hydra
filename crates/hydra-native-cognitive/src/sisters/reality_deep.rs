//! Priority 4: Deep Reality Integration — sister-first environment probing,
//! merged with local probes for a richer picture.

use super::connection::extract_text;
use super::cognitive::Sisters;

/// Combined environment profile from Reality sister + local probes.
#[derive(Debug, Clone, Default)]
pub struct EnvironmentProfile {
    pub os: String,
    pub arch: String,
    pub tools: Vec<String>,
    pub services: Vec<String>,
    pub cloud_provider: Option<String>,
    pub container_runtime: Option<String>,
    pub sister_context: Option<String>,
}

impl Sisters {
    /// PERCEIVE: Full environment probe via Reality sister, merged with local data.
    pub async fn reality_probe_environment(&self) -> EnvironmentProfile {
        let mut profile = local_probe();

        // Sister-first: enrich with Reality sister's deeper context
        if let Some(reality) = &self.reality {
            if let Ok(result) = reality.call_tool("reality_probe_environment", serde_json::json!({
                "include": ["os", "tools", "services", "cloud", "container"],
            })).await {
                let text = extract_text(&result);
                if !text.is_empty() {
                    profile.sister_context = Some(text);
                }
                // Extract cloud/container if present
                if let Some(cloud) = result.get("cloud_provider").and_then(|v| v.as_str()) {
                    profile.cloud_provider = Some(cloud.to_string());
                }
                if let Some(container) = result.get("container_runtime").and_then(|v| v.as_str()) {
                    profile.container_runtime = Some(container.to_string());
                }
                // Merge discovered tools
                if let Some(tools) = result.get("tools").and_then(|v| v.as_array()) {
                    for tool in tools {
                        if let Some(name) = tool.as_str() {
                            if !profile.tools.contains(&name.to_string()) {
                                profile.tools.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        profile
    }

    /// PERCEIVE: Assess capabilities for a specific project type.
    pub async fn reality_assess_capabilities(&self, project_type: &str) -> Option<String> {
        let reality = self.reality.as_ref()?;
        let result = reality.call_tool("reality_assess_capabilities", serde_json::json!({
            "project_type": project_type,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get current context from Reality sister for the /env command.
    pub async fn reality_get_context(&self) -> Option<String> {
        let reality = self.reality.as_ref()?;
        let result = reality.call_tool("reality_context", serde_json::json!({
            "input": "environment status",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

/// Local environment probe — runs even when Reality sister is offline.
fn local_probe() -> EnvironmentProfile {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();

    let mut tools = Vec::new();
    let check_tools = ["git", "cargo", "node", "npm", "python3", "go", "docker",
        "kubectl", "terraform", "aws", "gcloud", "az"];
    for tool in &check_tools {
        if which_exists(tool) {
            tools.push(tool.to_string());
        }
    }

    let mut services = Vec::new();
    let check_ports = [(5432, "PostgreSQL"), (3306, "MySQL"), (6379, "Redis"),
        (27017, "MongoDB"), (8080, "HTTP"), (3000, "Dev Server")];
    for (port, name) in &check_ports {
        if port_listening(*port) {
            services.push(name.to_string());
        }
    }

    EnvironmentProfile {
        os, arch, tools, services,
        cloud_provider: None,
        container_runtime: if which_exists("docker") {
            Some("Docker".into())
        } else { None },
        sister_context: None,
    }
}

fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn port_listening(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        std::time::Duration::from_millis(50),
    ).is_ok()
}

impl EnvironmentProfile {
    /// Human-readable summary for /env command.
    pub fn summary(&self) -> String {
        let mut out = format!("OS: {} ({})\n", self.os, self.arch);
        if !self.tools.is_empty() {
            out.push_str(&format!("Tools: {}\n", self.tools.join(", ")));
        }
        if !self.services.is_empty() {
            out.push_str(&format!("Services: {}\n", self.services.join(", ")));
        }
        if let Some(ref cloud) = self.cloud_provider {
            out.push_str(&format!("Cloud: {}\n", cloud));
        }
        if let Some(ref container) = self.container_runtime {
            out.push_str(&format!("Container: {}\n", container));
        }
        if let Some(ref ctx) = self.sister_context {
            out.push_str(&format!("Reality Context: {}\n", ctx));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_probe_runs() {
        let profile = local_probe();
        assert!(!profile.os.is_empty());
        assert!(!profile.arch.is_empty());
    }

    #[test]
    fn test_profile_summary() {
        let p = EnvironmentProfile {
            os: "macos".into(), arch: "aarch64".into(),
            tools: vec!["git".into(), "cargo".into()],
            services: vec!["PostgreSQL".into()],
            cloud_provider: Some("AWS".into()),
            container_runtime: Some("Docker".into()),
            sister_context: None,
        };
        let s = p.summary();
        assert!(s.contains("macos"));
        assert!(s.contains("git"));
        assert!(s.contains("PostgreSQL"));
        assert!(s.contains("AWS"));
    }
}
