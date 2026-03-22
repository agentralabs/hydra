//! Transcription — converts audio samples to text.
//!
//! Three-tier approach:
//! 1. whisper-cpp CLI binary (pre-built, downloaded by /voice setup)
//! 2. macOS built-in dictation (via osascript, zero install)
//! 3. Fallback: return placeholder with instructions
//!
//! Audio is written to a temp WAV file, whisper processes it, output parsed.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

/// Transcribe audio samples (16kHz mono f32) to text.
/// Tries whisper-cpp first, falls back to platform tools.
pub fn transcribe(samples: &[f32]) -> Result<String, String> {
    if samples.is_empty() {
        return Err("No audio to transcribe".into());
    }

    let wav_path = write_wav_temp(samples)?;

    // Tier 1: whisper-cpp binary
    let whisper_bin = whisper_binary_path();
    let model = crate::setup::whisper_model_path();
    if whisper_bin.exists() && model.exists() {
        return transcribe_with_whisper(&whisper_bin, &model, &wav_path);
    }

    // Tier 2: macOS built-in speech recognition
    #[cfg(target_os = "macos")]
    {
        if let Ok(text) = transcribe_with_macos(&wav_path) {
            return Ok(text);
        }
    }

    // Tier 3: Fallback
    let duration = samples.len() as f32 / 16000.0;
    Err(format!(
        "No STT engine available ({:.1}s recorded). Run /voice setup to download whisper.",
        duration
    ))
}

/// Path to the whisper-cpp binary.
fn whisper_binary_path() -> PathBuf {
    crate::setup::models_dir().join("whisper-cli")
}

/// Transcribe using whisper-cpp CLI.
fn transcribe_with_whisper(
    bin: &PathBuf,
    model: &PathBuf,
    wav_path: &std::path::Path,
) -> Result<String, String> {
    let output = Command::new(bin)
        .arg("-m")
        .arg(model)
        .arg("-f")
        .arg(wav_path)
        .arg("--no-timestamps")
        .arg("-l")
        .arg("en")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("whisper failed: {e}"))?;

    if !output.status.success() {
        return Err("whisper returned non-zero exit code".into());
    }

    let text = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
        .trim()
        .to_string();

    if text.is_empty() {
        Err("whisper returned empty transcription".into())
    } else {
        Ok(text)
    }
}

/// Transcribe using macOS speech recognition (SFSpeechRecognizer via swift).
#[cfg(target_os = "macos")]
fn transcribe_with_macos(wav_path: &std::path::Path) -> Result<String, String> {
    // Use a small Swift script to invoke SFSpeechRecognizer
    let script = format!(
        r#"
        import Foundation
        import Speech
        let url = URL(fileURLWithPath: "{}")
        let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "en-US"))!
        let request = SFSpeechURLRecognitionRequest(url: url)
        let semaphore = DispatchSemaphore(value: 0)
        var result = ""
        recognizer.recognitionTask(with: request) {{ r, error in
            if let r = r, r.isFinal {{ result = r.bestTranscription.formattedString }}
            semaphore.signal()
        }}
        semaphore.wait()
        print(result)
        "#,
        wav_path.display()
    );

    let output = Command::new("swift")
        .arg("-e")
        .arg(&script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("macOS STT failed: {e}"))?;

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        Err("macOS STT returned empty".into())
    } else {
        Ok(text)
    }
}

/// Write f32 mono 16kHz samples to a temporary WAV file.
fn write_wav_temp(samples: &[f32]) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("hydra-voice");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("capture.wav");

    let mut file = std::fs::File::create(&path).map_err(|e| format!("create wav: {e}"))?;

    let sample_rate: u32 = 16000;
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * (channels as u32) * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_size = (samples.len() * 2) as u32; // 16-bit = 2 bytes per sample
    let file_size = 36 + data_size;

    // WAV header
    file.write_all(b"RIFF").map_err(|e| format!("write: {e}"))?;
    file.write_all(&file_size.to_le_bytes()).map_err(|e| format!("write: {e}"))?;
    file.write_all(b"WAVE").map_err(|e| format!("write: {e}"))?;
    file.write_all(b"fmt ").map_err(|e| format!("write: {e}"))?;
    file.write_all(&16u32.to_le_bytes()).map_err(|e| format!("write: {e}"))?; // subchunk size
    file.write_all(&1u16.to_le_bytes()).map_err(|e| format!("write: {e}"))?; // PCM
    file.write_all(&channels.to_le_bytes()).map_err(|e| format!("write: {e}"))?;
    file.write_all(&sample_rate.to_le_bytes()).map_err(|e| format!("write: {e}"))?;
    file.write_all(&byte_rate.to_le_bytes()).map_err(|e| format!("write: {e}"))?;
    file.write_all(&block_align.to_le_bytes()).map_err(|e| format!("write: {e}"))?;
    file.write_all(&bits_per_sample.to_le_bytes()).map_err(|e| format!("write: {e}"))?;
    file.write_all(b"data").map_err(|e| format!("write: {e}"))?;
    file.write_all(&data_size.to_le_bytes()).map_err(|e| format!("write: {e}"))?;

    // Convert f32 [-1.0, 1.0] to i16 and write
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let i16_val = (clamped * i16::MAX as f32) as i16;
        file.write_all(&i16_val.to_le_bytes()).map_err(|e| format!("write: {e}"))?;
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_wav_creates_valid_file() {
        let samples: Vec<f32> = (0..16000).map(|i| (i as f32 / 16000.0 * 440.0 * std::f32::consts::TAU).sin() * 0.5).collect();
        let path = write_wav_temp(&samples).unwrap();
        assert!(path.exists());
        let meta = std::fs::metadata(&path).unwrap();
        // 44 bytes header + 32000 bytes data (16000 samples * 2 bytes)
        assert_eq!(meta.len(), 44 + 32000);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn transcribe_empty_returns_error() {
        assert!(transcribe(&[]).is_err());
    }
}
