//! Voice commands — manage voice input/output

use crate::output;

pub fn start() {
    output::print_header("Voice Mode");
    output::print_info("Starting voice interface...");
    output::print_kv("STT engine", "Whisper (not configured)");
    output::print_kv("TTS engine", "Piper (not configured)");
    output::print_kv("Wake word", "Hey Hydra");
    output::print_kv("Status", "Voice pipeline not yet configured");
    output::print_info("Configure with: hydra config set voice.stt whisper");
}

pub fn stop() {
    output::print_header("Voice Stop");
    output::print_info("Stopping voice interface");
}

pub fn status() {
    output::print_header("Voice Status");
    output::print_kv("Active", "No");
    output::print_kv("STT", "Not configured");
    output::print_kv("TTS", "Not configured");
}
