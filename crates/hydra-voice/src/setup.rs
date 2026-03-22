//! Voice setup — dependency checking and model management.
//!
//! Checks what voice capabilities are available.
//! Downloads whisper model if needed (for STT).
//! Detects native TTS engine.
//! Reports status to TUI.

use std::path::PathBuf;

use crate::native_tts::TtsEngine;

/// Full voice capability report.
#[derive(Debug, Clone)]
pub struct VoiceCapabilities {
    /// TTS engine detected.
    pub tts_engine: TtsEngine,
    /// Whether whisper model exists for STT.
    pub stt_available: bool,
    /// Path to whisper model (if available).
    pub whisper_model_path: Option<PathBuf>,
    /// Status messages for display.
    pub status_lines: Vec<String>,
}

impl VoiceCapabilities {
    /// Check all voice capabilities on this system.
    pub fn detect() -> Self {
        let tts_engine = TtsEngine::detect();
        let model_path = whisper_model_path();
        let stt_available = model_path.exists();

        let mut status_lines = Vec::new();

        // TTS status
        if tts_engine.is_available() {
            status_lines.push(format!("TTS: {:?} (ready)", tts_engine));
        } else {
            status_lines.push(format!("TTS: not available — {}", tts_engine.install_hint()));
        }

        // STT status
        if stt_available {
            status_lines.push(format!(
                "STT: whisper model loaded ({})",
                model_path.display()
            ));
        } else {
            status_lines.push(
                "STT: whisper model not found. Run /voice setup to download.".into(),
            );
        }

        Self {
            tts_engine,
            stt_available,
            whisper_model_path: if stt_available {
                Some(model_path)
            } else {
                None
            },
            status_lines,
        }
    }

    /// Whether any voice capability is available.
    pub fn any_available(&self) -> bool {
        self.tts_engine.is_available() || self.stt_available
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        let tts = if self.tts_engine.is_available() {
            "ready"
        } else {
            "unavailable"
        };
        let stt = if self.stt_available {
            "ready"
        } else {
            "needs setup"
        };
        format!("Voice: TTS={tts}, STT={stt}")
    }
}

/// Path to the whisper model file.
/// Located at ~/.hydra/models/ggml-base.en.bin
pub fn whisper_model_path() -> PathBuf {
    models_dir().join("ggml-base.en.bin")
}

/// Path to the models directory.
pub fn models_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hydra")
        .join("models")
}

/// Download the whisper base.en model if not present.
/// Returns Ok(path) on success, Err(message) on failure.
/// This is a blocking operation (~142MB download).
pub fn download_whisper_model() -> Result<PathBuf, String> {
    let path = whisper_model_path();
    if path.exists() {
        return Ok(path);
    }

    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir failed: {e}"))?;

    let url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";

    eprintln!("hydra: downloading whisper model (~142MB)...");
    eprintln!("hydra: from {url}");

    // Use curl or wget — available on all platforms
    let result = if command_exists("curl") {
        std::process::Command::new("curl")
            .arg("-L")
            .arg("-o")
            .arg(path.to_str().unwrap_or("model.bin"))
            .arg("--progress-bar")
            .arg(url)
            .status()
    } else if command_exists("wget") {
        std::process::Command::new("wget")
            .arg("-O")
            .arg(path.to_str().unwrap_or("model.bin"))
            .arg("--show-progress")
            .arg(url)
            .status()
    } else {
        return Err(
            "Neither curl nor wget found. Install one and retry, or download manually from:\n\
             https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin\n\
             Save to: ~/.hydra/models/ggml-base.en.bin"
                .into(),
        );
    };

    match result {
        Ok(status) if status.success() => {
            if path.exists() {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                eprintln!(
                    "hydra: whisper model downloaded ({:.1}MB)",
                    size as f64 / 1_000_000.0
                );
                Ok(path)
            } else {
                Err("Download appeared to succeed but file not found".into())
            }
        }
        Ok(status) => Err(format!("Download failed with exit code: {}", status)),
        Err(e) => Err(format!("Download command failed: {e}")),
    }
}

/// Download the whisper-cpp CLI binary for the current platform.
pub fn download_whisper_binary() -> Result<PathBuf, String> {
    let bin_path = models_dir().join("whisper-cli");
    if bin_path.exists() {
        return Ok(bin_path);
    }

    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;

    // Determine platform binary URL
    let (url, archive_name) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => (
            "https://github.com/ggerganov/whisper.cpp/releases/latest/download/whisper-cli-bin-darwin-arm64.zip",
            "whisper-cli-bin-darwin-arm64",
        ),
        ("macos", _) => (
            "https://github.com/ggerganov/whisper.cpp/releases/latest/download/whisper-cli-bin-darwin-x86_64.zip",
            "whisper-cli-bin-darwin-x86_64",
        ),
        ("linux", _) => (
            "https://github.com/ggerganov/whisper.cpp/releases/latest/download/whisper-cli-bin-linux-x86_64.zip",
            "whisper-cli-bin-linux-x86_64",
        ),
        _ => return Err("Unsupported platform for whisper binary".into()),
    };

    let zip_path = dir.join(format!("{archive_name}.zip"));

    eprintln!("hydra: downloading whisper-cli binary...");

    if command_exists("curl") {
        let status = std::process::Command::new("curl")
            .arg("-L")
            .arg("-o")
            .arg(zip_path.to_str().unwrap_or("whisper.zip"))
            .arg("--progress-bar")
            .arg(url)
            .status()
            .map_err(|e| format!("curl failed: {e}"))?;
        if !status.success() {
            return Err("Download failed".into());
        }
    } else {
        return Err("curl not found — cannot download whisper binary".into());
    }

    // Extract zip
    if command_exists("unzip") {
        let status = std::process::Command::new("unzip")
            .arg("-o")
            .arg("-j")
            .arg(&zip_path)
            .arg("-d")
            .arg(&dir)
            .stdout(std::process::Stdio::null())
            .status()
            .map_err(|e| format!("unzip failed: {e}"))?;
        if !status.success() {
            return Err("unzip failed".into());
        }
    } else {
        return Err("unzip not found — cannot extract whisper binary".into());
    }

    // Find the extracted binary (may be named whisper-cli or main)
    let candidates = ["whisper-cli", "main", "whisper"];
    for name in &candidates {
        let candidate = dir.join(name);
        if candidate.exists() {
            if candidate != bin_path {
                std::fs::rename(&candidate, &bin_path)
                    .map_err(|e| format!("rename: {e}"))?;
            }
            // Make executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    &bin_path,
                    std::fs::Permissions::from_mode(0o755),
                );
            }
            let _ = std::fs::remove_file(&zip_path);
            eprintln!("hydra: whisper-cli binary installed");
            return Ok(bin_path);
        }
    }

    Err("Binary not found in downloaded archive".into())
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
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
    fn detect_capabilities() {
        let caps = VoiceCapabilities::detect();
        eprintln!("Voice capabilities: {:?}", caps);
        eprintln!("Summary: {}", caps.summary());
    }

    #[test]
    fn model_path_is_in_hydra_dir() {
        let path = whisper_model_path();
        assert!(path.to_string_lossy().contains(".hydra"));
        assert!(path.to_string_lossy().contains("models"));
    }
}
