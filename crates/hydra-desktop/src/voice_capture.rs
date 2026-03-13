//! Cross-platform microphone capture + OpenAI Whisper STT + TTS playback.
//! Works on macOS (CoreAudio), Windows (WASAPI), Linux (ALSA/PulseAudio).

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use cpal::SampleFormat;

/// Record from the default microphone until `stop` is set to true.
/// Returns (mono f32 samples, sample_rate) or None if mic unavailable.
pub fn record_until_stopped(stop: Arc<AtomicBool>) -> Option<(Vec<f32>, u32)> {
    let host = cpal::default_host();
    let device = host.default_input_device()?;
    let dev_name = device.name().unwrap_or_else(|_| "unknown".into());
    eprintln!("[hydra:mic] Input device: {}", dev_name);

    let supported = device.default_input_config().ok()?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let format = supported.sample_format();
    eprintln!("[hydra:mic] Format: {:?}, {}Hz, {}ch", format, sample_rate, channels);

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let config: cpal::StreamConfig = supported.into();
    let stream = build_input_stream(&device, &config, format, channels, samples.clone())?;
    stream.play().ok()?;

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(stream);
    let result = samples.lock().ok()?.clone();
    eprintln!("[hydra:mic] Recorded {} samples ({:.1}s)", result.len(), result.len() as f32 / sample_rate as f32);
    Some((result, sample_rate))
}

/// Record with automatic silence detection — stops after `silence_secs` of quiet.
pub fn record_until_silence(
    stop: Arc<AtomicBool>, silence_secs: f32, min_speech_secs: f32, silence_threshold: f32,
) -> Option<(Vec<f32>, u32)> {
    let host = cpal::default_host();
    let device = host.default_input_device()?;
    let dev_name = device.name().unwrap_or_else(|_| "unknown".into());

    let supported = device.default_input_config().ok()?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let format = supported.sample_format();
    eprintln!("[hydra:mic] VAD: {:?} {}Hz {}ch ({})", format, sample_rate, channels, dev_name);

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let recent_rms: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let config: cpal::StreamConfig = supported.into();
    let stream = build_input_stream_vad(&device, &config, format, channels, samples.clone(), recent_rms.clone())?;
    stream.play().ok()?;
    eprintln!("[hydra:mic] Listening (auto-stop after {:.1}s silence)...", silence_secs);

    let check_interval = std::time::Duration::from_millis(100);
    let mut speech_detected = false;
    let mut silence_start: Option<std::time::Instant> = None;
    let min_samples = (min_speech_secs * sample_rate as f32) as usize;

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(check_interval);
        let current_rms = {
            let Ok(mut r) = recent_rms.lock() else { continue };
            if r.is_empty() { continue; }
            let avg = r.iter().sum::<f32>() / r.len() as f32;
            r.clear();
            avg
        };
        let total_samples = samples.lock().ok().map(|s| s.len()).unwrap_or(0);
        if current_rms > silence_threshold {
            if !speech_detected { eprintln!("[hydra:mic] Speech detected (rms={:.4})", current_rms); }
            speech_detected = true;
            silence_start = None;
        } else if speech_detected && total_samples > min_samples {
            let start = silence_start.get_or_insert_with(std::time::Instant::now);
            if start.elapsed().as_secs_f32() >= silence_secs {
                eprintln!("[hydra:mic] Silence detected — auto-stopping");
                break;
            }
        }
    }
    drop(stream);
    let result = samples.lock().ok()?.clone();
    eprintln!("[hydra:mic] Recorded {} samples ({:.1}s)", result.len(), result.len() as f32 / sample_rate as f32);
    Some((result, sample_rate))
}

/// Build a cpal input stream for any supported format (F32, I16, I32, U16).
fn build_input_stream(
    device: &cpal::Device, config: &cpal::StreamConfig, format: SampleFormat,
    channels: usize, buf: Arc<Mutex<Vec<f32>>>,
) -> Option<cpal::Stream> {
    let ch = channels.max(1);
    let err_fn = |e: cpal::StreamError| eprintln!("[hydra:mic] stream error: {}", e);
    let stream = match format {
        SampleFormat::F32 => device.build_input_stream(config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let Ok(mut b) = buf.lock() else { return };
                for chunk in data.chunks(ch) { b.push(chunk[0]); }
            }, err_fn, None),
        SampleFormat::I16 => device.build_input_stream(config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let Ok(mut b) = buf.lock() else { return };
                for chunk in data.chunks(ch) { b.push(chunk[0] as f32 / 32768.0); }
            }, err_fn, None),
        SampleFormat::I32 => device.build_input_stream(config,
            move |data: &[i32], _: &cpal::InputCallbackInfo| {
                let Ok(mut b) = buf.lock() else { return };
                for chunk in data.chunks(ch) { b.push(chunk[0] as f32 / 2147483648.0); }
            }, err_fn, None),
        SampleFormat::U16 => device.build_input_stream(config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                let Ok(mut b) = buf.lock() else { return };
                for chunk in data.chunks(ch) { b.push((chunk[0] as f32 - 32768.0) / 32768.0); }
            }, err_fn, None),
        _ => { eprintln!("[hydra:mic] Unsupported format: {:?}", format); return None; }
    };
    stream.ok()
}

