//! Platform abstraction layer — one function per action type.
//! Cross-platform commands for macOS, Linux, and Windows.

/// Open a new terminal window.
pub(crate) fn platform_new_terminal() -> String {
    if cfg!(target_os = "macos") {
        "osascript -e 'tell application \"Terminal\" to do script \"\"' -e 'tell application \"Terminal\" to activate'".into()
    } else if cfg!(target_os = "windows") {
        "start cmd".into()
    } else {
        "gnome-terminal 2>/dev/null || konsole 2>/dev/null || xfce4-terminal 2>/dev/null || xterm 2>/dev/null".into()
    }
}

pub(crate) fn platform_new_tab(browser: &str) -> String {
    if cfg!(target_os = "macos") {
        match browser {
            "firefox" => "open -a Firefox 'about:blank'".into(),
            "safari" => "osascript -e 'tell application \"Safari\" to activate' -e 'tell application \"System Events\" to keystroke \"t\" using command down'".into(),
            _ => "open -a 'Google Chrome' 'about:blank'".into(),
        }
    } else if cfg!(target_os = "windows") {
        format!("start {} about:blank", if browser == "firefox" { "firefox" } else { "chrome" })
    } else {
        format!("{} 'about:blank' 2>/dev/null", if browser == "firefox" { "firefox" } else { "google-chrome" })
    }
}

pub(crate) fn platform_open_url(url: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("open '{}'", url)
    } else if cfg!(target_os = "windows") {
        format!("start '{}'", url)
    } else {
        format!("xdg-open '{}' 2>/dev/null", url)
    }
}

pub(crate) fn platform_open_app(name: &str) -> String {
    // Resolve common aliases to their real app names
    let resolved = resolve_app_alias(name);

    if cfg!(target_os = "macos") {
        // macOS: `open -a "Name"` works for ANY installed .app
        // For CLI tools (code, docker), try the binary first
        if is_cli_tool(&resolved) {
            format!("{} 2>/dev/null || open -a '{}' 2>/dev/null", resolved, title_case(&resolved))
        } else {
            format!("open -a '{}' 2>/dev/null || open -a '{}' 2>/dev/null", title_case(&resolved), resolved)
        }
    } else if cfg!(target_os = "windows") {
        // Windows: `start` for known apps, or search Program Files
        format!("start \"\" \"{}\" 2>nul || where {} 2>nul && {} || echo App not found: {}", resolved, resolved, resolved, resolved)
    } else {
        // Linux: try lowercase binary name, then flatpak, then snap
        let bin = resolved.to_lowercase().replace(' ', "-");
        format!(
            "{bin} 2>/dev/null || flatpak run $(flatpak list --app | grep -i '{name}' | head -1 | awk '{{print $2}}') 2>/dev/null || snap run {bin} 2>/dev/null || echo 'App not found: {name}'",
            bin = bin,
            name = resolved,
        )
    }
}

pub(crate) fn platform_close_app(name: &str) -> String {
    let resolved = resolve_app_alias(name);
    if cfg!(target_os = "macos") {
        format!("osascript -e 'tell application \"{}\" to quit'", title_case(&resolved))
    } else if cfg!(target_os = "windows") {
        format!("taskkill /IM \"{}.exe\" /F 2>nul", resolved)
    } else {
        format!("pkill -f '{}' 2>/dev/null || killall '{}' 2>/dev/null", resolved, resolved)
    }
}

pub(crate) fn platform_minimize_app(name: &str) -> String {
    let resolved = resolve_app_alias(name);
    if cfg!(target_os = "macos") {
        format!("osascript -e 'tell application \"System Events\" to set visible of process \"{}\" to false'", title_case(&resolved))
    } else {
        format!("xdotool search --name '{}' windowminimize 2>/dev/null", resolved)
    }
}

pub(crate) fn platform_scroll(direction: &str, amount: &str) -> String {
    if cfg!(target_os = "macos") {
        let pixels = if amount == "max" { "9999" } else { "400" };
        let sign = if direction == "up" { "" } else { "-" };
        format!("osascript -e 'tell application \"System Events\" to scroll area 1 of (first process whose frontmost is true) by {{0, {}{}}}'", sign, pixels)
    } else {
        let button = if direction == "up" { "4" } else { "5" };
        let clicks = if amount == "max" { "50" } else { "5" };
        format!("xdotool click --repeat {} {} 2>/dev/null", clicks, button)
    }
}

