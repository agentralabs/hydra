# HYDRA ACTIONS

## What This Folder Is

This is where Hydra's executable actions live — shell commands, API calls, scheduled jobs, notifications. Drop a TOML file and Hydra can perform a new action. No code required.

## Rules

1. **One folder per action** — named after what it does (e.g., `alert-owner/`, `create-video/`)
2. **`action.toml` is required** — defines trigger, command, parameters, approval mode
3. **Credentials go in `vault/`** — never in this folder
4. **Approval modes protect you** — `required` means Hydra asks before executing
5. **Timeouts are enforced** — no action runs forever

## Structure

```
actions/
  README.md                  ← this file
  alert-owner/
    action.toml              ← desktop notification (auto-approve)
  create-video/
    action.toml              ← video generation (requires approval)
  edit-video/
    action.toml              ← ffmpeg editing (requires approval)
  example-notify/
    action.toml              ← example notification
  example-scheduled/
    action.toml              ← scheduled disk check
  generate-carousel/
    action.toml              ← social carousel (requires approval)
  generate-social-post/
    action.toml              ← social post (requires approval)
  learn-from-video/
    action.toml              ← video learning (requires approval)
```

## Format

```toml
# action.toml
[action]
name = "alert-owner"
description = "Send a desktop notification to the user"
trigger = "conditional"        # manual | conditional | scheduled
approval = "auto"              # auto | notify | required

[execute]
type = "shell"
command = "osascript -e 'display notification \"{message}\" with title \"Hydra\"'"
timeout_seconds = 5

[[params]]
name = "message"
description = "The notification message"
required = true
```

## Approval Modes

| Mode | What Happens |
|------|-------------|
| `auto` | Executes immediately. For safe, read-only operations. |
| `notify` | Executes, then tells you what it did. |
| `required` | Asks before executing. You approve or deny. |

## How It Works

When Hydra determines an action should fire (from user request, scheduled trigger, or conditional match), it loads the action definition, resolves parameters, checks the approval mode, and executes. Results appear as tool dots in the TUI conversation stream.

Drop a TOML file. Hydra learns a new action. That is it.
