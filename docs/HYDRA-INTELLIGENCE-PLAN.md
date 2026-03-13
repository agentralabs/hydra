# HYDRA INTELLIGENCE PLAN — Beyond Raw LLM

**Created**: 2026-03-13
**Status**: PLAN
**Depends on**: HYDRA-SUPERINTELLIGENCE-PLAN.md (Phases 1-7, all wired)
**Goal**: Make Hydra noticeably smarter than talking to Claude/GPT directly, while still depending on their reasoning. Intelligence = Context + Memory + Accuracy + Anticipation + Personalization.

**Core Insight**: A user should feel "I can't go back to raw Claude" after using Hydra for a week. That requires Hydra to remember, anticipate, personalize, and never be wrong about verifiable facts.

---

## Current State (2026-03-13)

| Feature | Status | Gap |
|---------|--------|-----|
| Outcome/Calibration tracking | IN-MEMORY ONLY | Dies every session. No learning accumulates. |
| Codebase graph index | MCP TOOL CALLS ONLY | No local index. Every query requires sister round-trip. |
| File watcher / proactive triggers | SPEC ONLY | `ProactiveTrigger::FileChanged` defined, no actual watcher. |
| Response verification | SELECTIVE | Skips greetings, short responses, simple intents. |
| User profile learning | STATIC CONFIG | Theme/font preferences only. No behavioral learning. |

---

## P0: Persistent Intelligence (DB-Backed Trackers)

**Why first**: Everything else (model routing, self-improvement, calibration) is useless if it resets every session. This is the foundation.

### Schema Changes — `hydra-db/src/schema_tables.rs`

Add 3 new tables (append to existing CREATE_TABLES):

```sql
CREATE TABLE IF NOT EXISTS outcome_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    intent_category TEXT NOT NULL,
    topic TEXT NOT NULL DEFAULT '',
    model_used TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK(outcome IN ('success','correction','failure','repeat','neutral')),
    tokens_used INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS calibration_buckets (
    bucket_index INTEGER PRIMARY KEY CHECK(bucket_index BETWEEN 0 AND 9),
    total INTEGER NOT NULL DEFAULT 0,
    successes INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS user_profile_learned (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trait_key TEXT NOT NULL UNIQUE,
    trait_value TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.5,
    observation_count INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_outcome_category ON outcome_history(intent_category);
CREATE INDEX IF NOT EXISTS idx_outcome_created ON outcome_history(created_at);
CREATE INDEX IF NOT EXISTS idx_user_trait ON user_profile_learned(trait_key);
```

### Implementation Files

#### 1. `hydra-db/src/store_intelligence.rs` (~120 lines) — NEW
DB methods for intelligence persistence:
- `save_outcome(category, topic, model, outcome, tokens)` — insert into outcome_history
- `load_outcomes(limit: u64) -> Vec<OutcomeRow>` — load recent outcomes
- `load_category_stats() -> Vec<(String, u64, u64)>` — aggregate (category, total, successes)
- `save_calibration_buckets(buckets: &[(u64, u64); 10])` — upsert all 10 buckets
- `load_calibration_buckets() -> [(u64, u64); 10]` — load bucket data
- `save_user_trait(key, value, confidence)` — upsert learned trait
- `load_user_traits() -> Vec<(String, String, f64)>` — all learned traits

#### 2. `hydra-native-cognitive/.../phase_learn_intelligence.rs` — MODIFY (~+30 lines)
- After recording outcome in OutcomeTracker: `db.save_outcome(...)`
- After calibration update: `db.save_calibration_buckets(...)`
- Add `load_from_db(db, tracker, calibration)` function

#### 3. `loop_runner.rs` — MODIFY (~+5 lines)
- Before `populate_from_history()`: call `load_from_db()` if db is available
- This means OutcomeTracker starts with ALL historical data, not just current session

#### 4. `hydra-db/src/schema_tables.rs` — MODIFY
- Append new table definitions to CREATE_TABLES
- Bump SCHEMA_VERSION to 2

### Tests
- `hydra-db/src/store_intelligence_tests.rs` (~80 lines) — save/load roundtrip tests
- Add to existing `tests/suite/main.rs`

### Verification
- `cargo test -p hydra-db -j 1`
- `cargo check -p hydra-native-cognitive -j 1`
- After 5 sessions, verify outcomes accumulate across restarts

---

## P1: Codebase Semantic Index

**Why second**: Users ask code questions constantly. Instant answers from a local index vs. 2-second sister round-trip = perceptible intelligence difference.

### Architecture

```
Background (Phase 5 scheduler)
    │
    ▼
CodebaseIndexer (runs during idle)
    ├─ Walk .rs/.ts/.py files
    ├─ Extract: function signatures, struct/type definitions, imports
    ├─ Build: symbol → file:line map, caller → callee edges
    └─ Store in DB (code_symbols + code_edges tables)

PERCEIVE phase
    ├─ Query local index for symbols mentioned in user text
    ├─ Inject relevant file:line locations into system prompt
    └─ Fallback to Codebase sister for deep queries
```

