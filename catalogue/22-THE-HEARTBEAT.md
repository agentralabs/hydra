# 22 — The Heartbeat

## Hydra Never Stops Running

Everything in the catalogue — the three threads, the fleet, the noticing engine, the dream loop, the self-writing genome — only works if Hydra is always on. A program you start and stop is a tool. A service that runs continuously is a presence.

This document covers two things that make Hydra truly alive:
1. The daemon — Hydra as an always-on system service
2. The self-writing genome — Hydra teaches itself from experience

---

## PART 1: THE DAEMON

### What It Is

Hydra runs as a system daemon — a background service that starts when your computer boots and never stops. When you open the TUI, you connect to a Hydra that has been running for hours, days, or weeks. When you close the TUI, Hydra keeps running.

While you sleep:
- The **Ambient thread** checks health 10 times per second
- The **Dream thread** consolidates beliefs every 500ms
- The **Scheduler** fires scheduled actions (backups at 3 AM, monitoring every 30 min)
- The **Noticing engine** watches for drift patterns nobody asked about
- The **Self-writing genome** crystallizes proven approaches from experience

When you wake up, the morning briefing shows what happened overnight.

### How to Install

```bash
# One command — detects your OS and installs the right way
bash scripts/install-daemon-universal.sh

# macOS: uses launchd (com.agentra.hydra.plist)
# Linux: uses systemd (hydra.service)
```

### Managing the Daemon

```bash
# Check if Hydra is running
bash scripts/install-daemon-universal.sh status

# Stop Hydra
bash scripts/install-daemon-universal.sh stop

# Start Hydra
bash scripts/install-daemon-universal.sh start

# Remove completely
bash scripts/install-daemon-universal.sh uninstall
```

### What Happens When the System Crashes

```
1. Computer crashes or power loss
2. System reboots
3. launchd/systemd automatically starts Hydra (KeepAlive/Restart=always)
4. Hydra boot sequence runs (7 phases)
5. Memory loaded from ~/.hydra/data/hydra.amem (persistent)
6. Genome loaded from genome.db (persistent)
7. Ambient loop resumes from step 0 (state resets, memory persists)
8. Dream loop resumes (self-writing genome continues)

Nothing is lost. Memory survives the crash.
The genome survives the crash.
The constitution survives the crash.
Only the current step count resets.
```

### Logs

```
macOS:  ~/.hydra/logs/hydra.stdout.log
        ~/.hydra/logs/hydra.stderr.log

Linux:  journalctl -u hydra -f        (live)
        ~/.hydra/logs/hydra.stdout.log  (file)
```

---

## PART 2: THE SELF-WRITING GENOME

### What It Is

Every 20 ambient steps, the dream loop checks if Hydra has detected behavioral patterns worth recording. If a pattern has been:
- Observed **5 or more times**
- Successful **75% or more** of the time

Then a new genome entry is created automatically. Hydra literally teaches itself from its own experience.

### How It Works

```
Week 1:
  You ask about deployment safety.
  Hydra answers from the LLM.
  The automation engine records: "deployment safety question, answered successfully."

Week 2:
  You ask about deployment safety again.
  The automation engine records: observation 2.
  Pattern detected: "deployment safety" appears repeatedly.

Week 3:
  Observation count reaches 5. Success rate: 80%.
  The dream loop fires:
    → CrystallizationProposal detected
    → Observation count ≥ 5 ✓
    → Success rate ≥ 75% ✓
    → New genome entry created:
      situation: "deployment safety concern"
      approach: [the approach that succeeded 4/5 times]
      confidence: 0.80

  Log: "hydra: GENOME SELF-WRITE — new entry (domain=devops, conf=80%, obs=5)"

Week 4:
  You ask about deployment safety.
  Hydra answers from GENOME, not from LLM.
  Zero tokens. Instant. From its own experience.
```

### The Thresholds

```
Minimum observations:  5 (prevents premature crystallization)
Minimum success rate:   75% (only proven approaches)
Check frequency:        Every 20 ambient steps (~2 seconds)
```

