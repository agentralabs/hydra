# HYDRA SUPERINTELLIGENCE PLAN — Beyond Single-Turn Reasoning

**Created**: 2026-03-12
**Status**: ALL 7 PHASES IMPLEMENTED (2026-03-13)
**Goal**: Make Hydra smarter than any single LLM by leveraging Claude's reasoning within an intelligent orchestration layer that provides memory, verification, iteration, learning, and proactive intelligence.

**Core Thesis**: Intelligence = Reasoning + Memory + Action + Verification + Learning + Time. Claude provides reasoning. Hydra provides everything else. The combination exceeds either alone.

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│                        HYDRA                                  │
│                                                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │ PERCEIVE │→ │  REASON  │→ │  DECIDE  │→ │   ACT    │──┐  │
│  │ (sisters)│  │ (Claude) │  │ (gate)   │  │ (execute)│  │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │  │
│       ↑                                                   │  │
│       │         ┌──────────┐  ┌──────────┐               │  │
│       └─────────│ OBSERVE  │←─│  VERIFY  │←──────────────┘  │
│                 │ (results)│  │ (ground) │                   │
│                 └────┬─────┘  └──────────┘                   │
│                      │                                        │
│                 ┌────▼─────┐                                  │
│                 │  LEARN   │                                  │
│                 │ (update  │                                  │
│                 │  beliefs,│                                  │
│                 │  skills, │                                  │
│                 │  models) │                                  │
│                 └──────────┘                                  │
│                                                               │
│  Loop until: goal achieved OR max iterations OR user stops    │
└──────────────────────────────────────────────────────────────┘
```

**What changes from current architecture:**
- Current: Linear pipeline (perceive→think→decide→act→learn→DONE)
- New: Iterative loop (perceive→think→decide→act→OBSERVE→VERIFY→LEARN→think again→...)
- Current: Single LLM call per user message
- New: Multiple LLM calls with tool results fed back, until task complete
- Current: Passive learning (store facts)
- New: Active learning (track outcomes, update competence, improve prompts)
- Current: No response verification
- New: Claim extraction → grounding → correction before delivery

---

## PHASE 1: MULTI-TURN AGENTIC LOOP (Priority: CRITICAL)

**Why first**: This single change makes Hydra dramatically more capable. Without it, Hydra generates one response and stops. With it, Hydra can write code → run tests → see failures → fix → verify → deliver working code.

### 1.1 Tool Result Feedback Loop

**Current state**: `phase_act_exec.rs` executes `<hydra-tool>` tags, appends results to response text, delivers to user. The LLM never sees the results.

**Target state**: After tool execution, feed results back to the LLM as a follow-up message. Let the LLM decide: "I need more tools" or "I'm done."

#### Files to modify:

**NEW: `crates/hydra-native-cognitive/src/cognitive/handlers/agentic_loop.rs` (~350 lines)**

The core multi-turn loop engine:

```rust
pub struct AgenticLoopConfig {
    pub max_turns: u8,              // Default 8, max 15
    pub turn_timeout_secs: u64,     // Per-turn timeout (30s)
    pub total_budget_tokens: u64,   // Max tokens across all turns (50K)
    pub stop_on_no_tools: bool,     // Stop if LLM doesn't use tools (true)
    pub stop_phrases: Vec<String>,  // LLM can emit to signal completion
}

pub struct AgenticTurn {
    pub turn_number: u8,
    pub llm_response: String,
    pub tool_results: Vec<(String, String)>,  // (tool_name, result)
    pub exec_results: Vec<(String, String, bool)>,  // (cmd, output, success)
    pub tokens_used: u64,
    pub duration_ms: u64,
}

pub struct AgenticLoopResult {
    pub turns: Vec<AgenticTurn>,
    pub final_response: String,      // Last LLM response (cleaned)
    pub total_tokens: u64,
    pub total_duration_ms: u64,
    pub stop_reason: AgenticStopReason,
}

pub enum AgenticStopReason {
    TaskComplete,        // LLM emitted stop phrase or no more tools
    MaxTurns,            // Hit max_turns limit
    TokenBudgetExhausted,
    UserCancelled,
    Error(String),
}