/// Build input stream with RMS tracking for VAD.
fn build_input_stream_vad(
    device: &cpal::Device, config: &cpal::StreamConfig, format: SampleFormat,
    channels: usize, buf: Arc<Mutex<Vec<f32>>>, rms: Arc<Mutex<Vec<f32>>>,
) -> Option<cpal::Stream> {
    let ch = channels.max(1);
    let err_fn = |e: cpal::StreamError| eprintln!("[hydra:mic] error: {}", e);
    macro_rules! vad_stream {
        ($t:ty, $conv:expr) => {{
            let buf = buf.clone(); let rms = rms.clone();
            device.build_input_stream(config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    let Ok(mut b) = buf.lock() else { return };
                    let mut sum = 0.0f32; let mut count = 0usize;
                    for chunk in data.chunks(ch) {
                        let s: f32 = $conv(chunk[0]);
                        b.push(s); sum += s * s; count += 1;
                    }
                    if count > 0 { if let Ok(mut r) = rms.lock() { r.push((sum / count as f32).sqrt()); } }
                }, err_fn, None)
        }};
    }
    let stream = match format {
        SampleFormat::F32 => vad_stream!(f32, |s: f32| s),
        SampleFormat::I16 => vad_stream!(i16, |s: i16| s as f32 / 32768.0),
        SampleFormat::I32 => vad_stream!(i32, |s: i32| s as f32 / 2147483648.0),
        SampleFormat::U16 => vad_stream!(u16, |s: u16| (s as f32 - 32768.0) / 32768.0),
        _ => { eprintln!("[hydra:mic] Unsupported format: {:?}", format); return None; }
    };
    stream.ok()
}

/// Encode f32 mono samples as WAV (16-bit PCM).
pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let mut buf = Vec::with_capacity(44 + data_size as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size).to_le_bytes());
    buf.extend_from_slice(b"WAVEfmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples { buf.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()); }
    buf
}

/// Send WAV audio to OpenAI Whisper API and return the transcript.
pub async fn transcribe_whisper(wav_data: Vec<u8>, api_key: &str, language: &str) -> Result<String, String> {
    let part = reqwest::multipart::Part::bytes(wav_data)
        .file_name("audio.wav").mime_str("audio/wav").map_err(|e| e.to_string())?;
    let mut form = reqwest::multipart::Form::new().text("model", "whisper-1").part("file", part);
    if !language.is_empty() { form = form.text("language", language.to_string()); }
    let resp = reqwest::Client::new()
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key).multipart(form).send().await
        .map_err(|e| format!("Network error: {}", e))?;
    if !resp.status().is_success() {
        let s = resp.status(); let b = resp.text().await.unwrap_or_default();
        return Err(format!("Whisper API error {}: {}", s, b));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json["text"].as_str().unwrap_or("").trim().to_string())
}

/// Call OpenAI TTS API and return raw PCM samples (24kHz mono f32).
pub async fn synthesize_openai_tts(text: &str, api_key: &str, voice: &str) -> Result<Vec<f32>, String> {
    if text.is_empty() { return Ok(Vec::new()); }
    let text = if text.len() > 4000 { &text[..4000] } else { text };
    let body = serde_json::json!({ "model": "tts-1", "input": text, "voice": voice, "response_format": "pcm" });
    let resp = reqwest::Client::new()
        .post("https://api.openai.com/v1/audio/speech")
        .bearer_auth(api_key).json(&body).send().await
        .map_err(|e| format!("TTS network error: {}", e))?;
    if !resp.status().is_success() {
        let s = resp.status(); let b = resp.text().await.unwrap_or_default();
        return Err(format!("TTS API error {}: {}", s, b));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("TTS read error: {}", e))?;
    Ok(bytes.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect())
}

/// Simple linear interpolation resampler (e.g. 24kHz → 48kHz).
fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio).ceil() as usize;
    (0..out_len).map(|i| {
        let src = i as f64 * ratio;
        let idx = src as usize;
        let frac = (src - idx as f64) as f32;
        let a = samples.get(idx).copied().unwrap_or(0.0);
        let b = samples.get(idx + 1).copied().unwrap_or(a);
        a + (b - a) * frac
    }).collect()
}

