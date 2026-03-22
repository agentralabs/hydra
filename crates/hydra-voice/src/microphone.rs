//! Microphone capture — cross-platform audio input via cpal.
//!
//! macOS: CoreAudio (zero install)
//! Linux: ALSA (needs libasound2-dev)
//!
//! Captures audio on a background thread, sends chunks via channel.
//! Never blocks the TUI. Auto-detects default input device.
//! Resamples to 16kHz mono f32 for whisper compatibility.

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Events sent from the audio capture thread to the consumer.
#[derive(Debug, Clone)]
pub enum MicEvent {
    /// A chunk of f32 mono 16kHz samples ready for processing.
    Samples(Vec<f32>),
    /// Speech started (RMS exceeded threshold).
    SpeechStarted,
    /// Silence detected — end of utterance.
    SilenceDetected,
    /// Error in audio capture.
    Error(String),
}

/// Microphone capture handle. Drop to stop recording.
pub struct MicCapture {
    /// Signal to stop the capture thread.
    stop: Arc<AtomicBool>,
    /// The capture thread handle.
    thread: Option<thread::JoinHandle<()>>,
}

impl MicCapture {
    /// Start capturing from the default microphone.
    /// Returns a receiver for MicEvents and a handle to stop capture.
    pub fn start(
        silence_threshold: f32,
        silence_chunks_for_end: usize,
    ) -> Result<(std::sync::mpsc::Receiver<MicEvent>, Self), String> {
        let (tx, rx) = std::sync::mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        let thread = thread::spawn(move || {
            if let Err(e) = run_capture(tx.clone(), stop_clone, silence_threshold, silence_chunks_for_end) {
                let _ = tx.send(MicEvent::Error(e));
            }
        });

        Ok((rx, Self {
            stop,
            thread: Some(thread),
        }))
    }

    /// Stop capture and wait for thread to finish.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for MicCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Get the name of the default input device.
pub fn default_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_input_device()
        .and_then(|d| d.name().ok())
}

/// Check if any microphone is available.
pub fn is_microphone_available() -> bool {
    let host = cpal::default_host();
    host.default_input_device().is_some()
}

/// List all available input devices.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Internal: run the capture loop on the audio thread.
fn run_capture(
    tx: std::sync::mpsc::Sender<MicEvent>,
    stop: Arc<AtomicBool>,
    silence_threshold: f32,
    silence_chunks_for_end: usize,
) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No microphone found. Connect a microphone and try again.")?;

    let config = device
        .default_input_config()
        .map_err(|e| format!("Failed to get audio config: {e}"))?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    // ~200ms chunks (matches Pulse spec)
    let chunk_samples = (sample_rate as usize * channels as usize) / 5;
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(chunk_samples * 2)));
    let buffer_clone = Arc::clone(&buffer);

    let stream_config = cpal::StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let tx_err = tx.clone();
    let stream = device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if let Ok(mut buf) = buffer_clone.lock() {
                    buf.extend_from_slice(data);
                }
            },
            move |err| {
                let _ = tx_err.send(MicEvent::Error(format!("Audio stream error: {err}")));
            },
            None,
        )
        .map_err(|e| format!("Failed to build audio stream: {e}"))?;

    stream.play().map_err(|e| format!("Failed to start audio: {e}"))?;

    eprintln!(
        "hydra: microphone active: {} ({}Hz, {} channels)",
        device.name().unwrap_or_else(|_| "unknown".into()),
        sample_rate,
        channels,
    );

    // VAD state
    let mut silent_count: usize = 0;
    let mut in_speech = false;

    // Main capture loop — runs until stop signal
    while !stop.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(50));

        let raw = {
            let mut buf = buffer.lock().unwrap();
            if buf.len() < chunk_samples {
                continue;
            }
            buf.drain(..).collect::<Vec<f32>>()
        };

        // Convert to mono 16kHz
        let mono = to_mono(&raw, channels);
        let resampled = resample(&mono, sample_rate, 16000);

        // Voice activity detection
        let energy = rms(&resampled);
        let is_loud = energy > silence_threshold;

        if is_loud {
            silent_count = 0;
            if !in_speech {
                in_speech = true;
                let _ = tx.send(MicEvent::SpeechStarted);
            }
            let _ = tx.send(MicEvent::Samples(resampled));
        } else if in_speech {
            silent_count += 1;
            if silent_count >= silence_chunks_for_end {
                in_speech = false;
                silent_count = 0;
                let _ = tx.send(MicEvent::SilenceDetected);
            } else {
                // Include trailing silence in the speech buffer
                let _ = tx.send(MicEvent::Samples(resampled));
            }
        }
    }

    drop(stream);
    Ok(())
}

/// Convert multi-channel audio to mono by averaging channels.
fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    samples
        .chunks_exact(channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Linear resampling from source rate to target rate.
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    (0..output_len)
        .map(|i| {
            let src_idx = i as f64 * ratio;
            let idx = src_idx as usize;
            let frac = (src_idx - idx as f64) as f32;
            let a = samples.get(idx).copied().unwrap_or(0.0);
            let b = samples.get(idx + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}

/// Calculate RMS (Root Mean Square) energy of audio samples.
fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_mono_stereo() {
        let stereo = vec![0.5, 0.3, 0.8, 0.2, -0.1, 0.1];
        let mono = to_mono(&stereo, 2);
        assert_eq!(mono.len(), 3);
        assert!((mono[0] - 0.4).abs() < 0.001);
    }

    #[test]
    fn resample_same_rate() {
        let input = vec![1.0, 2.0, 3.0];
        let output = resample(&input, 16000, 16000);
        assert_eq!(output, input);
    }

    #[test]
    fn resample_downsample() {
        let input: Vec<f32> = (0..48000).map(|i| (i as f32) / 48000.0).collect();
        let output = resample(&input, 48000, 16000);
        // Should be roughly 1/3 the length
        assert!((output.len() as f32 - 16000.0).abs() < 2.0);
    }

    #[test]
    fn rms_of_silence() {
        let silence = vec![0.0; 100];
        assert_eq!(rms(&silence), 0.0);
    }

    #[test]
    fn rms_of_signal() {
        let signal = vec![0.5; 100];
        assert!((rms(&signal) - 0.5).abs() < 0.001);
    }

    #[test]
    fn mic_availability_check() {
        // Just verifies the function doesn't panic
        let available = is_microphone_available();
        eprintln!("Microphone available: {available}");
        if available {
            eprintln!("Default: {:?}", default_device_name());
        }
    }
}
