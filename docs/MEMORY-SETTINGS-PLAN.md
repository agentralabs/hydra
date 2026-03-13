# Memory Capture Settings — End-to-End Implementation Plan

## Goal

Let users control how Hydra's Memory sister stores conversation data.
Three modes: **Full Conversation**, **Facts Only**, **None** — each with
clear warnings about trade-offs.

---

## Current State

- `RuntimeSettings.memory_capture` field exists with 3 values: `"all"`, `"facts"`, `"none"`
- Hard-coded to `"all"` in `app_send_handler.rs:115`
- No UI control, no signal, not persisted in profile
- The LEARN phase in `phase_learn.rs` already branches on this field:
  - `"all"` → V3 immortal exchange capture + comm trail
  - `"facts"` → comm trail only (decisions, corrections, evidence still stored)
  - `"none"` → no capture at all (beliefs/patterns still update)

---

## Files to Touch

| # | File | Change | Lines Added |
|---|------|--------|-------------|
| 1 | `crates/hydra-native-state/src/profile.rs` | Add `memory_capture: String` field to `PersistedProfile` | ~4 |
| 2 | `crates/hydra-desktop/src/app_init_settings.rs` | Extract `memory_capture` from loaded profile | ~2 |
| 3 | `crates/hydra-desktop/src/main.rs` | Add `settings_memory_capture` signal, init from profile | ~2 |
| 4 | `crates/hydra-desktop/src/app_rsx_settings_behavior.rs` | Add Memory section with 3-option selector + warnings | ~30 |
| 5 | `crates/hydra-desktop/src/app_send_handler.rs` | Wire signal to `RuntimeSettings.memory_capture` + persist in profile | ~3 |

**No new files.** All changes are small additions to existing files.

---

## Step-by-Step Implementation

### Step 1: Add field to PersistedProfile

**File:** `crates/hydra-native-state/src/profile.rs`

Add to `PersistedProfile` struct:
```rust
pub memory_capture: Option<String>,  // "all", "facts", "none"
```

Using `Option<String>` (not bare `String`) for backward compatibility —
old profiles without this field deserialize as `None`, which defaults to `"all"`.

Add to `Default` impl:
```rust
memory_capture: Some("all".into()),
```

**Verify:** `cargo check -p hydra-native-state -j 1`

---

### Step 2: Extract from profile on startup

**File:** `crates/hydra-desktop/src/app_init_settings.rs`

Add to `InitSettings` struct:
```rust
pub memory_capture: String,
```

Add to `extract_init_settings()`:
```rust
memory_capture: profile.as_ref()
    .and_then(|p| p.memory_capture.clone())
    .unwrap_or_else(|| "all".into()),
```

**Verify:** `cargo check -p hydra-desktop -j 1`

---

### Step 3: Add signal in main.rs

**File:** `crates/hydra-desktop/src/main.rs`

Add signal near other behavior settings (around line 170):
```rust
let mut settings_memory_capture = use_signal(move || init.memory_capture.clone());
```

Where `init` is the `InitSettings` struct extracted from profile.

**Current file is 400 lines — must not exceed.** This adds 1 line.
Check that `init.memory_capture` is available in scope (the `init` struct
is constructed around line 85 and used through line 210).

**Verify:** `cargo check -p hydra-desktop -j 1` + `wc -l main.rs <= 400`

---

### Step 4: Add UI in Behavior settings tab

**File:** `crates/hydra-desktop/src/app_rsx_settings_behavior.rs`

Add a new **"Memory"** section BEFORE the "Intent Cache" section (memory is
more important for users to understand). This positions it as the first
setting users see in Behavior.

**UI Design:**

```
Memory
  How Hydra remembers your conversations

  [Full Conversation]  [Facts Only]  [None]
     (segmented control, like other behavior settings)

  Warning text (changes based on selection):
  - Full: "Hydra remembers everything — every message, decision, and context.
           Best for ongoing projects. Enables 'where did we stop?' recall."
  - Facts: "Hydra learns your preferences and decisions but forgets raw
            conversation text. Good balance of privacy and usefulness."
  - None: "Hydra forgets everything after this session. No learning occurs.
           Use for sensitive or one-off conversations."
```