### Schema Changes

```sql
CREATE TABLE IF NOT EXISTS code_symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('function','struct','enum','trait','type','const','mod','impl')),
    file_path TEXT NOT NULL,
    line_number INTEGER NOT NULL,
    signature TEXT,
    visibility TEXT DEFAULT 'private',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS code_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_symbol TEXT NOT NULL,
    to_symbol TEXT NOT NULL,
    edge_type TEXT NOT NULL CHECK(edge_type IN ('calls','imports','implements','contains')),
    file_path TEXT NOT NULL,
    line_number INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_code_sym_name_file ON code_symbols(name, file_path, line_number);
CREATE INDEX IF NOT EXISTS idx_code_sym_kind ON code_symbols(kind);
CREATE INDEX IF NOT EXISTS idx_code_edge_from ON code_edges(from_symbol);
CREATE INDEX IF NOT EXISTS idx_code_edge_to ON code_edges(to_symbol);
```

### Implementation Files

#### 1. `hydra-kernel/src/code_index.rs` (~200 lines) — NEW
Regex-based symbol extractor (no tree-sitter dependency, keeps builds fast):
- `extract_rust_symbols(file_content: &str) -> Vec<Symbol>` — regex for `fn `, `struct `, `enum `, `trait `, `impl `, `type `, `const `, `mod `
- `extract_rust_calls(file_content: &str, defined_symbols: &[String]) -> Vec<Edge>` — find symbol references
- `Symbol { name, kind, line, signature, visibility }`
- `Edge { from_symbol, to_symbol, edge_type, file_path, line }`

#### 2. `hydra-kernel/src/code_index_walk.rs` (~150 lines) — NEW
Filesystem walker for indexing:
- `index_project(root: &Path, db: &HydraDb) -> IndexResult` — walk files, extract symbols, store in DB
- `incremental_update(root: &Path, db: &HydraDb, since: DateTime) -> IndexResult` — only changed files
- Respects `.gitignore`, skips `target/`, `node_modules/`
- Reports progress: files scanned, symbols found, edges detected

#### 3. `hydra-kernel/src/code_index_query.rs` (~100 lines) — NEW
Query interface for the cognitive loop:
- `find_symbol(db: &HydraDb, name: &str) -> Vec<SymbolLocation>` — where is this symbol defined?
- `find_callers(db: &HydraDb, symbol: &str) -> Vec<CallerInfo>` — who calls this?
- `find_callees(db: &HydraDb, symbol: &str) -> Vec<CalleeInfo>` — what does this call?
- `impact_analysis(db: &HydraDb, symbol: &str) -> ImpactReport` — what breaks if I change this?
- `symbols_in_file(db: &HydraDb, path: &str) -> Vec<Symbol>` — all symbols in a file

#### 4. `hydra-db/src/store_code_index.rs` (~100 lines) — NEW
DB methods for code index persistence:
- `upsert_symbols(symbols: &[Symbol])`
- `upsert_edges(edges: &[Edge])`
- `query_symbol(name: &str) -> Vec<SymbolRow>`
- `query_callers(symbol: &str) -> Vec<EdgeRow>`
- `clear_file_symbols(file_path: &str)` — for incremental updates

#### 5. Wire into BackgroundScheduler — MODIFY `background_tasks.rs`
- Add `BackgroundTaskType::CodeIndexUpdate` with 10-minute interval, Normal priority
- Wire execution: when this task is due, call `incremental_update()`

#### 6. Wire into PERCEIVE phase — MODIFY `phase_perceive.rs`
- Before sister calls: query local index for symbols mentioned in user text
- Add matches to `PerceiveResult` as `local_code_context: Vec<SymbolLocation>`
- Inject into system prompt: "Relevant code: `fn foo()` at `src/bar.rs:42`"

### Tests
- `hydra-kernel/src/code_index_tests.rs` (~150 lines)
  - Test Rust symbol extraction (fn, struct, enum, trait, impl)
  - Test call edge detection
  - Test `.gitignore` respecting
  - Test incremental update (only re-indexes changed files)
- `hydra-db/src/store_code_index_tests.rs` (~80 lines)
  - DB round-trip for symbols and edges

### Verification
- `cargo test -p hydra-kernel -j 1` — extraction tests
- `cargo test -p hydra-db -j 1` — DB tests
- Manual: type "what calls run_cognitive_loop" → Hydra answers instantly from index

---

## P2: Proactive File Watcher

**Why third**: This is the "wow" moment. Hydra notices things before you ask.

### Architecture

