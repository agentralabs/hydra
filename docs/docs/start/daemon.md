---
title: "Always-On Daemon"
description: "Hydra as a system service — starts at boot, never stops, self-writing genome."
---

## Install the Daemon


  
**macOS:**

    ```bash
    bash scripts/install-daemon-universal.sh
    ```
    Uses `launchd` with `com.agentra.hydra.plist`. Starts at login, restarts on crash.
  
  
**Linux:**

    ```bash
    bash scripts/install-daemon-universal.sh
    ```
    Uses `systemd` with `hydra.service`. Starts at boot, restarts on crash.
  


## Manage the Daemon

```bash
# Check status
bash scripts/install-daemon-universal.sh status

# Stop
bash scripts/install-daemon-universal.sh stop

# Start
bash scripts/install-daemon-universal.sh start

# Remove completely
bash scripts/install-daemon-universal.sh uninstall
```

## What Runs Continuously

| Thread | Frequency | What It Does |
|--------|-----------|-------------|
| **Active** | On demand | Responds when you connect via TUI |
| **Ambient** | Every 100ms | Health checks, invariants, signal dispatch |
| **Dream** | Every 500ms | Belief consolidation, self-writing genome |

## Self-Writing Genome

Every 20 ambient steps, the dream loop checks for patterns:

```
If automation engine detected a pattern:
  AND observed ≥ 5 times
  AND success rate ≥ 75%
  → New genome entry created automatically

Log: "hydra: GENOME SELF-WRITE — new entry (domain=devops, conf=80%, obs=5)"
```

Over time, Hydra needs the LLM less because the genome already has the answer.

## Logs

```bash
# macOS
tail -f ~/.hydra/logs/hydra.stdout.log

# Linux
journalctl -u hydra -f
```


:::warning

  The daemon must be running for scheduled actions, fleet agents, and the self-writing genome to work. Without it, Hydra only runs when you explicitly start it.

:::

