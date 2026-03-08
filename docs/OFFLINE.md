# Offline Mode

Hydra can operate without internet connectivity using local models and cached resources.

## Local LLM Setup

### Ollama

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Pull a model
ollama pull llama3

# Configure Hydra to use it
export HYDRA_LLM_PROVIDER=ollama
export HYDRA_OLLAMA_MODEL=llama3
```

### Configuration

```toml
# ~/.hydra/config.toml
[llm]
provider = "ollama"
ollama_model = "llama3"
ollama_endpoint = "http://localhost:11434"
```

## Offline Capabilities

| Feature | Offline Support |
|---------|----------------|
| Cognitive loop | Full (with local LLM) |
| Local sisters | Full |
| Remote sisters | Queued for later |
| Voice (Whisper) | Full |
| Voice (Piper) | Full |
| Skill execution | Local skills only |
| Federation | LAN peers only |

## Offline Queue

When Hydra is offline, operations that require network access are queued:

- Remote API calls are stored in the offline queue
- When connectivity resumes, queued operations execute automatically
- Queue has a configurable maximum size (default: 1000 entries)

## Resource Profiles for Offline

Use the `minimal` profile for constrained environments:

```bash
export HYDRA_PROFILE=minimal
```

This limits token budget to 10,000 and concurrent runs to 2, suitable for local models on limited hardware.
