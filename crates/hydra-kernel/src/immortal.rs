//! O42: Immortal Daemon — Hydra survives reboots and auto-repairs.
//!
//! macOS: launchd plist (already exists at com.agentra.hydra.plist).
//! Linux: systemd unit file generation.
//! Auto-repair: on startup, check for crash markers and restore state.

/// Install Hydra as a system daemon that auto-starts on boot.
pub fn install_daemon() -> Result<String, String> {
    if cfg!(target_os = "macos") { install_macos() }
    else if cfg!(target_os = "linux") { install_linux() }
    else { Err("Unsupported platform for daemon install".into()) }
}

/// Check if Hydra daemon is installed.
pub fn is_installed() -> bool {
    if cfg!(target_os = "macos") {
        let plist = dirs::home_dir().unwrap_or_default()
            .join("Library/LaunchAgents/com.agentra.hydra.plist");
        plist.exists()
    } else if cfg!(target_os = "linux") {
        let unit = dirs::home_dir().unwrap_or_default()
            .join(".config/systemd/user/hydra.service");
        unit.exists()
    } else { false }
}

/// Check for crash markers and restore state.
pub fn auto_repair() {
    let crash_marker = dirs::home_dir().unwrap_or_default().join(".hydra/CRASH");
    if crash_marker.exists() {
        eprintln!("hydra-immortal: crash marker detected — auto-repairing");
        // Clear stale locks
        let lock = dirs::home_dir().unwrap_or_default().join(".hydra/hydra.lock");
        if lock.exists() {
            let _ = std::fs::remove_file(&lock);
            eprintln!("hydra-immortal: stale lock cleared");
        }
        // Clear crash marker
        let _ = std::fs::remove_file(&crash_marker);
        eprintln!("hydra-immortal: repair complete — resuming normal operation");
    }
}

/// Write crash marker (called from panic hook).
pub fn mark_crash(reason: &str) {
    let path = dirs::home_dir().unwrap_or_default().join(".hydra/CRASH");
    let content = format!("crash at {}\nreason: {reason}", chrono::Utc::now());
    let _ = std::fs::write(path, content);
}

fn install_macos() -> Result<String, String> {
    let hydra_bin = std::env::current_exe()
        .map_err(|e| format!("Can't find hydra binary: {e}"))?;
    let plist_dir = dirs::home_dir().unwrap_or_default().join("Library/LaunchAgents");
    let _ = std::fs::create_dir_all(&plist_dir);
    let plist_path = plist_dir.join("com.agentra.hydra.plist");
    let log_dir = dirs::home_dir().unwrap_or_default().join(".hydra/logs");
    let _ = std::fs::create_dir_all(&log_dir);

    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>com.agentra.hydra</string>
    <key>ProgramArguments</key><array>
        <string>{}</string>
        <string>--daemon</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
    <key>StandardOutPath</key><string>{}/hydra.stdout.log</string>
    <key>StandardErrorPath</key><string>{}/hydra.stderr.log</string>
    <key>Nice</key><integer>10</integer>
    <key>LowPriorityIO</key><true/>
</dict>
</plist>"#, hydra_bin.display(), log_dir.display(), log_dir.display());

    std::fs::write(&plist_path, plist).map_err(|e| format!("Write plist: {e}"))?;
    // Load the daemon
    let _ = std::process::Command::new("launchctl")
        .args(["load", &plist_path.to_string_lossy()]).output();
    eprintln!("hydra-immortal: daemon installed (macOS launchd)");
    Ok(format!("Installed at {}", plist_path.display()))
}

fn install_linux() -> Result<String, String> {
    let hydra_bin = std::env::current_exe()
        .map_err(|e| format!("Can't find hydra binary: {e}"))?;
    let unit_dir = dirs::home_dir().unwrap_or_default().join(".config/systemd/user");
    let _ = std::fs::create_dir_all(&unit_dir);
    let unit_path = unit_dir.join("hydra.service");

    let unit = format!(r#"[Unit]
Description=Hydra Autonomous Agent
After=network.target

[Service]
Type=simple
ExecStart={} --daemon
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
"#, hydra_bin.display());

    std::fs::write(&unit_path, unit).map_err(|e| format!("Write unit: {e}"))?;
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"]).output();
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "enable", "--now", "hydra"]).output();
    eprintln!("hydra-immortal: daemon installed (Linux systemd)");
    Ok(format!("Installed at {}", unit_path.display()))
}