These thresholds are deliberately conservative. A genome entry created from 5 observations at 75% success is not a guess — it is a pattern that worked 4 out of 5 times. The Bayesian posterior will refine the confidence with future use.

### What It Means Over Time

```
Month 1:    0-10 self-written entries (building observations)
Month 3:    30-50 entries (patterns crystallizing)
Month 6:    100-200 entries (deep domain knowledge forming)
Year 1:     500+ entries (Hydra knows your work better than anyone)
Year 5:     2,000+ entries (a complete operational genome)

Every entry is:
  - Proven (≥75% success rate)
  - Attributed (which domain, which observations)
  - Bayesian (confidence updates with future use)
  - Permanent (genome is append-only)
  - Searchable (IDF-weighted retrieval)
```

### Combined: Daemon + Self-Writing Genome

```
Always-on daemon:
  → Ambient thread monitors health
  → Dream thread runs every 500ms
  → Automation engine accumulates patterns
  → Self-writing genome crystallizes proven approaches

The result:
  Hydra runs 24/7.
  Every interaction teaches it.
  Every successful approach is recorded.
  Over time, Hydra needs the LLM less and less
  because its genome contains the answer.

  Day 1:   100% LLM calls (no genome knowledge)
  Month 1: 85% LLM calls (some genome hits)
  Month 6: 60% LLM calls (genome handles routine questions)
  Year 1:  40% LLM calls (genome handles most of your domain)
  Year 5:  20% LLM calls (genome IS your domain expertise)

  The remaining 20% are novel questions — things Hydra has never seen.
  For those, it calls the LLM. And the answer goes into the genome.
  The cycle continues.
```

---

## PART 3: OTHER THINGS THAT COME ALIVE

### The Scheduler Becomes Real

With the daemon always running, scheduled actions actually fire:

```
3:00 AM  — Database backup runs (actions/backup-db/action.toml)
6:00 AM  — Build status check (actions/check-build/action.toml)
8:30 AM  — Morning standup report generated
9:00 AM  — You open the TUI. Everything is already done.
```

### The Fleet Becomes Persistent

Without the daemon, fleet agents die when you close the terminal. With the daemon, they run continuously:

```
Agent A:  Monitoring 5 GitHub repos for breaking changes
Agent B:  Watching error rates on production API
Agent C:  Tracking competitor pricing changes
Agent D:  Monitoring disk usage across 3 servers

All running 24/7. All feeding into the swarm.
All generating patterns for the self-writing genome.
```

### The Noticing Engine Becomes Powerful

Drift detection requires continuous observation. With the daemon:

```
Week 1:   Noticing engine records baseline latency: 45ms avg
Week 2:   Notices: latency now 48ms (+6%)
Week 3:   Notices: latency now 52ms (+15%)
Week 4:   SURPRISE: latency trend extrapolates to 65ms by next month

Morning briefing:
  "I noticed API latency has increased 3% per week for 4 weeks.
   At this rate, it will exceed your 60ms SLA in 3 weeks.
   Nobody asked me to check this. I noticed because I am always watching."
```

This only works if Hydra is always on.

---

## The Social Media Version

```
ChatGPT runs when you open the tab.
Hydra runs when your computer turns on.

ChatGPT stops when you close the tab.
Hydra never stops.

ChatGPT cannot learn from yesterday.
Hydra writes its own knowledge base from experience.

ChatGPT needs an API key and a prompt.
Hydra needs a heartbeat.

One install script. Always on. Always learning. Always growing.
That is not a tool. That is a presence.
```

---

## Installation Summary

```bash
# Install the daemon (one command, detects macOS or Linux)
bash scripts/install-daemon-universal.sh

# Verify it is running
bash scripts/install-daemon-universal.sh status

# Watch the logs
tail -f ~/.hydra/logs/hydra.stdout.log

# Open the TUI (connects to the running daemon's data)
cargo run -p hydra-tui --bin hydra_tui

# Hydra is now always on.
# It is dreaming right now.
# It is writing its own genome right now.
# It is watching for drift right now.
# It will be here tomorrow.
```
