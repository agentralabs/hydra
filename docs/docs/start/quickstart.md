---
title: "Quickstart"
description: "3 minutes from clone to first response."
---

## Prerequisites



  ### Rust

    Rust 2024 edition. Install from [rustup.rs](https://rustup.rs)
  
  ### LLM API Key

    Anthropic, OpenAI, Gemini, or Ollama (local)
  



## Step 1: Clone

```bash
git clone git@github.com:agentralabs/hydra.git
cd hydra
```

## Step 2: Configure

```bash
cp .env.example .env
```

Edit `.env` and add your API key:

```bash .env
ANTHROPIC_API_KEY=sk-ant-your-key-here
```


:::tip

  Hydra supports 4 LLM providers. Set `HYDRA_LLM_PROVIDER` to `openai`, `gemini`, or `ollama` to use alternatives.

:::


## Step 3: Build and Run

```bash
cargo build --release -p hydra-kernel --bin hydra
cargo run --release -p hydra-kernel --bin hydra -- "what is the circuit breaker pattern?"
```

You should see a response enriched with genome knowledge:

```
Netflix's famous approach was the circuit breaker pattern, implemented
through their open-source library Hystrix...

[fa2f0499|llm-short|234tok|1847ms|mw=8]
```

The receipt footer `[session|path|tokens|duration|middlewares]` confirms the full pipeline ran.

## Step 4: Verify

```bash
# Run the harness — should show 47/47 (100%)
cargo run -p hydra-harness --bin harness -- --hours 1
```

## What to Do Next



  - **[Interactive Mode](/start/interactive-mode)** — 
    REPL for conversation
  
  - **[TUI Cockpit](/start/tui-cockpit)** — 
    Full terminal interface with animations
  
  - **[Always-On Daemon](/start/daemon)** — 
    Install Hydra as a system service
  
  - **[Add Skills](/extend/skills)** — 
    Teach Hydra your domain
  


