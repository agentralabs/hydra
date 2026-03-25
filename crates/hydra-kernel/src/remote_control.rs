//! O41: Multi-Machine Control — use any computer Hydra can SSH to.
//!
//! Extends the AMM + atomic input algebra to work over SSH.
//! Screenshot: ssh machine "screencapture -x /tmp/frame.png" + scp back.
//! Input: ssh machine "cliclick c:x,y" or ssh machine "xdotool click 1".
//! First contact protocol works remotely — discover menus, tools, shortcuts.

use crate::remote_exec::ssh_execute;

/// A remote machine that Hydra can control.
#[derive(Debug, Clone)]
pub struct RemoteMachine {
    pub name: String,
    pub host: String,
    pub os: RemoteOS,
}

#[derive(Debug, Clone)]
pub enum RemoteOS { MacOS, Linux, Unknown }

impl RemoteMachine {
    /// Detect OS of remote machine.
    pub fn detect_os(name: &str) -> Self {
        let (uname, _) = ssh_execute(name, "uname -s").unwrap_or_default();
        let os = if uname.trim().contains("Darwin") { RemoteOS::MacOS }
            else if uname.trim().contains("Linux") { RemoteOS::Linux }
            else { RemoteOS::Unknown };
        let host = uname.trim().to_string(); // simplified
        eprintln!("hydra-remote: {name} detected as {:?}", os);
        Self { name: name.into(), host, os }
    }

    /// Capture screenshot from remote machine.
    pub fn screenshot(&self) -> Result<Vec<u8>, String> {
        let remote_path = "/tmp/hydra-remote-screenshot.png";
        let capture_cmd = match self.os {
            RemoteOS::MacOS => format!("screencapture -x -t png {remote_path}"),
            RemoteOS::Linux => format!("gnome-screenshot -f {remote_path} 2>/dev/null || scrot {remote_path}"),
            RemoteOS::Unknown => return Err("Unknown remote OS".into()),
        };
        let (_, success) = ssh_execute(&self.name, &capture_cmd)
            .map_err(|e| format!("Remote capture failed: {e}"))?;
        if !success { return Err("Screenshot command failed".into()); }

        // SCP the file back
        let local_path = format!("/tmp/hydra-remote-{}.png", self.name);
        let scp = std::process::Command::new("scp")
            .args([&format!("{}:{remote_path}", self.name), &local_path])
            .output().map_err(|e| format!("SCP failed: {e}"))?;
        if !scp.status.success() { return Err("SCP failed".into()); }

        std::fs::read(&local_path).map_err(|e| format!("Read failed: {e}"))
    }

    /// Execute an input atom on the remote machine.
    pub fn execute_input(&self, action: &str) -> Result<String, String> {
        let cmd = match self.os {
            RemoteOS::MacOS => format!("cliclick {action}"),
            RemoteOS::Linux => {
                // Translate cliclick syntax to xdotool
                if action.starts_with("c:") {
                    let coords = &action[2..];
                    let parts: Vec<&str> = coords.split(',').collect();
                    if parts.len() == 2 {
                        format!("xdotool mousemove {} {} click 1", parts[0], parts[1])
                    } else { format!("xdotool {action}") }
                } else { format!("xdotool {action}") }
            }
            RemoteOS::Unknown => return Err("Unknown OS".into()),
        };
        let (output, success) = ssh_execute(&self.name, &cmd)
            .map_err(|e| format!("Remote input failed: {e}"))?;
        if success { Ok(output) } else { Err(output) }
    }

    /// Click at coordinates on remote machine.
    pub fn click(&self, x: i32, y: i32) -> Result<(), String> {
        self.execute_input(&format!("c:{x},{y}"))?;
        Ok(())
    }

    /// Type text on remote machine.
    pub fn type_text(&self, text: &str) -> Result<(), String> {
        let cmd = match self.os {
            RemoteOS::MacOS => format!(
                "osascript -e 'tell application \"System Events\" to keystroke \"{}\"'",
                text.replace('"', "\\\"")),
            RemoteOS::Linux => format!("xdotool type --clearmodifiers '{}'", text.replace('\'', "'\\''")),
            RemoteOS::Unknown => return Err("Unknown OS".into()),
        };
        let (_, success) = ssh_execute(&self.name, &cmd)
            .map_err(|e| format!("Remote type failed: {e}"))?;
        if success { Ok(()) } else { Err("Type command failed".into()) }
    }

    /// Key press on remote machine.
    pub fn key_press(&self, key: &str) -> Result<(), String> {
        let cmd = match self.os {
            RemoteOS::MacOS => format!("osascript -e 'tell application \"System Events\" to key code {}'",
                match key { "enter" => "36", "tab" => "48", "escape" => "53", _ => "36" }),
            RemoteOS::Linux => format!("xdotool key {}", match key {
                "enter" => "Return", "tab" => "Tab", "escape" => "Escape", k => k }),
            RemoteOS::Unknown => return Err("Unknown OS".into()),
        };
        ssh_execute(&self.name, &cmd).map_err(|e| format!("{e}"))?;
        Ok(())
    }

    /// List machines from config.
    pub fn list_machines() -> Vec<String> {
        let path = dirs::home_dir().unwrap_or_default().join(".hydra/machines.toml");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let table: toml::Table = content.parse().unwrap_or_default();
        table.get("machines").and_then(|m| m.as_array())
            .map(|arr| arr.iter().filter_map(|v|
                v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string())
            ).collect())
            .unwrap_or_default()
    }
}
