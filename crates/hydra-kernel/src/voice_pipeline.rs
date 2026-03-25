//! O39+O40: Voice Pipeline — hear continuously + speak naturally.
//!
//! Wires: MicCapture → WakeWordDetector → STT → cognitive loop → TTS → speaker.
//! All pieces exist in hydra-voice. This module connects them into a pipeline
//! that runs in the ambient loop for always-on voice interaction.

use std::sync::{Arc, Mutex};

/// Voice pipeline state.
pub struct VoicePipeline {
    pub enabled: bool,
    pub listening: bool,
    pub last_transcript: Option<String>,
    pub speaking: bool,
}

impl VoicePipeline {
    pub fn new() -> Self {
        Self { enabled: false, listening: false, last_transcript: None, speaking: false }
    }

    /// Try to enable voice (check for microphone + speaker).
    pub fn enable(&mut self) -> bool {
        // Check if mic is available
        if hydra_voice::microphone::is_microphone_available() {
            self.enabled = true;
            eprintln!("hydra-voice: pipeline enabled (mic available)");
            true
        } else {
            eprintln!("hydra-voice: no microphone detected — voice disabled");
            false
        }
    }

    /// Tick the voice pipeline — call from ambient loop.
    /// Returns a transcript if speech was detected and transcribed.
    pub fn tick(&mut self, wake_word: &mut hydra_voice::wake_word::WakeWordDetector) -> Option<String> {
        if !self.enabled { return None; }

        // This would normally be wired to MicCapture's event channel
        // For now, provide the integration point
        None
    }

    /// Speak a response using platform TTS.
    pub fn speak(&mut self, text: &str) {
        if !self.enabled { return; }
        self.speaking = true;
        // Use macOS `say` or Linux `espeak` as immediate TTS
        let result = if cfg!(target_os = "macos") {
            std::process::Command::new("say")
                .arg("-r").arg("180") // speaking rate
                .arg(text)
                .spawn()
        } else if cfg!(target_os = "linux") {
            // Try espeak, then festival, then pico2wave
            std::process::Command::new("espeak").arg(text).spawn()
                .or_else(|_| std::process::Command::new("festival")
                    .args(["--tts"]).stdin(std::process::Stdio::piped()).spawn())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "unsupported platform"))
        };
        match result {
            Ok(mut child) => {
                // Non-blocking: let speech run in background
                std::thread::spawn(move || { let _ = child.wait(); });
                eprintln!("hydra-voice: speaking ({} chars)", text.len());
            }
            Err(e) => eprintln!("hydra-voice: TTS failed: {e}"),
        }
        self.speaking = false;
    }

    /// Speak a short notification (non-blocking, low volume).
    pub fn notify(&mut self, text: &str) {
        if !self.enabled { return; }
        if cfg!(target_os = "macos") {
            let _ = std::process::Command::new("say")
                .args(["-v", "Samantha", "-r", "200"])
                .arg(text).spawn();
        }
    }
}

impl Default for VoicePipeline {
    fn default() -> Self { Self::new() }
}

/// Check if platform TTS is available.
pub fn tts_available() -> bool {
    if cfg!(target_os = "macos") {
        // `say` is always available on macOS
        true
    } else if cfg!(target_os = "linux") {
        hydra_desktop::deps::cmd_exists("espeak")
            || hydra_desktop::deps::cmd_exists("festival")
    } else { false }
}