```
FileWatcher (background thread)
    ├─ Watches project root via `notify` crate
    ├─ Debounces events (100ms)
    ├─ Classifies: new file, modified, deleted, git conflict
    └─ Sends ProactiveEvent to cognitive loop

ProactiveEngine
    ├─ Receives file events
    ├─ Correlates: "test file changed but no test run" → suggest
    ├─ Detects: merge conflicts, broken imports, stale locks
    └─ Sends CognitiveUpdate::ProactiveSuggestion to UI
```

### Dependencies
- Add `notify = "6"` to `hydra-pulse/Cargo.toml` (cross-platform file watching)

### Implementation Files

#### 1. `hydra-pulse/src/file_watcher.rs` (~150 lines) — NEW
- `FileWatcher::new(root: PathBuf) -> Self`
- `start(tx: mpsc::UnboundedSender<FileEvent>)` — spawns background thread
- `stop()` — graceful shutdown
- `FileEvent { path, kind: Created|Modified|Deleted, timestamp }`
- Debounce: coalesce events within 100ms window
- Ignore: `target/`, `.git/objects`, `node_modules/`, `*.swp`, `*.lock` (write-only)

#### 2. `hydra-pulse/src/proactive_engine.rs` (~200 lines) — NEW (or extend existing)
Correlate file events into proactive suggestions:
- `on_file_event(event: FileEvent) -> Option<ProactiveSuggestion>`
- Detections:
  - `*.rs` modified but no `cargo check` in last 5 min → "Want me to check for errors?"
  - `Cargo.toml` changed → "Dependencies changed. Run `cargo update`?"
  - `.git/MERGE_HEAD` exists → "You have unresolved merge conflicts in: ..."
  - Test file modified → "Run tests for this module?"
  - New untracked file → "New file detected: `foo.rs`. Add to git?"
  - `Cargo.lock` conflict → "Lock file conflict detected. Resolve with `cargo update`?"

#### 3. CognitiveUpdate addition — MODIFY `cognitive_update.rs`
```rust
ProactiveSuggestion { title: String, message: String, action: Option<String> },
```

#### 4. Wire into desktop/CLI — MODIFY app startup
- Start FileWatcher on app launch with project root
- Route FileEvents through ProactiveEngine
- Display suggestions in UI (toast/banner)

### Tests
- `hydra-pulse/src/file_watcher_tests.rs` (~80 lines)
  - Test debouncing
  - Test ignore patterns
  - Test event classification
- `hydra-pulse/src/proactive_engine_tests.rs` (~100 lines)
  - Test: .rs change → check suggestion
  - Test: merge conflict detection
  - Test: no spam (same suggestion not repeated within 5 min)

### Verification
- `cargo test -p hydra-pulse -j 1`
- Manual: edit a `.rs` file → see Hydra suggest "check for errors?"
- Manual: create merge conflict → see Hydra detect it

---

## P3: Never-Wrong Verification

**Why fourth**: Trust is the adoption multiplier. One wrong file path kills credibility.

### Current Gap
`verify_response.rs` has `should_verify()` that skips:
- Greetings/farewells/thanks (OK to skip)
- Responses < 80 chars (BAD — short responses can still have wrong file paths)
- Simple intents (BAD — simple answers can reference wrong symbols)

### Changes

#### 1. `verify_response.rs` — MODIFY `should_verify()` (~10 lines changed)
Change from intent-based gating to content-based gating:
```rust
fn should_verify(response: &str, _intent: &ClassifiedIntent) -> bool {
    // Always verify if response contains verifiable claims
    let has_file_paths = response.contains('/') || response.contains(".rs") || response.contains(".ts");
    let has_code_symbols = response.contains("fn ") || response.contains("struct ") || response.contains("class ");
    let has_numbers = response.chars().any(|c| c.is_ascii_digit());
    has_file_paths || has_code_symbols || has_numbers
}
```

#### 2. `verify_response.rs` — ADD fast-path verification (~30 lines)
For short responses (< 200 chars), only verify file paths (cheapest check):
```rust
fn fast_verify_paths(response: &str) -> Vec<Correction> {
    // Extract paths, check if they exist on filesystem
    // No sister calls needed — pure filesystem check
}
```

#### 3. Confidence display — MODIFY system prompt or response post-processing
When CalibrationTracker confidence is below threshold, append a subtle indicator:
- High confidence (>0.8): no indicator
- Medium confidence (0.5-0.8): "[confidence: moderate]"
- Low confidence (<0.5): "[confidence: low — please verify]"

### Tests
- Add to `verify_response.rs` test module:
  - Test: short response with file path → still verified
  - Test: greeting with no claims → not verified (ok)
  - Test: response mentioning `src/foo.rs` → path checked

### Verification
- `cargo test -p hydra-native-cognitive -j 1`
- Manual: ask "where is the main function?" → file path in response is verified

