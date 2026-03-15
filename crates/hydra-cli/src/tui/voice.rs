//! TUI Voice Input — microphone capture + Whisper STT for terminal mode.
//! Uses cpal for cross-platform audio capture and OpenAI Whisper for transcription.
//! Enabled via the "voice" feature flag.

use super::app::{App, Message, MessageRole};

#[cfg(feature = "voice")]
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

impl App {
    /// Toggle voice listening mode. When activated:
    /// 1. Records from microphone until silence detected
    /// 2. Sends audio to Whisper API for transcription
    /// 3. Inserts transcribed text as user input
    pub(crate) fn slash_cmd_voice(&mut self, timestamp: &str) {
        #[cfg(not(feature = "voice"))]
        {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Voice input requires the 'voice' feature. Rebuild with: cargo build --bin hydra-cli --features voice -j 1".into(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        #[cfg(feature = "voice")]
        {
            let openai_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
            if openai_key.is_empty() {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Voice input requires OPENAI_API_KEY (for Whisper STT). Set it in your environment.".into(),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
                return;
            }

            self.messages.push(Message {
                role: MessageRole::System,
                content: "Listening... speak now (auto-stops after 2s silence)".into(),
                timestamp: timestamp.to_string(),
                phase: None,
            });

            // Spawn voice capture in background thread (blocking audio I/O)
            let stop = Arc::new(AtomicBool::new(false));
            let stop_clone = stop.clone();

            let result = std::thread::spawn(move || {
                record_and_transcribe(stop_clone, &openai_key)
            }).join();

            match result {
                Ok(Ok(transcript)) if !transcript.is_empty() => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Heard: \"{}\"", transcript),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                    // Set as input for the user to review/send
                    self.input = transcript;
                    self.cursor_pos = self.input.len();
                }
                Ok(Ok(_)) => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No speech detected. Try again.".into(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                Ok(Err(e)) => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Voice error: {}", e),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                Err(_) => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "Voice capture thread panicked.".into(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
        }
    }
}

/// Record from microphone and transcribe via Whisper.
#[cfg(feature = "voice")]
fn record_and_transcribe(stop: Arc<AtomicBool>, api_key: &str) -> Result<String, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::Mutex;

    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No microphone found")?;
    let supported = device.default_input_config()
        .map_err(|e| format!("Mic config error: {}", e))?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let config: cpal::StreamConfig = supported.into();

    // Build input stream
    let buf = samples.clone();
    let ch = channels.max(1);
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if let Ok(mut b) = buf.lock() {
                for chunk in data.chunks(ch) { b.push(chunk[0]); }
            }
        },
        |e| eprintln!("[hydra:mic] stream error: {}", e),
        None,
    ).map_err(|e| format!("Stream error: {}", e))?;

    stream.play().map_err(|e| format!("Play error: {}", e))?;
    eprintln!("[hydra:mic] Recording...");

    // Record with silence detection (2s silence = stop)
    let silence_threshold = 0.01_f32;
    let mut speech_detected = false;
    let mut silence_start: Option<std::time::Instant> = None;
    let min_samples = (0.5 * sample_rate as f32) as usize;

    for _ in 0..200 { // Max 20 seconds (200 × 100ms)
        std::thread::sleep(std::time::Duration::from_millis(100));
        if stop.load(Ordering::Relaxed) { break; }

        let (total, rms) = {
            let s = samples.lock().map_err(|_| "Lock error")?;
            let len = s.len();
            let recent = &s[len.saturating_sub(4800)..]; // Last 100ms at 48kHz
            let rms = if recent.is_empty() { 0.0 } else {
                (recent.iter().map(|s| s * s).sum::<f32>() / recent.len() as f32).sqrt()
            };
            (len, rms)
        };

        if rms > silence_threshold {
            if !speech_detected { eprintln!("[hydra:mic] Speech detected"); }
            speech_detected = true;
            silence_start = None;
        } else if speech_detected && total > min_samples {
            let start = silence_start.get_or_insert_with(std::time::Instant::now);
            if start.elapsed().as_secs_f32() >= 2.0 {
                eprintln!("[hydra:mic] Silence detected — stopping");
                break;
            }
        }
    }
    drop(stream);

    let audio = samples.lock().map_err(|_| "Lock error")?.clone();
    if audio.len() < min_samples {
        return Ok(String::new()); // Too short
    }
    eprintln!("[hydra:mic] Recorded {:.1}s", audio.len() as f32 / sample_rate as f32);

    // Encode as WAV
    let wav = encode_wav(&audio, sample_rate);

    // Transcribe via Whisper (blocking)
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all().build().map_err(|e| format!("Runtime error: {}", e))?;
    rt.block_on(transcribe_whisper(wav, api_key))
}

#[cfg(feature = "voice")]
fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let mut buf = Vec::with_capacity(44 + data_size as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size).to_le_bytes());
    buf.extend_from_slice(b"WAVEfmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    buf
}

#[cfg(feature = "voice")]
async fn transcribe_whisper(wav_data: Vec<u8>, api_key: &str) -> Result<String, String> {
    let part = reqwest::multipart::Part::bytes(wav_data)
        .file_name("audio.wav").mime_str("audio/wav").map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1").part("file", part);
    let resp = reqwest::Client::new()
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key).multipart(form).send().await
        .map_err(|e| format!("Network error: {}", e))?;
    if !resp.status().is_success() {
        let s = resp.status();
        let b = resp.text().await.unwrap_or_default();
        return Err(format!("Whisper API error {}: {}", s, b));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json["text"].as_str().unwrap_or("").trim().to_string())
}
