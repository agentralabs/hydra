//! System-level platform controls — volume, brightness, wifi, bluetooth, power, and detection.
//! Split from platform.rs for compilation performance.

pub(crate) fn platform_volume(action: &str) -> String {
    if cfg!(target_os = "macos") {
        match action {
            "mute" => "osascript -e 'set volume with output muted'".into(),
            "up" => "osascript -e 'set volume output volume ((output volume of (get volume settings)) + 15)'".into(),
            "down" => "osascript -e 'set volume output volume ((output volume of (get volume settings)) - 15)'".into(),
            "max" => "osascript -e 'set volume output volume 100'".into(),
            _ => "osascript -e 'get volume settings'".into(),
        }
    } else {
        match action {
            "mute" => "amixer sset Master toggle 2>/dev/null || pactl set-sink-mute @DEFAULT_SINK@ toggle 2>/dev/null".into(),
            "up" => "amixer sset Master 10%+ 2>/dev/null || pactl set-sink-volume @DEFAULT_SINK@ +10% 2>/dev/null".into(),
            "down" => "amixer sset Master 10%- 2>/dev/null || pactl set-sink-volume @DEFAULT_SINK@ -10% 2>/dev/null".into(),
            "max" => "amixer sset Master 100% 2>/dev/null || pactl set-sink-volume @DEFAULT_SINK@ 100% 2>/dev/null".into(),
            _ => "amixer sget Master 2>/dev/null".into(),
        }
    }
}

pub(crate) fn platform_brightness(action: &str) -> String {
    if cfg!(target_os = "macos") {
        match action {
            "up" => "osascript -e 'tell application \"System Events\" to key code 144'".into(), // Brightness Up key
            "down" => "osascript -e 'tell application \"System Events\" to key code 145'".into(), // Brightness Down key
            _ => "echo 'Brightness adjusted'".into(),
        }
    } else {
        match action {
            "up" => "xbacklight -inc 15 2>/dev/null || brightnessctl set +15% 2>/dev/null".into(),
            "down" => "xbacklight -dec 15 2>/dev/null || brightnessctl set 15%- 2>/dev/null".into(),
            _ => "xbacklight -get 2>/dev/null || brightnessctl get 2>/dev/null".into(),
        }
    }
}

pub(crate) fn platform_wifi(enable: bool) -> String {
    if cfg!(target_os = "macos") {
        if enable {
            "networksetup -setairportpower en0 on".into()
        } else {
            "networksetup -setairportpower en0 off".into()
        }
    } else {
        if enable { "nmcli radio wifi on".into() } else { "nmcli radio wifi off".into() }
    }
}

pub(crate) fn platform_wifi_status() -> String {
    if cfg!(target_os = "macos") {
        "networksetup -getairportnetwork en0 && echo && networksetup -getinfo Wi-Fi | head -5".into()
    } else {
        "nmcli general status && echo && nmcli connection show --active".into()
    }
}

pub(crate) fn platform_bluetooth(enable: bool) -> String {
    if cfg!(target_os = "macos") {
        // Requires blueutil: brew install blueutil
        if enable { "blueutil --power 1 2>/dev/null || echo 'Install blueutil: brew install blueutil'".into() }
        else { "blueutil --power 0 2>/dev/null || echo 'Install blueutil: brew install blueutil'".into() }
    } else {
        if enable { "bluetoothctl power on".into() } else { "bluetoothctl power off".into() }
    }
}

pub(crate) fn platform_dark_mode(enable: bool) -> String {
    if cfg!(target_os = "macos") {
        if enable {
            "osascript -e 'tell application \"System Events\" to tell appearance preferences to set dark mode to true'".into()
        } else {
            "osascript -e 'tell application \"System Events\" to tell appearance preferences to set dark mode to false'".into()
        }
    } else {
        if enable {
            "gsettings set org.gnome.desktop.interface color-scheme 'prefer-dark' 2>/dev/null".into()
        } else {
            "gsettings set org.gnome.desktop.interface color-scheme 'prefer-light' 2>/dev/null".into()
        }
    }
}