---

## P4: Adaptive User Profile

**Why fifth**: Compounds over time. After 2 weeks, Hydra feels personalized.

### What to Learn

| Trait Key | How Detected | Effect |
|-----------|-------------|--------|
| `expertise_level` | Vocabulary complexity, question depth | Adjust explanation detail |
| `preferred_language` | Which file types they edit most | Default to that language |
| `verbosity_preference` | Do they say "too long" or "tell me more"? | Adjust response length |
| `test_first` | Do they ask for tests before code? | Suggest tests proactively |
| `preferred_model` | Which model gets the most "thanks"? | Default to that model |
| `active_hours` | When do they interact? | Schedule background tasks accordingly |
| `correction_patterns` | What do they correct most? | Avoid those mistakes |
| `project_role` | "I'm the frontend dev" / code patterns | Tailor to their domain |

### Implementation Files

#### 1. `hydra-native-cognitive/src/cognitive/user_model.rs` (~150 lines) — NEW
- `UserModel` struct with learned traits as `HashMap<String, LearnedTrait>`
- `LearnedTrait { value: String, confidence: f64, observations: u32 }`
- `observe_interaction(text: &str, response: &str, outcome: Outcome)` — update traits:
  - Count corrections → `verbosity_preference` if "too long" / "too short"
  - Detect expertise from vocabulary: "monomorphization" → expert; "how do I make a variable" → beginner
  - Track file types touched → `preferred_language`
  - Track hours of interaction → `active_hours`
- `get_trait(key: &str) -> Option<&LearnedTrait>`
- `system_prompt_additions() -> String` — generate prompt fragment from learned traits

#### 2. Wire into PERCEIVE — MODIFY `phase_perceive.rs` (~+5 lines)
- Load UserModel from DB at start
- Add `user_model_context` to PerceiveResult

#### 3. Wire into THINK prompt — MODIFY `phase_think_prompt.rs` (~+10 lines)
- Append `user_model.system_prompt_additions()` to system prompt
- Example: "The user is an expert Rust developer who prefers concise responses."

#### 4. Wire into LEARN — MODIFY `phase_learn_intelligence.rs` (~+10 lines)
- After recording outcome: `user_model.observe_interaction(text, response, outcome)`
- Periodically save to DB via `store_intelligence.rs`

### Tests
- `hydra-native-cognitive/src/cognitive/user_model_tests.rs` (~80 lines)
  - Test expertise detection from vocabulary
  - Test verbosity learning from corrections
  - Test system prompt generation

### Verification
- After 10 interactions with expert vocabulary → Hydra should describe user as "experienced developer"
- After user says "too long" twice → responses should get shorter

---

## Implementation Order

```
P0 (Persistence)        ←── Foundation, everything depends on this
 │
 ├── P3 (Verification)  ←── Quick win, small change, big trust impact
 │
 ├── P4 (User Profile)  ←── Small, starts learning immediately
 │
 ├── P1 (Code Index)    ←── Medium effort, transforms code queries
 │
 └── P2 (File Watcher)  ←── Largest effort, biggest "wow" factor
```

### File Budget

| Priority | New Files | Modified Files | New Lines | Phase |
|----------|-----------|----------------|-----------|-------|
| P0 | 2 | 3 | ~230 | Persistence |
| P1 | 5 | 3 | ~780 | Code Index |
| P2 | 3 | 3 | ~530 | File Watcher |
| P3 | 0 | 1 | ~40 | Verification |
| P4 | 1 | 3 | ~250 | User Profile |
| **Total** | **11** | **13** | **~1,830** | — |

All files under 400 lines. Tests in existing `tests/suite/` pattern.

### Constraints
- All files < 400 lines (OOM guard)
- `cargo check -j 1` / `cargo test -j 1` always
- No new crate dependencies except `notify` for P2
- No tree-sitter (too heavy for Mac Mini) — regex-based extraction for P1
- New tests go in `tests/suite/main.rs` modules
- hydra-native depends ONLY on hydra-native-state + hydra-native-cognitive

---

## Success Criteria

After all 5 priorities are implemented, these scenarios should work:

1. **Memory across sessions**: "Yesterday you mentioned the auth module was slow. Since then, 2 commits optimized it."
2. **Instant code knowledge**: "What calls `run_cognitive_loop`?" → instant answer from local index, no sister delay
3. **Proactive detection**: User creates merge conflict → Hydra detects and offers to help within 2 seconds
4. **Never wrong about files**: Every `src/foo.rs` reference in responses is verified to exist
5. **Personalized responses**: After 20 interactions, response style matches user's expertise level
6. **Smart model selection**: After tracking outcomes for a week, Hydra picks the right model 90%+ of the time
7. **Self-improvement**: Hydra identifies its weak categories and adjusts without human intervention
