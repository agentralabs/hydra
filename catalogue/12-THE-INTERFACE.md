# 12 — The Interface

## The Cockpit

Hydra's interface is a terminal cockpit — a full-screen ratatui application with pinned regions, animated spinners, and semantic colors. Everything happens inside the TUI. No external windows. No browser. One cockpit.

## Two Views

### Welcome Screen
```
┌─ TOP FRAME (amber border) ─────────────────────────┐
│  ◈  H Y D R A              │  WORKING CONTEXT       │
│  ● alive │ v0.1.0 │ step 0 │  project: ~/hydra      │
│  Good morning, Omoshola.    │  branch: main · clean   │
│                              │                         │
│  COGNITIVE STATE             │  RECENT ACTIVITY        │
│  beliefs loaded   0          │  ✓ kernel boot complete │
│  skills active    13         │  ✓ constitution (7 laws)│
│  persona          core       │  ◑ genome: 13 entries   │
└──────────────────────────────┴─────────────────────────┘
┌─ BOTTOM FRAME (green border) ──────────────────────────┐
│  WHILE YOU WERE AWAY         │  ENTITY HEALTH           │
│  ○ No pending briefings      │  V(Ψ)  +1.00  stable    │
│                               │  Γ̂(Ψ)  +0.003 growing   │
│                               │  depth  0  morphic evts  │
│                               │  genome 13 entries       │
└───────────────────────────────┴──────────────────────────┘
  ◈  what are we building today?█
```

The HYDRA wordmark uses exact gradient colors:
```
H → #d4aa6e (gold)
Y → #d4aa6e (gold)
D → #e8c87a (gold bright)
R → #7ac87a (green)
A → #6ab8d4 (cyan)
```

Any keypress → transitions to cockpit. Ctrl+C → clean exit.

### Cockpit View
```
┌──────────────────────────────────────────────────┐
│  ▶ what is the circuit breaker pattern?          │
│  Netflix's famous approach was the circuit       │
│  breaker pattern, implemented through Hystrix... │
│    ℹ [fa2f0499|llm-short|19tok|1643ms|mw=8]    │
│                                                   │
│  ▶ why do rewrites fail?                         │
│  Most rewrites fail because they start from      │
│  code instead of interfaces...                   │
│    ℹ [cycle|2.1s|mw=8]                          │
├──────────────────────────────────────────────────┤
│  ◑ Cogitating                                     │
├──────────────────────────────────────────────────┤
│  ◈  [user types here]█                           │
├──────────────────────────────────────────────────┤
│ ◈ Hydra  session:5m  tasks:0  V=1.00  tokens:42 │
└──────────────────────────────────────────────────┘
```

## The Thinking Verb System

When Hydra is processing, an animated spinner shows what it is doing:

```
Spinner frames: ◌ → ◐ → ◑ → ◒ → ◓ → ●  (180ms per frame)
Verb rotation: every 2200ms

12 permanent verb contexts, each with its own color:
  General (amber):     Cogitating, Ruminating, Deliberating, Musing
  Forge (coral):       Forging, Smithing, Blueprinting, Crafting
  Codebase (cyan):     Scanning, Parsing, Traversing, Indexing
  Memory (green):      Remembering, Recollecting, Excavating, Surfacing
  Synthesis (purple):  Synthesizing, Ideating, Contemplating, Composing
  Workflow (blue):     Orchestrating, Sequencing, Pipelining, Routing
  Veritas (teal):      Verifying, Truthing, Validating, Cross-checking
  Aegis (red):         Shielding, Fortifying, Guarding, Sentineling
  Dream (indigo):      Dreaming, Drifting, Night-thinking, Star-gazing
  Persona (pink):      Channeling, Voicing, Shifting, Embodying
  Data (sage):         Crunching, Munging, Tabulating, Correlating
  Hydra (gold):        Hydrating, Multi-minding, Ring-resonating
```

Same verb = same color. Always. These assignments never change.

## The Output Pacer

Nothing bypasses the pacer. Every piece of content goes through speed control:
- **Sentence boundary**: 80ms pause (lets the eye track)
- **Paragraph boundary**: 120ms pause
- **Code block**: 200ms pause (signals importance)
- **Error content**: 0.5x speed (demands attention)
- **User scrolling**: 2x speed (they want to see more)
- **User typing**: 5x speed (they are ahead of the output)

Pacing carries information. Fast = routine. Slow = important. Hold = urgent.

## Voice (hydra-voice)

Push-to-talk inside the input box (Ctrl+V):
- Live transcription appears in the input box
- User can edit transcript before sending
- Voice input behaves exactly like typed input
- TTS playback toggleable

Visual indicators:
```
● Listening...
● Transcribing...
● Speaking...
```

## Companion (hydra-companion)

Background tasks visible in the stream:
```
⏵ Companion ▸ monitoring repo...                  ●
⏵ Companion ▸ found issue in auth-service         2.3s
```

Rules:
- All actions must be visible in chat history
- User can pause/stop any task
- No autonomous decisions without user approval

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-tui` | 2,477 | Cockpit rendering, pacer, verb system, stream |
| `hydra-companion` | 948 | Signal classification, background tasks |
| `hydra-voice` | 708 | Pulse STT/TTS, speculative processing |

## In Plain Terms

Imagine a cockpit where:
- Everything is one conversation (no tabs, no windows, no popups)
- The system shows you what it is thinking as it thinks (verb spinner)
- Output appears at human reading speed, not machine dump speed (pacer)
- Background work is always visible, never hidden (companion)
- You can talk instead of type, and the text appears before you finish speaking (voice)
- The health of the system is always visible at the bottom (status line)

That is Hydra's interface. One screen. Everything visible. Nothing hidden.