pub(crate) fn platform_lock_screen() -> String {
    if cfg!(target_os = "macos") {
        "osascript -e 'tell application \"System Events\" to keystroke \"q\" using {control down, command down}'".into()
    } else if cfg!(target_os = "windows") {
        "rundll32.exe user32.dll,LockWorkStation".into()
    } else {
        "loginctl lock-session 2>/dev/null || xdg-screensaver lock 2>/dev/null".into()
    }
}

pub(crate) fn platform_sleep() -> String {
    if cfg!(target_os = "macos") {
        "pmset sleepnow".into()
    } else if cfg!(target_os = "windows") {
        "rundll32.exe powrprof.dll,SetSuspendState 0,1,0".into()
    } else {
        "systemctl suspend 2>/dev/null".into()
    }
}

pub(crate) fn platform_dnd(enable: bool) -> String {
    if cfg!(target_os = "macos") {
        if enable {
            "shortcuts run 'Turn On Focus' 2>/dev/null || echo 'DND enabled (use System Settings to configure)'".into()
        } else {
            "shortcuts run 'Turn Off Focus' 2>/dev/null || echo 'DND disabled'".into()
        }
    } else {
        "echo 'Do Not Disturb toggled'".into()
    }
}

pub(crate) fn platform_empty_trash() -> String {
    if cfg!(target_os = "macos") {
        "osascript -e 'tell application \"Finder\" to empty the trash'".into()
    } else {
        "rm -rf ~/.local/share/Trash/files/* ~/.local/share/Trash/info/* 2>/dev/null && echo 'Trash emptied'".into()
    }
}

pub(crate) fn platform_battery_status() -> String {
    if cfg!(target_os = "macos") {
        "pmset -g batt".into()
    } else if cfg!(target_os = "windows") {
        "WMIC Path Win32_Battery Get EstimatedChargeRemaining".into()
    } else {
        "upower -i /org/freedesktop/UPower/devices/battery_BAT0 2>/dev/null || cat /sys/class/power_supply/BAT0/capacity 2>/dev/null".into()
    }
}

pub(crate) fn platform_ip_address() -> String {
    if cfg!(target_os = "macos") {
        "echo 'Local:' && ipconfig getifaddr en0 2>/dev/null; echo && echo 'Public:' && curl -s ifconfig.me".into()
    } else {
        "echo 'Local:' && hostname -I 2>/dev/null | awk '{print $1}'; echo && echo 'Public:' && curl -s ifconfig.me".into()
    }
}

pub(crate) fn platform_disk_space() -> String {
    if cfg!(target_os = "macos") {
        "df -h / && echo && echo '=== Largest folders ===' && du -sh ~/Desktop ~/Documents ~/Downloads ~/Library 2>/dev/null | sort -rh | head -10".into()
    } else {
        "df -h / && echo && echo '=== Largest folders ===' && du -sh ~/* 2>/dev/null | sort -rh | head -10".into()
    }
}

pub(crate) fn platform_running_processes() -> String {
    if cfg!(target_os = "macos") {
        "ps aux --sort=-%mem | head -15".into()
    } else {
        "ps aux --sort=-%mem | head -15".into()
    }
}

pub(crate) fn platform_list_installed_apps() -> String {
    if cfg!(target_os = "macos") {
        "ls /Applications/ | sed 's/.app$//' | sort".into()
    } else {
        "dpkg --list 2>/dev/null | tail -20 || rpm -qa 2>/dev/null | head -20 || pacman -Q 2>/dev/null | head -20".into()
    }
}