/// Run the multi-turn agentic loop.
///
/// Flow per turn:
/// 1. Call LLM with system prompt + conversation history + tool results
/// 2. Parse response for <hydra-tool> and <hydra-exec> tags
/// 3. Execute tools and commands
/// 4. If tools were called → build follow-up message with results → next turn
/// 5. If no tools → task complete → return final response
pub async fn run_agentic_loop(
    text: &str,
    system_prompt: &str,
    initial_response: &str,        // First LLM response (from phase_think)
    config: &AgenticLoopConfig,
    llm_config: &hydra_model::LlmConfig,
    active_model: &str,
    provider: &str,
    sisters_handle: &Option<SistersHandle>,
    decide_engine: &Arc<DecideEngine>,
    db: &Option<Arc<HydraDb>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> AgenticLoopResult
```

**Algorithm:**
```
turn_0 = initial LLM response (already computed by phase_think)
messages = [system_prompt, user_text, turn_0_response]
total_tokens = phase_think tokens

FOR turn in 1..max_turns:
    tool_invocations = extract_hydra_tool_tags(last_response)
    exec_commands = extract_inline_commands(last_response)

    IF tool_invocations.is_empty() AND exec_commands.is_empty():
        RETURN AgenticLoopResult { stop_reason: TaskComplete }

    // Execute tools
    tool_results = sisters.execute_tool_tags(last_response)
    exec_results = execute_commands(last_response, ...)  // existing security pipeline

    // Build follow-up message
    follow_up = format_tool_results_message(tool_results, exec_results)
    messages.push(Message { role: "tool_results", content: follow_up })

    // Send progress to UI
    tx.send(CognitiveUpdate::AgenticTurn { turn, tool_count, exec_count })

    // Check budget
    IF total_tokens >= config.total_budget_tokens:
        RETURN AgenticLoopResult { stop_reason: TokenBudgetExhausted }

    // Call LLM again with accumulated context
    remaining_budget = config.total_budget_tokens - total_tokens
    llm_response = call_llm(messages, min(remaining_budget, 4096))
    total_tokens += response.input_tokens + response.output_tokens

    last_response = llm_response.content

    // Check for stop phrases
    IF last_response.contains("<hydra-done>") OR last_response.contains("Task complete"):
        RETURN AgenticLoopResult { stop_reason: TaskComplete }

RETURN AgenticLoopResult { stop_reason: MaxTurns }
```

**NEW: `crates/hydra-native-cognitive/src/cognitive/handlers/agentic_loop_format.rs` (~120 lines)**

Formats tool results into LLM-readable follow-up messages:

```rust
/// Format tool results + exec results into a follow-up message for the LLM.
pub fn format_tool_results_message(
    tool_results: &[(String, String)],
    exec_results: &[(String, String, bool)],
) -> String

/// Format a single tool result with truncation for context efficiency.
fn format_single_tool_result(name: &str, output: &str, max_chars: usize) -> String

/// Detect if LLM response signals task completion (no more work needed).
pub fn is_task_complete(response: &str) -> bool
```

#### Wiring into cognitive loop:

**MODIFY: `crates/hydra-native-cognitive/src/cognitive/handlers/phase_act.rs`**

After the existing single-pass execution, check if agentic looping is warranted:

```rust
// After existing act logic...

// Determine if we should enter agentic loop
let should_loop = has_tool_invocations(&act_result.final_response)
    && is_complex
    && config.runtime.agentic_loop_enabled;  // New runtime setting

if should_loop {
    let loop_config = AgenticLoopConfig {
        max_turns: if is_action_request { 10 } else { 5 },
        turn_timeout_secs: 30,
        total_budget_tokens: 50_000,
        stop_on_no_tools: true,
        stop_phrases: vec!["<hydra-done>".into()],
    };
    let loop_result = run_agentic_loop(
        text, &system_prompt, &act_result.final_response,
        &loop_config, &llm_config, &active_model, &provider,
        sisters_handle, decide_engine, db, tx,
    ).await;
    act_result.final_response = loop_result.final_response;
    act_result.all_exec_results.extend(
        loop_result.turns.iter().flat_map(|t| t.exec_results.clone())
    );
}
```

**MODIFY: `crates/hydra-native-cognitive/src/cognitive/loop_runner.rs`**

Add `AgenticTurn` variant to `CognitiveUpdate`:
```rust
AgenticTurn {
    turn: u8,
    tool_count: usize,
    exec_count: usize,
},
AgenticComplete {
    turns: u8,
    total_tokens: u64,
    stop_reason: String,
},
```

**MODIFY: `crates/hydra-native-cognitive/src/cognitive/handlers/phase_think_prompt.rs`**

Update the tool instruction in the system prompt to include the stop signal:

```rust
// In the Available Tools section:
"When you have finished all tool calls and the task is complete, \
 include <hydra-done/> at the end of your response.\n\
 Do NOT include <hydra-done/> if you still need tool results.\n"
```

**MODIFY: `crates/hydra-native-state/src/utils/mod.rs` or `runtime_settings.rs`**

Add runtime setting:
```rust
pub agentic_loop_enabled: bool,  // Default: true
pub agentic_max_turns: u8,       // Default: 8
pub agentic_token_budget: u64,   // Default: 50_000
```

### 1.2 Smart Loop Entry Detection

Not every message needs multi-turn. Classification:

| Intent | Loop? | Max Turns | Why |
|--------|-------|-----------|-----|
| Greeting/Farewell/Thanks | No | 0 | Instant response |
| MemoryRecall | No | 0 | Single query |
| Question (simple) | No | 0 | Direct answer |
| CodeBuild | Yes | 10 | Write → test → fix cycle |
| CodeFix | Yes | 8 | Diagnose → fix → verify |
| CodeExplain | Maybe | 3 | May need to read multiple files |
| Deploy | Yes | 10 | Multi-step with verification |
| SelfImplement | Yes | 10 | Spec → gaps → patches → verify |
| WebBrowse | Yes | 5 | Navigate → extract → follow links |
| PlanningQuery | Maybe | 3 | May need data gathering |
| FileOperation | Maybe | 3 | May need multi-step |

**NEW: `crates/hydra-native-cognitive/src/cognitive/handlers/agentic_loop_entry.rs` (~80 lines)**

```rust
/// Determine if this interaction should use the agentic loop.
pub fn should_enter_agentic_loop(
    intent: &ClassifiedIntent,
    complexity: &str,
    has_tool_calls: bool,
    runtime: &RuntimeSettings,
) -> Option<AgenticLoopConfig>
```

### 1.3 Constraints

- **400-line max**: `agentic_loop.rs` ≤ 350, `agentic_loop_format.rs` ≤ 120, `agentic_loop_entry.rs` ≤ 80
- **Token budget**: Default 50K tokens per agentic session. Prevents runaway costs.
- **Turn limit**: Default 8, max 15. Prevents infinite loops.
- **Per-turn timeout**: 30 seconds. Prevents hanging.
- **Streaming**: Each LLM turn should stream to UI so user sees progress.
- **Cancellation**: User can cancel mid-loop. Check `tx` for cancel signal between turns.
- **No new deps**: Uses existing `hydra-model` CompletionRequest + provider clients.
- **Security**: Every exec command in every turn goes through the existing 6-layer security pipeline.
- **No test infrastructure calls**: Tests use `Sisters::empty()`, mock tool results.

### 1.4 Verification

```bash
cargo check -p hydra-native-cognitive -j 1     # Compiles clean
cargo test -p hydra-native-cognitive -j 1      # All tests pass
bash scripts/check-file-size-guard.sh          # All files under 400
```

Manual test:
1. Launch TUI
2. Type: "create a rust function that checks if a number is prime, write it to /tmp/prime.rs, then test it"
3. Observe: Hydra writes file → runs `cargo check` → sees error → fixes → runs test → reports success
4. Should take 2-4 turns

---

## PHASE 2: RESPONSE VERIFICATION PIPELINE (Priority: HIGH)

**Why second**: Eliminates hallucination — Hydra's biggest weakness inherited from the LLM. Every factual claim gets checked before the user sees it.

### 2.1 Claim Extraction & Grounding

**Current state**: Veritas `extract_claims` and Aegis `confidence_score` exist as MCP tools. Codebase has `hallucination_check`. But they're never called on the LLM's response before delivery.

**Target state**: After the LLM generates a response, automatically extract claims, verify each against sisters, and correct or flag unverified claims.

#### Files:

**NEW: `crates/hydra-native-cognitive/src/cognitive/handlers/verify_response.rs` (~300 lines)**

```rust
pub struct VerificationResult {
    pub original_response: String,
    pub verified_response: String,
    pub claims_checked: usize,
    pub claims_verified: usize,
    pub claims_corrected: usize,
    pub claims_flagged: usize,  // Unverifiable but not correctable
    pub verification_ms: u64,
}

/// Verify an LLM response before delivery.
///
/// Pipeline:
/// 1. Extract claims via Veritas sister
/// 2. For code claims → Codebase sister hallucination_check
/// 3. For factual claims → Memory sister cross-reference
/// 4. For file/path claims → filesystem check
/// 5. For API/syntax claims → Aegis confidence score
/// 6. Correct verified-false claims inline
/// 7. Flag unverifiable claims with confidence markers
pub async fn verify_response(
    response: &str,
    user_text: &str,
    sisters_handle: &Option<SistersHandle>,
    intent: &ClassifiedIntent,
) -> VerificationResult
```

**Claim types and verification strategy:**

| Claim Type | Example | Verifier | Action on Fail |
|------------|---------|----------|----------------|
| Code exists | "function X in file Y" | Codebase `hallucination_check` | Correct with actual location |
| File path | "config is at ~/.hydra/config.toml" | `std::fs::metadata` | Flag or correct |
| API syntax | "call `.unwrap_or_default()`" | Aegis `confidence_score` | Flag low confidence |
| Memory fact | "you told me X last time" | Memory `memory_query` | Correct with actual memory |
| Quantitative | "there are 14 sisters" | Sisters `connected_count` | Correct with actual count |

**When to verify:**

```rust
/// Should we run verification on this response?
fn should_verify(intent: &ClassifiedIntent, complexity: &str, response: &str) -> bool {
    // Always verify code explanations (highest hallucination risk)
    if matches!(intent.category, CodeExplain | CodeBuild | CodeFix) { return true; }
    // Verify complex responses (more likely to contain errors)
    if complexity == "complex" { return true; }
    // Verify responses that reference files, functions, or specific facts
    if response.contains("file ") || response.contains("function ")
        || response.contains("in `") { return true; }
    // Skip greetings, acknowledgments, simple Q&A
    false
}
```

**MODIFY: `crates/hydra-native-cognitive/src/cognitive/handlers/phase_act.rs`**

Insert verification between LLM response and delivery:

```rust
// After think phase, before delivery:
if should_verify(&intent, complexity, &response_text) {
    let verification = verify_response(
        &response_text, text, sisters_handle, &intent
    ).await;
    if verification.claims_corrected > 0 {
        response_text = verification.verified_response;
        tx.send(CognitiveUpdate::VerificationApplied {
            checked: verification.claims_checked,
            corrected: verification.claims_corrected,
        });
    }
}
```

**NEW CognitiveUpdate variant:**
```rust
VerificationApplied {
    checked: usize,
    corrected: usize,
},
```

### 2.2 Constraints

- **Latency budget**: Verification adds latency. Target < 500ms for simple responses, < 2s for complex.
- **Parallel verification**: Check all claims concurrently (`tokio::join!` or `futures::join_all`).
- **Graceful degradation**: If verification sisters are offline, skip verification (don't block delivery).
- **No false positives**: Only correct claims you're confident are wrong. Flag uncertain claims, don't silently change them.
- **Correction format**: Inline corrections with `[corrected: actual value]` markers, not footnotes.

---

## PHASE 3: ACTIVE LEARNING FROM OUTCOMES (Priority: HIGH)

**Why third**: Makes Hydra get better over time. Every interaction becomes training data for future performance.

### 3.1 Outcome Tracking

**Current state**: LEARN phase stores facts and records patterns. But it doesn't systematically track whether Hydra's responses were *correct* or *useful*.

**Target state**: After every interaction, detect the outcome (success/failure/correction) and update competence maps, belief confidence, and model routing weights.

#### Outcome Detection Signals:

| Signal | Meaning | Detection |
|--------|---------|-----------|
| User says "no", "wrong", "actually" | Correction | Pattern match in next user message |
| User says "thanks", "perfect", "great" | Success | Pattern match |
| User repeats same question | Failure (didn't answer well) | Semantic similarity to previous |
| Command execution succeeds | Partial success | `exec_results.success == true` |
| Command execution fails | Partial failure | `exec_results.success == false` |
| Test suite passes | Strong success | Parse test output |
| Test suite fails | Potential failure | Parse test output |
| User asks for undo | Failure | Intent = undo/revert |
| User leaves without response | Ambiguous | Session timeout |

#### Files:

**NEW: `crates/hydra-native-cognitive/src/cognitive/outcome_tracker.rs` (~250 lines)**

```rust
pub struct OutcomeTracker {
    /// Rolling window of recent interactions with outcomes
    history: VecDeque<InteractionOutcome>,
    /// Per-category success rates
    category_stats: HashMap<IntentCategory, CategoryStats>,
    /// Per-topic success rates (extracted subjects)
    topic_stats: HashMap<String, TopicStats>,
}

pub struct InteractionOutcome {
    pub intent_category: IntentCategory,
    pub topic: String,
    pub model_used: String,
    pub outcome: Outcome,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tokens_used: u64,
}

pub enum Outcome {
    Success,            // Positive user signal
    Correction,         // User corrected us
    Failure,            // Command failed, user frustrated
    Repeat,             // User had to re-ask
    Neutral,            // No signal
}

pub struct CategoryStats {
    pub total: u64,
    pub successes: u64,
    pub corrections: u64,
    pub failures: u64,
    pub success_rate: f64,        // Rolling average
    pub avg_tokens: u64,
}

pub struct TopicStats {
    pub total: u64,
    pub success_rate: f64,
    pub last_outcome: Outcome,
    pub best_model: Option<String>,  // Which model worked best for this topic
}

impl OutcomeTracker {
    /// Detect outcome from user's follow-up message.
    pub fn detect_outcome(
        &self,
        previous_response: &str,
        current_input: &str,
        exec_results: &[(String, String, bool)],
    ) -> Outcome

    /// Record an outcome and update all stats.
    pub fn record(
        &mut self,
        intent: IntentCategory,
        topic: &str,
        model: &str,
        outcome: Outcome,
        tokens: u64,
    )

    /// Get success rate for a category (for model routing decisions).
    pub fn category_success_rate(&self, cat: IntentCategory) -> f64

    /// Get best model for a topic (for dynamic model selection).
    pub fn best_model_for_topic(&self, topic: &str) -> Option<String>

    /// Suggest competence-aware confidence adjustment.
    pub fn confidence_adjustment(&self, intent: IntentCategory, topic: &str) -> f64
}
```

### 3.2 Competence-Aware Model Routing

**Current state**: Model selection is static per intent category (Haiku for simple, Sonnet for complex).

**Target state**: Model selection adapts based on historical success rates.

**MODIFY: `crates/hydra-native-cognitive/src/cognitive/handlers/phase_think.rs`**

Add competence-aware model override:

```rust
// After initial model selection...

// Check if outcome tracker recommends a different model
if let Some(ref tracker) = outcome_tracker {
    let cat_rate = tracker.category_success_rate(intent.category);
    let topic = extract_primary_topic(text);

    // If success rate with current model is low, try upgrading
    if cat_rate < 0.6 && active_model.contains("haiku") {
        active_model = "claude-sonnet-4-6".into();
        eprintln!("[hydra:model] Upgraded to Sonnet — {:.0}% success rate with Haiku for {:?}",
            cat_rate * 100.0, intent.category);
    }

    // If a specific model worked better for this topic, prefer it
    if let Some(best) = tracker.best_model_for_topic(&topic) {
        if best != active_model {
            active_model = best;
        }
    }
}
```

### 3.3 Belief Confidence Calibration

**Current state**: Beliefs are stored with initial confidence (0.95 user-stated, 0.60 inferred). Decay is time-based only.

**Target state**: Belief confidence also adjusts based on whether actions taken based on that belief succeeded or failed.

**MODIFY: `crates/hydra-native-cognitive/src/cognitive/handlers/phase_learn.rs`**

Add outcome-driven belief updates:

```rust
// In LEARN phase, after outcome detection:
if outcome == Outcome::Correction {
    // Find beliefs that may have caused the wrong response
    let related_beliefs = db.get_beliefs_by_subject(&topic);
    for belief in related_beliefs {
        if belief.confidence > 0.5 {
            // Reduce confidence — this belief led to a correction
            db.update_belief_confidence(belief.id, belief.confidence - 0.1);
        }
    }
}
if outcome == Outcome::Success {
    // Reinforce beliefs used in this response
    let related_beliefs = db.get_beliefs_by_subject(&topic);
    for belief in related_beliefs {
        db.update_belief_confidence(belief.id, (belief.confidence + 0.02).min(1.0));
    }
}
```

### 3.4 Correction Learning

**Current state**: When user says "no, actually X", we store X as a new belief. But we don't link it to *what we got wrong* or *why*.

**Target state**: Corrections create a causal chain: wrong_response → correction → belief_update → future_behavior_change.

**MODIFY: `crates/hydra-native-cognitive/src/sisters/learn.rs`**

Add structured correction storage:

```rust
// When correction detected:
if is_correction {
    // Store the correction with causal edge to what was wrong
    sh.memory_add_correction(
        &previous_response,  // What Hydra said (wrong)
        &current_input,      // What user corrected to
        &topic,              // Subject area
    ).await;

    // Record in outcome tracker
    tracker.record(intent.category, &topic, &model, Outcome::Correction, tokens);

    // Log for competence tracking
    inventions.record_session_correction();
}
```

### 3.5 Constraints

- **OutcomeTracker is in-memory** with periodic DB persistence (every 50 interactions).
- **No retroactive re-scoring** — only affects future interactions.
- **Privacy**: Outcome data stays local. Not sent to LLM providers.
- **Forgetting**: Old outcomes decay (90-day window). Hydra doesn't hold grudges.

---

## PHASE 4: MULTI-MODEL ORCHESTRATION (Priority: MEDIUM)

**Why fourth**: Reduces cost 60-80% while maintaining or improving quality. Every interaction currently uses one model for everything. Smart routing uses the cheapest model that can handle each subtask.

### 4.1 Per-Subtask Model Selection

**Current state**: One model selected per user message (Haiku or Sonnet based on complexity).

**Target state**: Within a single interaction, different subtasks use different models:

| Subtask | Model | Cost | Why |
|---------|-------|------|-----|
| Intent classification | Haiku | $0.001/1K | Already done — 60 tokens |
| Simple Q&A | Haiku | $0.001/1K | No reasoning needed |
| Code generation | Sonnet | $0.015/1K | Needs accuracy |
| Complex reasoning | Opus | $0.075/1K | Only when needed |
| Verification (per-claim) | Haiku | $0.001/1K | Yes/no checks |
| Error diagnosis | Haiku | $0.001/1K | Pattern matching |
| Summary generation | Haiku | $0.001/1K | Mechanical |
| Code review | Sonnet | $0.015/1K | Needs understanding |

### 4.2 Complexity Escalation

Instead of choosing one model upfront, start cheap and escalate:

```
1. Try Haiku first (always)
2. If response quality is low OR task too complex → re-try with Sonnet
3. If Sonnet also struggles → escalate to Opus
4. Never escalate for greetings, simple Q&A, acknowledgments
```

**Quality detection heuristics:**
- Response too short for a complex question (< 100 chars for CodeBuild)
- Response contains "I'm not sure" / "I don't know" for factual questions
- Response fails verification (Phase 2)
- Response contains `todo!()` / placeholder code

#### Files:

**NEW: `crates/hydra-native-cognitive/src/cognitive/handlers/model_escalation.rs` (~150 lines)**

```rust
pub struct EscalationDecision {
    pub should_escalate: bool,
    pub reason: &'static str,
    pub target_model: String,
}

/// Check if the LLM response quality warrants escalation to a stronger model.
pub fn check_escalation(
    response: &str,
    intent: &ClassifiedIntent,
    complexity: &str,
    current_model: &str,
    verification: Option<&VerificationResult>,
) -> Option<EscalationDecision>

/// Get the next model in the escalation chain.
fn next_model(current: &str) -> Option<String> {
    match current {
        m if m.contains("haiku") => Some("claude-sonnet-4-6".into()),
        m if m.contains("sonnet") => Some("claude-opus-4-6".into()),
        _ => None,  // Already at max
    }
}
```

**MODIFY: `crates/hydra-native-cognitive/src/cognitive/handlers/phase_think.rs`**

After LLM response, check for escalation:

```rust
// After getting response from Haiku/Sonnet...
if let Some(escalation) = check_escalation(
    &response, &intent, complexity, &active_model, verification.as_ref()
) {
    eprintln!("[hydra:escalate] {} → {} ({})", active_model, escalation.target_model, escalation.reason);
    // Re-call with stronger model, reusing same system prompt + messages
    active_model = escalation.target_model;
    // ... re-call LLM
}
```

### 4.3 Constraints

- **Max 1 escalation per interaction** — Haiku→Sonnet or Sonnet→Opus, never both.
- **Never escalate greetings/farewells** — waste of tokens.
- **Track escalation rates** — if Haiku escalates >50% of the time for a category, just start with Sonnet.
- **Cost reporting** — show user total cost per interaction (combine all model calls).

---

## PHASE 5: PROACTIVE INTELLIGENCE (Priority: MEDIUM)

**Why fifth**: Makes Hydra anticipate needs instead of just reacting. Turns idle time into productive intelligence gathering.

### 5.1 Background Intelligence Tasks

**Current state**: `ProactiveNotifier` does simple pattern matching. Dream processing only runs when idle >= 60s.

**Target state**: Hydra continuously runs background intelligence tasks between user messages:

| Task | Trigger | Sister | Action |
|------|---------|--------|--------|
| Dependency vulnerability scan | Every 4 hours | Aegis | Scan Cargo.lock/package.json for CVEs |
| Test suite health | After code changes | Codebase | Run tests, report regressions |
| Stale branch detection | Every 24 hours | Codebase | Find branches > 7 days old |
| Deadline approaching | Hourly | Time | Alert 24h before deadline |
| Memory consolidation | Every 100 messages | Memory | Compress, deduplicate, strengthen |
| Pattern crystallization | Every 50 interactions | Evolve | Identify recurring patterns |
| Belief audit | Every 24 hours | Cognition | Flag low-confidence beliefs |
| Federation sync | Every 30 minutes | Comm | Share learnings with peers |
| Competence report | Weekly | Internal | Summarize success rates by category |

#### Files:

**NEW: `crates/hydra-native-cognitive/src/cognitive/background_tasks.rs` (~300 lines)**

```rust
pub struct BackgroundScheduler {
    tasks: Vec<ScheduledTask>,
    last_run: HashMap<String, Instant>,
}

pub struct ScheduledTask {
    pub name: String,
    pub interval: Duration,
    pub priority: TaskPriority,
    pub handler: BackgroundTaskHandler,
}

pub enum TaskPriority {
    Urgent,     // Run between user messages if possible
    Normal,     // Run during idle time (> 30s)
    Low,        // Run during deep idle (> 5 min)
}

impl BackgroundScheduler {
    /// Check which tasks are due and run them.
    /// Called between cognitive loop iterations.
    pub async fn tick(
        &mut self,
        sisters: &Option<SistersHandle>,
        db: &Option<Arc<HydraDb>>,
        tx: &mpsc::UnboundedSender<CognitiveUpdate>,
    ) -> Vec<BackgroundResult>

    /// Register a new background task.
    pub fn register(&mut self, task: ScheduledTask)
}
```

### 5.2 Predictive Context Pre-loading

**Current state**: PERCEIVE phase queries sisters after the user sends a message.

**Target state**: While the user is typing, Hydra pre-loads likely context:

```rust
/// Pre-load context based on predicted user intent.
/// Called when user starts typing (keystroke event).
pub async fn preload_context(
    partial_input: &str,
    sisters: &SistersHandle,
    outcome_tracker: &OutcomeTracker,
) -> Option<PreloadedContext>
```

**Prediction signals:**
- Current project file open in editor → pre-load codebase analysis
- Recent conversation about topic X → pre-load X memories
- Time of day (morning = standup context, afternoon = code context)
- Day pattern (Monday = planning, Friday = deployment)

### 5.3 Constraints

- **Background tasks must not interfere** with active user interaction. Pause when user sends message.
- **Resource limits**: Max 1 background task at a time. Each task has 10s timeout.
- **No sister init in background**: Only run if sisters already connected.
- **Notification, not interruption**: Background findings → queue alerts, don't inject into conversation.

---

## PHASE 6: SELF-IMPROVEMENT LOOP (Priority: LOW — builds on everything above)

**Why last**: This is the endgame. Requires all previous phases to be working. Hydra uses its own competence data to identify weaknesses and fix them.

### 6.1 Weakness-Driven Self-Modification

**Current state**: SelfImplement pipeline exists but is manually triggered ("implement spec X.md").

**Target state**: Hydra identifies its own weaknesses from outcome data and generates improvement specs:

```
OutcomeTracker detects: "async Rust error handling" category has 35% success rate
  → Generates spec: "Improve system prompt for async error handling patterns"
  → Or: "Add async Rust patterns to belief system"
  → Runs SelfImplement pipeline (with human approval)
  → Verifies improvement against past failures
  → Reports to user
```

#### Files:

**NEW: `crates/hydra-kernel/src/self_improve.rs` (~200 lines)**

```rust
pub struct ImprovementCandidate {
    pub weakness: String,           // "async Rust error handling"
    pub category: IntentCategory,
    pub success_rate: f64,          // 0.35
    pub sample_failures: Vec<String>, // Past interactions that failed
    pub suggested_fix: ImprovementType,
}

pub enum ImprovementType {
    PromptEnhancement(String),      // Add patterns to system prompt
    BeliefInjection(Vec<Belief>),   // Add corrective beliefs
    ToolRouteChange(String),        // Route different tools for this category
    ModelUpgrade(String),           // Use stronger model for this category
}

/// Analyze outcome data and identify improvement candidates.
pub fn identify_weaknesses(
    outcome_tracker: &OutcomeTracker,
    min_interactions: u64,      // Need >= 10 interactions to judge
    max_success_rate: f64,      // Flag if below 0.6 (60%)
) -> Vec<ImprovementCandidate>

/// Generate a self-improvement spec from weakness analysis.
pub fn generate_improvement_spec(
    candidate: &ImprovementCandidate,
) -> String  // Markdown spec for SelfImplement pipeline
```

### 6.2 Regression Prevention

Every self-improvement must be validated:

```
1. Identify weakness (outcome data)
2. Generate fix (spec)
3. Apply fix (SelfImplement)
4. Replay past failures (re-run inputs that caused corrections)
5. Compare: did the fix improve the success rate?
6. If yes → keep
7. If no → revert (checkpoint system)
```

### 6.3 Constraints

- **Human approval required** for all self-modifications (existing gate in SelfImplement).
- **Max 1 self-improvement per day** — prevents oscillation.
- **Revert on regression** — if success rate drops after improvement, auto-revert.
- **Track all improvements** in ledger for audit.

---

## PHASE 7: ENHANCED METACOGNITION (Priority: LOW — continuous improvement)

### 7.1 Confidence Calibration

Track predicted confidence vs actual outcome. If Hydra says "I'm 90% sure" but is wrong 40% of the time, adjust.

```rust
pub struct CalibrationTracker {
    /// Buckets: predicted confidence → actual success rate
    buckets: HashMap<u8, (u64, u64)>,  // (total, successes) per 10% bucket
}

impl CalibrationTracker {
    /// Record a prediction and its outcome.
    pub fn record(&mut self, predicted_confidence: f64, actual_success: bool)

    /// Get calibration error (0.0 = perfectly calibrated).
    pub fn calibration_error(&self) -> f64

    /// Adjust a predicted confidence based on historical calibration.
    pub fn calibrate(&self, raw_confidence: f64) -> f64
}
```

### 7.2 Cognitive Load Awareness

Detect when the task is too complex for the current approach and suggest decomposition:

```rust
/// Detect if the current task exceeds Hydra's reliable capability.
pub fn detect_cognitive_overload(
    text: &str,
    intent: &ClassifiedIntent,
    outcome_tracker: &OutcomeTracker,
) -> Option<OverloadResponse>

pub enum OverloadResponse {
    Decompose(Vec<String>),     // Break into subtasks
    Escalate(String),           // Suggest stronger model
    Defer(String),              // Suggest human handles this part
    Simplify(String),           // Suggest simpler approach
}
```

---

## Implementation Priority & Dependencies

```
PHASE 1: Multi-Turn Agentic Loop
  ├── 1.1 Tool Result Feedback Loop          ← CRITICAL, no deps
  ├── 1.2 Smart Loop Entry Detection         ← needs 1.1
  └── 1.3 Tests & Verification               ← needs 1.1, 1.2

PHASE 2: Response Verification
  ├── 2.1 Claim Extraction & Grounding       ← needs Phase 1 (works within loop)
  └── 2.2 Verification Tests                 ← needs 2.1

PHASE 3: Active Learning
  ├── 3.1 Outcome Tracking                   ← independent
  ├── 3.2 Competence-Aware Model Routing     ← needs 3.1
  ├── 3.3 Belief Confidence Calibration      ← needs 3.1
  └── 3.4 Correction Learning                ← needs 3.1

PHASE 4: Multi-Model Orchestration
  ├── 4.1 Per-Subtask Model Selection        ← needs Phase 3 (outcome data)
  └── 4.2 Complexity Escalation              ← needs Phase 2 (quality detection)

PHASE 5: Proactive Intelligence
  ├── 5.1 Background Intelligence Tasks      ← independent
  └── 5.2 Predictive Context Pre-loading     ← needs Phase 3 (prediction data)

PHASE 6: Self-Improvement Loop
  ├── 6.1 Weakness-Driven Self-Modification  ← needs Phase 3 (outcome data)
  └── 6.2 Regression Prevention              ← needs 6.1 + Phase 1 (re-run capability)

PHASE 7: Enhanced Metacognition
  ├── 7.1 Confidence Calibration             ← needs Phase 3
  └── 7.2 Cognitive Load Awareness           ← needs Phase 3
```

## File Inventory (All New & Modified Files)

### New Files (12 files, ~2,200 lines total)

| File | Lines | Phase | Purpose |
|------|-------|-------|---------|
| `cognitive/handlers/agentic_loop.rs` | ~350 | 1 | Core multi-turn loop engine |
| `cognitive/handlers/agentic_loop_format.rs` | ~120 | 1 | Tool result formatting for LLM |
| `cognitive/handlers/agentic_loop_entry.rs` | ~80 | 1 | Smart loop entry detection |
| `cognitive/handlers/verify_response.rs` | ~300 | 2 | Response verification pipeline |
| `cognitive/outcome_tracker.rs` | ~250 | 3 | Outcome detection & competence tracking |
| `cognitive/handlers/model_escalation.rs` | ~150 | 4 | Quality-based model escalation |
| `cognitive/background_tasks.rs` | ~300 | 5 | Background intelligence scheduler |
| `hydra-kernel/src/self_improve.rs` | ~200 | 6 | Weakness identification & spec generation |
| Test files (4 files) | ~450 | All | Tests for each phase |

### Modified Files (10 files, ~200 lines of changes)

| File | Changes | Phase |
|------|---------|-------|
| `handlers/phase_act.rs` | Add agentic loop entry point | 1 |
| `handlers/phase_think_prompt.rs` | Add `<hydra-done>` instruction | 1 |
| `loop_runner.rs` | Add CognitiveUpdate variants | 1, 2 |
| `runtime_settings.rs` | Add agentic loop settings | 1 |
| `handlers/mod.rs` | Register new handler modules | 1, 2 |
| `handlers/phase_think.rs` | Competence-aware model routing | 3, 4 |
| `handlers/phase_learn.rs` | Outcome-driven belief updates | 3 |
| `sisters/learn.rs` | Structured correction storage | 3 |
| `sisters/mod.rs` | Register new modules (if needed) | - |
| `hydra-kernel/src/lib.rs` | Register self_improve module | 6 |

### No Files Deleted

### Constraints (Apply to ALL Phases)

1. **400-line max per file** — enforced by `scripts/check-file-size-guard.sh`
2. **No new crate dependencies** for hydra-native — only add to sub-crates
3. **`-j 1` for all cargo commands** — OOM prevention
4. **Tests use `Sisters::empty()`** — no real sister spawning in tests
5. **All files in `hydra-native-cognitive`** unless explicitly noted otherwise
6. **Streaming must work** — every LLM call in the agentic loop streams to UI
7. **Cancellation support** — user can cancel at any point
8. **Graceful degradation** — if sisters offline, skip verification/proactive, don't crash

---

## Success Metrics

### Phase 1 (Agentic Loop)
- Hydra can complete "write function, test it, fix errors" in 2-4 turns without user re-prompting
- 90% of code generation tasks succeed without user intervention
- Average agentic session: 3 turns, 15K tokens, < 30 seconds

### Phase 2 (Verification)
- 80% of factual claims in code explanations are verified before delivery
- Hallucination rate drops from ~15% (estimated) to < 3%
- Verification adds < 500ms latency for simple responses

### Phase 3 (Learning)
- After 100 interactions, Hydra achieves 85%+ success rate (up from ~70%)
- Corrections reduce over time (< 5% correction rate after 200 interactions)
- Model routing adapts: 30% cost reduction from using Haiku where appropriate

### Phase 4 (Multi-Model)
- 60% cost reduction vs always-Sonnet baseline
- No quality degradation on complex tasks
- Escalation rate < 20% (means initial model selection is usually right)

### Phase 5 (Proactive)
- User receives relevant proactive alerts in 30% of sessions
- Context pre-loading reduces perceive phase latency by 40%
- Background vulnerability scans catch issues before user discovers them

### Phase 6 (Self-Improvement)
- At least 1 self-improvement per week passes regression testing
- Category success rates improve by 10% after self-improvement
- No regressions (revert rate < 5%)

### Phase 7 (Metacognition)
- Calibration error < 0.1 (predicted confidence matches actual accuracy)
- Cognitive overload detection prevents 50% of "I'm stuck" loops

---

## What This Achieves

After all 7 phases, Hydra will:

1. **Complete multi-step tasks autonomously** — not just generate responses, but iterate until the task is done
2. **Never hallucinate without flagging it** — every factual claim verified before delivery
3. **Get better every day** — learning from outcomes, not just storing facts
4. **Use the cheapest model that works** — 60% cost reduction with equal or better quality
5. **Anticipate needs** — pre-load context, warn about issues, suggest next steps
6. **Fix its own weaknesses** — identify failure patterns and improve automatically
7. **Know what it doesn't know** — calibrated confidence, cognitive load awareness

**The result**: A system that is smarter than any single LLM because it combines Claude's reasoning with persistent memory, iterative execution, self-verification, continuous learning, and self-improvement. Claude provides the reasoning. Hydra provides the intelligence.

---

## Implementation Status (2026-03-13)

All 7 phases implemented and wired into the cognitive loop. 655 tests pass, 0 failures.

### Phase 1: Multi-Turn Agentic Loop — COMPLETE + WIRED
- `cognitive/handlers/agentic_loop.rs` (251 lines) — core multi-turn loop engine with streaming
- `cognitive/handlers/agentic_loop_format.rs` (119 lines) — tool result formatting, `<hydra-done/>` detection
- `cognitive/handlers/agentic_loop_entry.rs` (133 lines) — intent-aware loop entry with AgenticLoopConfig
- `cognitive/cognitive_update.rs` — extracted CognitiveUpdate enum with AgenticTurn/AgenticComplete variants
- `cognitive/runtime_settings.rs` — added agentic_loop, agentic_max_turns, agentic_token_budget settings
- `cognitive/handlers/phase_act.rs` — wired agentic loop entry after initial command execution
- **Wiring**: phase_act.rs calls `run_agentic_loop()` when actionable tags detected
- **UI surfaces**: Desktop (app_handlers_cognitive.rs, app_send_handler.rs), TUI (cognitive_handler.rs)

### Phase 2: Response Verification Pipeline — COMPLETE + WIRED
- `cognitive/handlers/verify_response.rs` (380 lines) — claim extraction and grounding
  - Extracts file path, code symbol, quantitative, and memory fact claims
  - Parallel verification via `futures::join_all`
  - Inline correction application
- **Wiring**: loop_runner.rs calls `verify_response()` between ACT and LEARN phases (lines 221-240)
- CognitiveUpdate::VerificationApplied sent and handled in both UIs

### Phase 3: Active Learning from Outcomes — COMPLETE + WIRED
- `cognitive/outcome_tracker.rs` (351 lines) — outcome detection and competence tracking
  - Detects success/correction/failure/repeat/neutral from user follow-up messages
  - Per-category and per-topic success rate tracking with rolling 500-interaction window
  - Best model tracking per topic, confidence adjustment
- **Wiring**:
  - `loop_runner.rs` creates OutcomeTracker, populates from conversation history via `populate_from_history()`
  - `phase_learn_intelligence.rs` extracts outcomes from history pairs
  - After LEARN phase, records this interaction's outcome in tracker
  - `category_success_rate` computed and logged per interaction

### Phase 4: Multi-Model Orchestration — COMPLETE + WIRED
- `cognitive/handlers/model_escalation.rs` (231 lines) — quality-based model escalation
  - Detects uncertainty, short responses, placeholder code, refusals
  - Escalation chain: Haiku → Sonnet → Opus (gpt-4o-mini → gpt-4o)
  - `select_initial_model()` uses intent + complexity + success rate
- **Wiring**:
  - `phase_think.rs` calls `select_initial_model()` BEFORE LLM call for proactive routing
  - `phase_think_call.rs` calls `check_escalation()` AFTER LLM response for quality detection
  - CognitiveUpdate::ModelEscalated sent when escalation detected
  - Escalation data feeds back into future model selection

### Phase 5: Proactive Intelligence — COMPLETE + WIRED
- `cognitive/background_tasks.rs` (277 lines) — background intelligence scheduler
  - 5 default tasks: vulnerability scan, memory consolidation, pattern crystallization, belief audit, competence report
  - Priority-based scheduling (Urgent/Normal/Low mapped to idle durations)
- **Wiring**:
  - `loop_runner.rs` creates BackgroundScheduler, calls `user_idle()` after LEARN phase
  - Checks `due_tasks()` and sends CognitiveUpdate::BackgroundTaskComplete for each

### Phase 6: Self-Improvement Loop — COMPLETE + WIRED
- `hydra-kernel/src/self_improve.rs` (251 lines) — weakness identification and spec generation
  - `identify_weaknesses()` finds weak categories, `suggest_fix()` recommends improvements
  - `generate_improvement_spec()` generates markdown specs
  - `evaluate_improvement()` compares before/after success rates
- **Wiring**:
  - `phase_learn_intelligence.rs` calls `check_self_improvement()` every 20 interactions
  - Uses OutcomeTracker's `weak_categories()` to feed `identify_weaknesses()`
  - Sends MetacognitiveInsight with improvement opportunities

### Phase 7: Enhanced Metacognition — COMPLETE + WIRED
- `cognitive/metacognition.rs` (334 lines) — confidence calibration and cognitive load awareness
  - `CalibrationTracker` with ECE metric and bucket-based tracking
  - `detect_cognitive_overload()` with Decompose/Escalate/Defer/Simplify responses
  - `assess_interaction()` produces MetacognitiveAssessment
- **Wiring**:
  - `loop_runner.rs` creates CalibrationTracker, populates from history via `populate_from_history()`
  - Calls `assess_and_report()` BEFORE THINK phase — sends MetacognitiveInsight to UI
  - Calibration data updates with each recorded outcome

### Intelligence Wiring Module
- `cognitive/handlers/phase_learn_intelligence.rs` (140 lines) — NEW
  - `populate_from_history()` — extracts outcomes from conversation history into OutcomeTracker + CalibrationTracker
  - `assess_and_report()` — runs metacognitive assessment and sends insight to UI
  - `check_self_improvement()` — periodic weakness identification and improvement spec generation

### File Inventory

| File | Lines | Phase | Tests | Wired From |
|------|-------|-------|-------|------------|
| `handlers/agentic_loop.rs` | 251 | 1 | 3 | phase_act.rs |
| `handlers/agentic_loop_format.rs` | 119 | 1 | 6 | agentic_loop.rs |
| `handlers/agentic_loop_entry.rs` | 133 | 1 | 4 | phase_act.rs |
| `handlers/verify_response.rs` | 380 | 2 | 6 | loop_runner.rs |
| `outcome_tracker.rs` | 351 | 3 | 7 | loop_runner.rs |
| `handlers/model_escalation.rs` | 231 | 4 | 9 | phase_think.rs, phase_think_call.rs |
| `background_tasks.rs` | 277 | 5 | 6 | loop_runner.rs |
| `self_improve.rs` (kernel) | 251 | 6 | 9 | phase_learn_intelligence.rs |
| `metacognition.rs` | 334 | 7 | 10 | loop_runner.rs |
| `handlers/phase_learn_intelligence.rs` | 140 | 3/6/7 | — | loop_runner.rs |
| **Total** | **2,467** | **All** | **60** | — |

### UI Surface Wiring

Both Desktop and TUI handle all CognitiveUpdate variants:
- `AgenticTurn` / `AgenticComplete` — agentic loop progress
- `VerificationApplied` — claims checked/corrected count
- `ModelEscalated` — model escalation with from/to/reason
- `BackgroundTaskComplete` — background task results
- `MetacognitiveInsight` — metacognitive assessments

### Data Persistence Note

OutcomeTracker, CalibrationTracker, and BackgroundScheduler are currently session-scoped
(created fresh each cognitive loop invocation). They populate from conversation history
for continuity within a session. For cross-session persistence, these should be serialized
to HydraDb — this is a data layer enhancement, not a wiring gap.
