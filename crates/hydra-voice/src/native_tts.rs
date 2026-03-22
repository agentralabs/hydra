//! Native TTS — speaks text using platform commands.
//!
//! macOS:  `say` (built-in, zero install)
//! Linux:  `espeak-ng` or `espeak` or `festival` (common, one apt install)
//! WSL:    same as Linux
//!
//! Runs as async subprocess — never blocks the TUI.
//! Falls back gracefully if no TTS engine is available.

use std::process::Command;

/// Which TTS engine is available on this system.
#[derive(Debug, Clone, PartialEq)]
pub enum TtsEngine {
    /// macOS built-in `say` command.
    MacSay,
    /// Linux `espeak-ng` (modern).
    EspeakNg,
    /// Linux `espeak` (legacy).
    Espeak,
    /// No TTS engine found.
    None,
}

impl TtsEngine {
    /// Detect the best available TTS engine on this system.
    pub fn detect() -> Self {
        if cfg!(target_os = "macos") {
            // macOS always has `say`
            if command_exists("say") {
                return Self::MacSay;
            }
        }
        // Linux / WSL
        if command_exists("espeak-ng") {
            return Self::EspeakNg;
        }
        if command_exists("espeak") {
            return Self::Espeak;
        }
        Self::None
    }

    /// Whether any TTS engine is available.
    pub fn is_available(&self) -> bool {
        *self != Self::None
    }

    /// Get install instructions for the current platform.
    pub fn install_hint(&self) -> &'static str {
        if cfg!(target_os = "macos") {
            "macOS: `say` should be built-in. Check /usr/bin/say"
        } else {
            "Linux: sudo apt install espeak-ng  (or: sudo dnf install espeak-ng)"
        }
    }
}

/// Speak text asynchronously using the detected engine.
/// Returns immediately — speech happens in background subprocess.
/// Returns the child process handle for interruption.
pub fn speak_async(engine: &TtsEngine, text: &str) -> Option<std::process::Child> {
    let sanitized = sanitize_for_speech(text);
    if sanitized.is_empty() {
        return None;
    }

    let result = match engine {
        TtsEngine::MacSay => Command::new("say")
            .arg("-r")
            .arg("180") // slightly faster than default
            .arg(&sanitized)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn(),
        TtsEngine::EspeakNg => Command::new("espeak-ng")
            .arg("-s")
            .arg("160") // words per minute
            .arg(&sanitized)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn(),
        TtsEngine::Espeak => Command::new("espeak")
            .arg("-s")
            .arg("160")
            .arg(&sanitized)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn(),
        TtsEngine::None => return None,
    };

    match result {
        Ok(child) => Some(child),
        Err(e) => {
            eprintln!("hydra: TTS speak failed: {e}");
            None
        }
    }
}

/// Speak text and block until finished. For short alerts only.
pub fn speak_blocking(engine: &TtsEngine, text: &str) {
    if let Some(mut child) = speak_async(engine, text) {
        let _ = child.wait();
    }
}

/// Interrupt any running TTS by killing the subprocess.
pub fn interrupt(child: &mut Option<std::process::Child>) {
    if let Some(c) = child.take() {
        let mut c = c;
        let _ = c.kill();
        let _ = c.wait();
    }
}

/// Sanitize text for shell-safe speech.
/// Only allows letters, digits, whitespace, and basic punctuation.
fn sanitize_for_speech(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?'-".contains(*c))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Check if a command exists in PATH.
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_something() {
        let engine = TtsEngine::detect();
        // On CI this might be None, on macOS it should be MacSay
        eprintln!("Detected TTS engine: {:?}", engine);
    }

    #[test]
    fn sanitize_removes_dangerous_chars() {
        let input = "hello; rm -rf / && echo pwned";
        let safe = sanitize_for_speech(input);
        assert!(!safe.contains(';'));
        assert!(!safe.contains('/'));
        assert!(!safe.contains('&'));
    }

    #[test]
    fn empty_text_returns_none() {
        let engine = TtsEngine::MacSay;
        assert!(speak_async(&engine, "").is_none());
    }
}