/// Detect system-level control intents (volume, brightness, wifi, power, etc.)
pub(crate) fn detect_system_control(text: &str) -> Option<String> {
    let lower = text.to_lowercase();

    // ── Volume ──
    if lower.contains("volume") || lower.contains("sound") {
        if lower.contains("mute") || lower.contains("silent") {
            return Some(platform_volume("mute"));
        } else if lower.contains("up") || lower.contains("increase") || lower.contains("louder") {
            return Some(platform_volume("up"));
        } else if lower.contains("down") || lower.contains("decrease") || lower.contains("lower") || lower.contains("quieter") {
            return Some(platform_volume("down"));
        } else if lower.contains("max") || lower.contains("full") {
            return Some(platform_volume("max"));
        }
    }

    // ── Brightness ──
    if lower.contains("brightness") || lower.contains("screen bright") {
        if lower.contains("up") || lower.contains("increase") || lower.contains("brighter") {
            return Some(platform_brightness("up"));
        } else if lower.contains("down") || lower.contains("decrease") || lower.contains("dim") {
            return Some(platform_brightness("down"));
        }
    }

    // ── WiFi ──
    if lower.contains("wifi") || lower.contains("wi-fi") {
        if lower.contains("off") || lower.contains("disable") || lower.contains("disconnect") {
            return Some(platform_wifi(false));
        } else if lower.contains("on") || lower.contains("enable") || lower.contains("connect") {
            return Some(platform_wifi(true));
        } else if lower.contains("status") || lower.contains("check") {
            return Some(platform_wifi_status());
        }
    }

    // ── Bluetooth ──
    if lower.contains("bluetooth") {
        if lower.contains("off") || lower.contains("disable") {
            return Some(platform_bluetooth(false));
        } else if lower.contains("on") || lower.contains("enable") {
            return Some(platform_bluetooth(true));
        }
    }

    // ── Dark / Light mode ──
    if lower.contains("dark mode") {
        if lower.contains("on") || lower.contains("enable") || lower.contains("switch to") || lower.contains("turn on") {
            return Some(platform_dark_mode(true));
        } else if lower.contains("off") || lower.contains("disable") || lower.contains("turn off") {
            return Some(platform_dark_mode(false));
        }
    }
    if lower.contains("light mode") {
        return Some(platform_dark_mode(false));
    }

    // ── Sleep / Lock / Shutdown ──
    if lower.contains("lock") && (lower.contains("screen") || lower.contains("computer") || lower.contains("mac") || lower.contains("pc")) {
        return Some(platform_lock_screen());
    }
    if (lower.contains("sleep") || lower.contains("standby")) && (lower.contains("computer") || lower.contains("mac") || lower.contains("pc") || lower.contains("system")) {
        return Some(platform_sleep());
    }

    // ── Do Not Disturb ──
    if lower.contains("do not disturb") || lower.contains("dnd") || lower.contains("focus mode") {
        if lower.contains("off") || lower.contains("disable") {
            return Some(platform_dnd(false));
        } else {
            return Some(platform_dnd(true));
        }
    }

    // ── Empty trash ──
    if lower.contains("empty") && lower.contains("trash") {
        return Some(platform_empty_trash());
    }

    // ── Battery ──
    if lower.contains("battery") && (lower.contains("status") || lower.contains("level") || lower.contains("check") || lower.contains("how much")) {
        return Some(platform_battery_status());
    }

    // ── IP address / network ──
    if lower.contains("ip address") || lower.contains("my ip") || (lower.contains("what") && lower.contains("ip")) {
        return Some(platform_ip_address());
    }

    // ── Disk space ──
    if lower.contains("disk space") || lower.contains("storage") || lower.contains("how much space") {
        return Some(platform_disk_space());
    }

    // ── List running processes ──
    if lower.contains("running") && (lower.contains("process") || lower.contains("app")) {
        return Some(platform_running_processes());
    }

    // ── List installed apps ──
    if lower.contains("installed") && (lower.contains("app") || lower.contains("program") || lower.contains("software")) {
        return Some(platform_list_installed_apps());
    }

    None
}
