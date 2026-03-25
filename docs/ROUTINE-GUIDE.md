# Hydra Routine Guide — Training & Custom Routines

## What Are Routines?

Routines are scheduled task sequences that Hydra executes automatically. They serve two purposes:

1. **Training** — The default 15 routines exercise every capability area so Hydra's genome, muscle memory, and state graphs accumulate real experience.
2. **Automation** — You can create custom routines for any recurring task (daily reports, monitoring, content creation, etc.).

## Default 15 Training Routines

| # | Name | Time | Area | Active From |
|---|------|------|------|-------------|
| 00 | daily-health | 6:00 AM | Health & self-test | Day 1 |
| 01 | shell-mastery | 7:00 AM | Shell commands | Day 1 |
| 02 | browser-stealth | 8:00 AM | Browser automation | Day 1 |
| 03 | desktop-basics | 9:00 AM | Desktop control | Day 1 |
| 04 | coding-pipeline | 10:00 AM | Code generation | Day 1 |
| 05 | web-knowledge | 11:00 AM | Intelligence & search | Day 1 |
| 06 | security-immune | 12:00 PM | Security & threats | Day 1 |
| 07 | monitoring | 1:00 PM | System monitoring | Day 1 |
| 08 | learning-genome | 2:00 PM | Genome & learning | Day 1 |
| 09 | desktop-advanced | 3:00 PM | Muscle memory, OCR | Day 5 |
| 10 | proactive-autonomy | 4:00 PM | Autonomy & judgment | Day 7 |
| 11 | remote-collaboration | 5:00 PM | API & remote access | Day 10 |
| 12 | evolution-self | 6:00 PM | Self-evolution | Day 14 |
| 13 | video-creative | 7:00 PM | Creative & video | Day 10 |
| 14 | full-integration | 8:00 PM | Cross-capability | Day 7 |

## TOML Format Reference

```toml
[routine]
name = "my-routine"                    # Unique name (required)
description = "What this routine does" # Human-readable (required)
capability_area = "shell"              # Category for tracking (required)
schedule = "daily 09:00"              # When to run (required)
enabled = true                         # Set false to disable (default: true)
difficulty = 3                         # 1-5 scale (default: 1)
day_start = 5                          # Start on training day N (optional)
day_end = 30                           # Stop after training day N (optional)

[[steps]]
goal = "Do something specific"         # Natural language goal (required)
step_type = "shell"                    # shell | desktop | browser | verify | api
timeout_secs = 60                      # Max time for this step (default: 60)
success_criteria = "output contains X" # Optional success check
```

### Schedule Formats

| Format | Example | Meaning |
|--------|---------|---------|
| `daily HH:MM` | `daily 09:00` | Every day at 9:00 AM |
| `weekly DAY HH:MM` | `weekly mon 10:00` | Every Monday at 10:00 AM |
| `hourly` | `hourly` | Every hour |
| `every Nm` | `every 30m` | Every 30 minutes |
| `every Nh` | `every 2h` | Every 2 hours |

### Step Types

| Type | What It Does |
|------|-------------|
| `shell` | Runs a shell command via conductor |
| `desktop` | Controls desktop app via AMM 6-layer stack |
| `browser` | Automates browser via Chrome DevTools |
| `verify` | Checks a condition without side effects |
| `api` | Makes an HTTP API call |

### Capability Areas

Use any string. Common areas: `health`, `shell`, `browser`, `desktop`, `coding`, `intelligence`, `security`, `monitoring`, `learning`, `desktop-advanced`, `autonomy`, `remote`, `evolution`, `creative`, `integration`

## How to Create Your Own Routine

### Step 1: Write a TOML file

```toml
[routine]
name = "youtube-upload-check"
description = "Check YouTube channel stats and plan next upload"
capability_area = "content"
schedule = "daily 10:00"

[[steps]]
goal = "Open browser, navigate to YouTube Studio, take screenshot of analytics"
step_type = "browser"
timeout_secs = 60

[[steps]]
goal = "Extract view count and subscriber count from the page"
step_type = "browser"
timeout_secs = 30

[[steps]]
goal = "Write a summary report to ~/content/daily-stats.md"
step_type = "shell"
timeout_secs = 30
```

### Step 2: Install it

**Option A** — Drop into gateway:
```bash
cp youtube-upload-check.routine.toml ~/.hydra/drop/
```

**Option B** — Place directly:
```bash
cp youtube-upload-check.routine.toml ~/.hydra/routines/
```

### Step 3: Verify

Hydra will pick it up on the next proactive tick (~30 seconds). Check logs:
```
hydra-routine: loaded youtube-upload-check
```

## Managing Routines

### Disable a routine
Edit the TOML file and set `enabled = false`:
```toml
[routine]
enabled = false
```

### View training progress
Ask Hydra: "show my training progress" or check:
```bash
cat ~/.hydra/routines/history.jsonl | tail -20
```

### View run history
```bash
cat ~/.hydra/routines/history.jsonl
```
Each line is a JSON record: routine name, timestamp, success, steps completed, duration.

## Example Custom Routines

### Video Content Workflow
```toml
[routine]
name = "daily-content-edit"
description = "Edit today's raw footage into a polished video"
capability_area = "video"
schedule = "daily 14:00"

[[steps]]
goal = "Open DaVinci Resolve"
step_type = "desktop"
timeout_secs = 30

[[steps]]
goal = "Import the latest .mp4 file from ~/raw-footage/"
step_type = "desktop"
timeout_secs = 60

[[steps]]
goal = "Cut dead air sections (silence longer than 3 seconds)"
step_type = "desktop"
timeout_secs = 120

[[steps]]
goal = "Add intro title card from ~/templates/intro.png"
step_type = "desktop"
timeout_secs = 60

[[steps]]
goal = "Export as MP4, 1080p, H.264, to ~/exports/"
step_type = "desktop"
timeout_secs = 120
```

### Daily Code Review
```toml
[routine]
name = "daily-code-review"
description = "Review yesterday's commits for quality"
capability_area = "coding"
schedule = "daily 09:00"

[[steps]]
goal = "Run git log --since=yesterday --oneline and list all commits"
step_type = "shell"

[[steps]]
goal = "For each commit, run git diff and check for security issues"
step_type = "shell"

[[steps]]
goal = "Run the full test suite and report any failures"
step_type = "shell"

[[steps]]
goal = "Write a summary to ~/reports/code-review-today.md"
step_type = "shell"
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Routine not firing | Check `enabled = true` and schedule time has passed |
| Wrong time zone | Schedule uses UTC — adjust hours accordingly |
| Step timing out | Increase `timeout_secs` for that step |
| Routine not found | Verify file is in `~/.hydra/routines/` with `.toml` extension |
| Parse error | Check TOML syntax — `[routine]` section is required |
| Day range wrong | `day_start` counts from first routine install date |
