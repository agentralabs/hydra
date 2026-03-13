// Shared voice start/stop logic — included in the main component.
// Usage: `let toggle_voice = include!("app_voice_trigger.rs");`
// In companion mode: uses silence detection (auto-stop after 1.5s quiet).
// In workspace mode: manual stop via button click.
// Supports barge-in: if TTS is playing, cancels it before starting mic.
// After transcription, sets `voice_pending_send` signal.
{
    use dioxus::prelude::Callback;

    let cb = Callback::new(move |()| {
        let listening = *voice_listening.read();
        if listening {
            // Manual stop — break the auto-listen cycle entirely
            mic_stop_flag.read().store(true, std::sync::atomic::Ordering::Relaxed);
            companion_auto_listen.set(false);
            cognitive_done.set(false);
            eprintln!("[hydra:voice] Manual stop — auto-listen disabled");
        } else {
            // Barge-in: cancel TTS if it's playing so mic doesn't capture Hydra's voice
            if *tts_playing.peek() {
                eprintln!("[hydra:voice] Barge-in — cancelling TTS");
                pulse.read().cancel_tts();
                tts_playing.set(false);
            }
            let openai_key = settings_openai_key.read().clone();
            if openai_key.is_empty() {
                active_error.set(Some(FriendlyError {
                    message: "Voice input requires an OpenAI API key".into(),
                    explanation: "Go to Settings > Models and enter your OpenAI key.".into(),
                    options: vec![], icon_state: "error".into(), can_undo: false,
                }));
                return;
            }
            voice_listening.set(true);
            cognitive_done.set(false); // reset for next cycle
            let flag = Arc::new(AtomicBool::new(false));
            mic_stop_flag.set(flag.clone());
            let is_companion = *current_mode.read() == "companion";

            let (tx, rx) = tokio::sync::oneshot::channel::<Option<(Vec<f32>, u32)>>();
            std::thread::spawn(move || {
                let result = if is_companion {
                    // Auto-stop: 1.5s silence, 0.3s min speech, 0.02 RMS threshold
                    voice_capture::record_until_silence(flag, 1.5, 0.3, 0.02)
                } else {
                    voice_capture::record_until_stopped(flag)
                };
                let _ = tx.send(result);
            });

            spawn(async move {
                if let Ok(Some((samples, sample_rate))) = rx.await {
                    if samples.len() > 1600 {
                        let wav = voice_capture::encode_wav(&samples, sample_rate);
                        let key = settings_openai_key.read().clone();
                        let lang = settings_stt_lang.read().clone();
                        match voice_capture::transcribe_whisper(wav, &key, &lang).await {
                            Ok(text) if !text.is_empty() => {
                                eprintln!("[hydra:voice] Transcribed: {}", &text[..text.len().min(80)]);
                                voice_pending_send.set(Some(text.clone()));
                                input.set(text);
                            }
                            Err(e) => {
                                eprintln!("[hydra:voice] Transcription error: {}", e);
                                active_error.set(Some(FriendlyError {
                                    message: "Transcription failed".into(), explanation: e,
                                    options: vec![], icon_state: "error".into(), can_undo: false,
                                }));
                            }
                            _ => { eprintln!("[hydra:voice] Empty transcription"); }
                        }
                    } else {
                        eprintln!("[hydra:voice] Too short ({} samples)", samples.len());
                    }
                } else {
                    active_error.set(Some(FriendlyError {
                        message: "Microphone not available".into(),
                        explanation: "No input device found.".into(),
                        options: vec![], icon_state: "error".into(), can_undo: false,
                    }));
                }
                voice_listening.set(false);
            });
        }
    });
    cb
}