pub(crate) fn platform_type_text(content: &str) -> String {
    let escaped = content.replace('\'', "'\\''");
    if cfg!(target_os = "macos") {
        format!("osascript -e 'tell application \"System Events\" to keystroke \"{}\"'", escaped)
    } else {
        format!("xdotool type '{}' 2>/dev/null", escaped)
    }
}

pub(crate) fn platform_screenshot() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let path = format!("{}/Desktop/screenshot_{}.png", home, timestamp);
    if cfg!(target_os = "macos") {
        format!("screencapture -x '{}'", path)
    } else if cfg!(target_os = "windows") {
        "snippingtool /clip".into()
    } else {
        format!("gnome-screenshot -f '{}' 2>/dev/null || scrot '{}' 2>/dev/null", path, path)
    }
}

pub(crate) fn platform_system_info() -> String {
    if cfg!(target_os = "macos") {
        "echo '=== System ===' && sw_vers && echo && echo '=== Hardware ===' && sysctl -n machdep.cpu.brand_string && echo && echo '=== Memory ===' && sysctl -n hw.memsize | awk '{printf \"%.0f GB\\n\", $1/1073741824}' && echo && echo '=== Disk ===' && df -h / | tail -1".into()
    } else if cfg!(target_os = "windows") {
        "systeminfo".into()
    } else {
        "echo '=== System ===' && uname -a && echo && cat /etc/os-release 2>/dev/null && echo && echo '=== CPU ===' && lscpu | head -5 && echo && echo '=== Memory ===' && free -h | head -2 && echo && echo '=== Disk ===' && df -h / | tail -1".into()
    }
}

/// Resolve common app aliases to their real names
pub(crate) fn resolve_app_alias(name: &str) -> String {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "chrome" | "google chrome" => "Google Chrome".into(),
        "vscode" | "vs code" | "code" => "Visual Studio Code".into(),
        "iterm" | "iterm2" => "iTerm".into(),
        "postman" => "Postman".into(),
        "browser" => "Google Chrome".into(),
        "mail" | "email" => if cfg!(target_os = "macos") { "Mail".into() } else { "thunderbird".into() },
        "files" | "file manager" => if cfg!(target_os = "macos") { "Finder".into() } else { "nautilus".into() },
        "settings" | "preferences" | "system preferences" => {
            if cfg!(target_os = "macos") { "System Settings".into() } else { "gnome-control-center".into() }
        }
        "activity monitor" | "task manager" => {
            if cfg!(target_os = "macos") { "Activity Monitor".into() } else { "gnome-system-monitor".into() }
        }
        "word" => "Microsoft Word".into(),
        "excel" => "Microsoft Excel".into(),
        "powerpoint" | "ppt" => "Microsoft PowerPoint".into(),
        "teams" => "Microsoft Teams".into(),
        "figma" => "Figma".into(),
        "notion" => "Notion".into(),
        "obs" | "obs studio" => "OBS".into(),
        "whatsapp" => "WhatsApp".into(),
        _ => name.to_string(),
    }
}

/// Check if this is a CLI tool rather than a GUI app
pub(crate) fn is_cli_tool(name: &str) -> bool {
    let cli_tools = ["code", "docker", "npm", "node", "python", "pip", "cargo", "git",
                     "brew", "htop", "vim", "nvim", "tmux", "kubectl", "terraform"];
    cli_tools.iter().any(|t| name.to_lowercase() == *t)
}

/// Convert "google chrome" → "Google Chrome"
pub(crate) fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strip articles: "the calculator" → "calculator", "a terminal" → "terminal"
pub(crate) fn strip_articles(s: &str) -> String {
    let lower = s.to_lowercase();
    for prefix in &["the ", "a ", "an ", "my ", "that "] {
        if lower.starts_with(prefix) {
            return s[prefix.len()..].to_string();
        }
    }
    s.to_string()
}