/// Select the best output device cross-platform.
/// Strategy: default device first (correct on Windows/Linux), with fallback
/// heuristics to avoid display/HDMI outputs (common macOS issue).
fn select_output_device(host: &cpal::Host) -> Result<cpal::Device, String> {
    let devs: Vec<_> = host.output_devices().map(|d| d.collect()).unwrap_or_default();
    for d in &devs { eprintln!("[hydra:audio] Found output: {}", d.name().unwrap_or_default()); }

    // Check if default device looks like a display output (macOS issue)
    if let Some(default) = host.default_output_device() {
        let name = default.name().unwrap_or_default().to_lowercase();
        let is_display = name.contains("hdmi") || name.contains("displayport")
            || name.contains("benq") || name.contains("dell") || name.contains("lg ")
            || name.contains("samsung") || name.contains("acer") || name.contains("asus");
        if !is_display { return Ok(default); }
        eprintln!("[hydra:audio] Default is display output ({}), looking for speakers...", name);
    }

    // Fallback: find a device that looks like speakers
    if let Some(d) = devs.iter().position(|d| {
        let n = d.name().unwrap_or_default().to_lowercase();
        n.contains("speaker") || n.contains("headphone") || n.contains("built")
            || n.contains("realtek") || n.contains("pulse") || n.contains("pipewire")
            || n.contains("default") || n.contains("mac")
    }) {
        let mut devs = devs;
        return Ok(devs.swap_remove(d));
    }

    // Last resort: any output device, or default
    host.default_output_device().ok_or_else(|| "No audio output device found".to_string())
}

/// Find a compatible output config, supporting F32 and I16 (cross-platform).
fn find_output_config(device: &cpal::Device) -> Result<(cpal::SupportedStreamConfigRange, SampleFormat), String> {
    let configs: Vec<_> = device.supported_output_configs()
        .map_err(|e| format!("Audio config error: {}", e))?.collect();
    // Prefer mono F32, then stereo F32, then mono I16, then stereo I16
    for fmt in [SampleFormat::F32, SampleFormat::I16] {
        if let Some(c) = configs.iter().find(|c| c.channels() == 1 && c.sample_format() == fmt) {
            return Ok((c.clone(), fmt));
        }
        if let Some(c) = configs.iter().find(|c| c.sample_format() == fmt) {
            return Ok((c.clone(), fmt));
        }
    }
    Err("No compatible audio output format (need F32 or I16)".into())
}

/// Play f32 PCM samples through the best output device. Blocks until done.
/// `volume` is 0-100 (percentage). Cross-platform: F32/I16 output, auto-resample.
pub fn play_audio(samples: Vec<f32>, sample_rate: u32, volume: u8) -> Result<(), String> {
    let vol = (volume.min(100) as f32) / 100.0;
    let host = cpal::default_host();
    let device = select_output_device(&host)?;
    let dev_name = device.name().unwrap_or_else(|_| "unknown".into());
    eprintln!("[hydra:audio] Output device: {}", dev_name);

    let (supported, out_fmt) = find_output_config(&device)?;
    let min_rate = supported.min_sample_rate().0;
    let max_rate = supported.max_sample_rate().0;
    let device_rate = sample_rate.clamp(min_rate, max_rate);
    let samples = if device_rate != sample_rate {
        eprintln!("[hydra:audio] Resampling {}Hz -> {}Hz ({} samples)", sample_rate, device_rate, samples.len());
        resample_linear(&samples, sample_rate, device_rate)
    } else { samples };
    let config = supported.with_sample_rate(cpal::SampleRate(device_rate)).into();
    let cursor = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let samples = Arc::new(samples);
    let done = Arc::new(AtomicBool::new(false));
    let channels = supported.channels() as usize;

    let stream = build_output_stream(&device, &config, out_fmt, channels, samples.clone(), cursor.clone(), done.clone(), vol)?;
    stream.play().map_err(|e| format!("Audio play error: {}", e))?;
    while !done.load(Ordering::Relaxed) { std::thread::sleep(std::time::Duration::from_millis(20)); }
    std::thread::sleep(std::time::Duration::from_millis(80)); // flush buffer
    Ok(())
}

/// Build output stream for F32 or I16 format with volume control.
fn build_output_stream(
    device: &cpal::Device, config: &cpal::StreamConfig, format: SampleFormat,
    channels: usize, samples: Arc<Vec<f32>>,
    cursor: Arc<std::sync::atomic::AtomicUsize>, done: Arc<AtomicBool>, vol: f32,
) -> Result<cpal::Stream, String> {
    let err_fn = |e: cpal::StreamError| eprintln!("[hydra:audio] output error: {}", e);
    match format {
        SampleFormat::F32 => device.build_output_stream(config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut pos = cursor.load(Ordering::Relaxed);
                for frame in data.chunks_mut(channels) {
                    let s = if pos < samples.len() { samples[pos] * vol } else { 0.0 };
                    for out in frame.iter_mut() { *out = s; }
                    pos += 1;
                }
                cursor.store(pos, Ordering::Relaxed);
                if pos >= samples.len() { done.store(true, Ordering::Relaxed); }
            }, err_fn, None).map_err(|e| format!("Stream error: {}", e)),
        SampleFormat::I16 => device.build_output_stream(config,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                let mut pos = cursor.load(Ordering::Relaxed);
                for frame in data.chunks_mut(channels) {
                    let s = if pos < samples.len() { (samples[pos] * vol * 32767.0) as i16 } else { 0 };
                    for out in frame.iter_mut() { *out = s; }
                    pos += 1;
                }
                cursor.store(pos, Ordering::Relaxed);
                if pos >= samples.len() { done.store(true, Ordering::Relaxed); }
            }, err_fn, None).map_err(|e| format!("Stream error: {}", e)),
        _ => Err(format!("Unsupported output format: {:?}", format)),
    }
}
