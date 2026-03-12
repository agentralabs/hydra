//! Real microphone capture + OpenAI Whisper transcription + TTS playback.

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
    let config = device.default_input_config().ok()?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_cb = samples.clone();

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let Ok(mut buf) = samples_cb.lock() else { return };
                if channels == 1 {
                    buf.extend_from_slice(data);
                } else {
                    for chunk in data.chunks(channels) {
                        buf.push(chunk[0]);
                    }
                }
            },
            |err| eprintln!("[hydra] mic error: {}", err),
            None,
        )
        .ok()?;

    stream.play().ok()?;
    println!("[hydra] mic recording started");

    // Wait until stop flag is set
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    drop(stream);
    let result = samples.lock().ok()?.clone();
    println!("[hydra] mic recorded {} samples at {}Hz", result.len(), sample_rate);
    Some((result, sample_rate))
}

/// Encode f32 mono samples as a WAV file (16-bit PCM).
pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len();
    let data_size = (num_samples * 2) as u32;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(44 + data_size as usize);
    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &sample in samples {
        let s = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}

/// Send WAV audio to OpenAI Whisper API and return the transcript.
/// `language` is an ISO-639-1 code (e.g. "en", "es", "fr") or empty for auto-detect.
pub async fn transcribe_whisper(wav_data: Vec<u8>, api_key: &str, language: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let part = reqwest::multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let mut form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", part);
    if !language.is_empty() {
        form = form.text("language", language.to_string());
    }

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Whisper API error {}: {}", status, body));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json["text"].as_str().unwrap_or("").trim().to_string())
}

/// Call OpenAI TTS API and return raw PCM samples (24kHz mono f32).
/// Uses the `tts-1` model with the selected voice.
pub async fn synthesize_openai_tts(
    text: &str,
    api_key: &str,
    voice: &str,
) -> Result<Vec<f32>, String> {
    if text.is_empty() { return Ok(Vec::new()); }
    // Truncate to ~4000 chars (TTS API limit is 4096)
    let text = if text.len() > 4000 { &text[..4000] } else { text };
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "tts-1",
        "input": text,
        "voice": voice,
        "response_format": "pcm",
    });
    let resp = client
        .post("https://api.openai.com/v1/audio/speech")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("TTS network error: {}", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("TTS API error {}: {}", status, body));
    }
    // OpenAI PCM format: 24kHz, 16-bit signed LE, mono
    let bytes = resp.bytes().await.map_err(|e| format!("TTS read error: {}", e))?;
    let samples: Vec<f32> = bytes.chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0)
        .collect();
    Ok(samples)
}

/// Play f32 PCM samples through the default output device. Blocks until done.
pub fn play_audio(samples: Vec<f32>, sample_rate: u32) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host.default_output_device()
        .ok_or("No audio output device found")?;
    let supported = device.supported_output_configs()
        .map_err(|e| format!("Audio config error: {}", e))?
        .find(|c| c.channels() == 1 && c.sample_format() == SampleFormat::F32)
        .or_else(|| device.supported_output_configs().ok()?
            .find(|c| c.sample_format() == SampleFormat::F32))
        .ok_or("No compatible audio output format")?;
    let config = supported.with_sample_rate(cpal::SampleRate(sample_rate)).into();
    let cursor = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let samples = Arc::new(samples);
    let done = Arc::new(AtomicBool::new(false));
    let done_cb = done.clone();
    let cursor_cb = cursor.clone();
    let samples_cb = samples.clone();
    let channels = supported.channels() as usize;
    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let pos = cursor_cb.load(Ordering::Relaxed);
            for frame in data.chunks_mut(channels) {
                let idx = pos + frame.len() / channels;
                let sample = if pos < samples_cb.len() { samples_cb[pos] } else { 0.0 };
                for s in frame.iter_mut() { *s = sample; }
                cursor_cb.store(idx.min(samples_cb.len()), Ordering::Relaxed);
            }
            if cursor_cb.load(Ordering::Relaxed) >= samples_cb.len() {
                done_cb.store(true, Ordering::Relaxed);
            }
        },
        |err| eprintln!("[hydra] audio output error: {}", err),
        None,
    ).map_err(|e| format!("Audio stream error: {}", e))?;
    stream.play().map_err(|e| format!("Audio play error: {}", e))?;
    // Wait for playback to finish
    while !done.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // Small tail to let audio buffer flush
    std::thread::sleep(std::time::Duration::from_millis(100));
    Ok(())
}
