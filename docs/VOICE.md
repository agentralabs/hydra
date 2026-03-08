# Voice Commands

Hydra includes a voice interface for hands-free interaction.

## Setup

```bash
# Enable voice in config
export HYDRA_VOICE=true
```

Or in `~/.hydra/config.toml`:

```toml
[voice]
enabled = true
wake_word = "hey hydra"
engine = "whisper"    # whisper (local) or cloud
```

## Requirements

- **Whisper** for speech-to-text (local, offline-capable)
- **Piper** for text-to-speech (local, offline-capable)
- Microphone access

## Commands

| Voice Command | Action |
|--------------|--------|
| "Hey Hydra, run [task]" | Start a new task |
| "Hey Hydra, status" | Check current status |
| "Hey Hydra, approve" | Approve pending action |
| "Hey Hydra, deny" | Deny pending action |
| "Hey Hydra, stop" | Cancel current run |
| "Hey Hydra, kill" | Emergency stop |

## Desktop Globe

The desktop app features a Siri-style globe that:

- **Idle** — Slow ambient animation
- **Listening** — Pulsing glow
- **Processing** — Spinning with phase colors
- **Speaking** — Wave-form animation

## Current Status

Voice infrastructure is built but not production-ready in V1. The desktop app globe UI works as a visual indicator of cognitive phases.