**Implementation pattern** (matches existing segmented controls):
```rust
div { class: "settings-section",
    h3 { class: "settings-section-title", "Memory" }
    p { class: "settings-desc", style: "margin-bottom: 12px;",
        "Control how Hydra's memory sister stores your conversations."
    }
    div { class: "settings-row",
        span { class: "settings-label", "Capture Mode" }
        div { class: "segmented-control",
            { let opts = [("all", "Full Conversation"), ("facts", "Facts Only"), ("none", "None")];
              rsx! {
                for (val, label) in opts.iter() {
                    button {
                        class: if *settings_memory_capture.read() == *val { "segment active" } else { "segment" },
                        onclick: { let v = val.to_string(); move |_| settings_memory_capture.set(v.clone()) },
                        "{label}"
                    }
                }
            } }
        }
    }
    // Dynamic warning based on current selection
    {
        let mode = settings_memory_capture.read().clone();
        let (icon, warning) = match mode.as_str() {
            "all" => ("\u{1F4BE}", "Hydra remembers everything — every message, decision, and context. Best for ongoing projects. Enables \"where did we stop?\" recall."),
            "facts" => ("\u{1F512}", "Hydra learns preferences and decisions but forgets raw conversation. Good balance of privacy and usefulness."),
            _ => ("\u{26A0}", "Hydra forgets everything after this session. No learning occurs. Use for sensitive or one-off conversations."),
        };
        rsx! {
            div { class: "settings-row", style: "background: var(--bg-elevated); border-radius: 8px; padding: 12px;",
                p { class: "settings-desc", "{icon} {warning}" }
            }
        }
    }
}
```

**Estimated lines added:** ~25-30.
**Current file:** 175 lines → ~200-205 lines. Well under 400.

**Verify:** `cargo check -p hydra-desktop -j 1`

---

### Step 5: Wire signal to RuntimeSettings + persist

**File:** `crates/hydra-desktop/src/app_send_handler.rs`

**5a. Wire to RuntimeSettings** (replace hard-coded `"all"`):

Change line 115 from:
```rust
memory_capture: "all".into(),
```
To:
```rust
memory_capture: settings_memory_capture.read().clone(),
```

**5b. Persist in save_current_profile closure:**

Add to `PersistedProfile` construction (around line 393):
```rust
memory_capture: Some(settings_memory_capture.read().clone()),
```

**Current file:** 397 lines. These changes modify existing lines (net +1).

**Verify:** `cargo check -p hydra-desktop -j 1` + `wc -l app_send_handler.rs <= 400`

---

## Verification Checklist

After all steps complete:

- [ ] `cargo check -p hydra-native-state -j 1` — profile struct compiles
- [ ] `cargo check -p hydra-desktop -j 1` — desktop compiles
- [ ] `cargo check -p hydra-cli -j 1` — TUI still compiles (uses same RuntimeSettings)
- [ ] `wc -l` on all changed files — none exceed 400 lines
- [ ] `bash scripts/check-file-size-guard.sh` — passes

## Manual Test Plan

1. **Launch desktop** — open Settings > Behavior
2. **Verify default** — "Full Conversation" should be selected
3. **Switch to "Facts Only"** — warning text updates dynamically
4. **Switch to "None"** — warning text updates
5. **Click "Save & Close"** — profile saved
6. **Restart desktop** — verify setting persists (check ~/.hydra/profile.json)
7. **Send a message in "Facts Only" mode** — check logs for:
   - `comm_session_log` SHOULD fire
   - `memory_capture_exchange` should NOT fire
8. **Send a message in "None" mode** — check logs:
   - Neither `comm_session_log` nor `memory_capture_exchange` should fire
9. **Switch back to "Full Conversation"** — verify full capture resumes

## Edge Cases Handled

| Edge Case | How It's Handled |
|-----------|-----------------|
| Old profile without `memory_capture` field | `Option<String>` → `None` → defaults to `"all"` |
| User changes mid-conversation | Takes effect on NEXT message (RuntimeSettings built per-message) |
| TUI uses same RuntimeSettings | TUI already passes `memory_capture: "all"` — no change needed unless TUI settings are added later |
| Beliefs still captured in "none" mode | By design — beliefs are separate from conversation capture. The LEARN phase belief extraction runs independently of `memory_capture` |
| "Facts" mode still stores decisions/evidence | Correct — `should_capture_facts()` returns true for "facts", allowing `memory_add()` calls for corrections/decisions/evidence. Only V3 immortal full-transcript is skipped |

## Not In Scope

- TUI memory settings (TUI has no settings UI for behavior)
- Per-session override (always uses global setting)
- Memory deletion/export UI (separate feature)
- Wake word implementation (unrelated)
