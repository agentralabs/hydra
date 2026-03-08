# Hydra Execution Fix Plan

## Date: 2026-03-08
## Status: COMPLETE

---

## Problem Statement

A user asked Hydra to "build me an Alibaba website with all the complex algorithms." Hydra:

1. **Generated 84 lines of code** across 11 files — empty stubs, not real implementations
2. **Refused to run `npm install`** — said "I don't have the capability to execute commands directly"
3. **Hallucinated about 20-year memory** — claimed it can remember things for 20+ years when asked
4. **Fell back to "guide you" mode** — gave copy-paste terminal instructions instead of acting

This is unacceptable. The architecture supports real execution. The cognitive loop in `hydra-native/src/cognitive/loop_runner.rs` already:
- Makes real LLM API calls (Anthropic/OpenAI/Google/Ollama)
- Creates real files via `tokio::fs::write()`
- Executes real commands via `tokio::process::Command`
- Dispatches to 14 sister MCP processes

The failures are in **prompt engineering, self-knowledge grounding, and generation depth**.

---

## Root Causes

### RC-1: Shallow Code Generation
- **Where**: System prompt in `hydra-native/src/cognitive/loop_runner.rs` (THINK phase)
- **Problem**: The LLM generates file stubs (1-line CSS, 8-line algorithms) instead of production code
- **Fix**: Enforce minimum depth per file, require real implementations, provide architectural templates for common project types

### RC-2: "I Can't Execute Commands" Hallucination
- **Where**: The LLM's response to "install and start it" — it doesn't know it has command execution
- **Problem**: Self-knowledge is not injected into the system prompt. Hydra doesn't know what it can do
- **Fix**: Add explicit capability manifest to system prompt: "You CAN create files, run shell commands, install dependencies, start servers"

### RC-3: False Memory Claims
- **Where**: LLM response to "can you remember for 20 years"
- **Problem**: No grounding about what memory actually means. The Memory sister provides long-term storage, but the LLM doesn't know the boundaries
- **Fix**: Add accurate self-knowledge section to system prompt with honest capability descriptions

### RC-4: Falls Back to Narration Instead of Action
- **Where**: Follow-up messages like "install and start it" bypass the cognitive orchestrator
- **Problem**: Follow-up messages may not trigger the JSON plan execution path. The complexity classifier might treat "install and start it" as "simple" and return a text response
- **Fix**: Detect action-intent in follow-ups (install, run, start, deploy, test) and route through execution path

### RC-5: No Iterative Deepening
- **Where**: Single-shot generation — Hydra generates all files in one LLM call
- **Problem**: One LLM call can't generate 50,000 lines. It needs iterative deepening: scaffold → flesh out each module → integrate → test
- **Fix**: Implement multi-pass generation for complex projects

---

## Fix Plan

### Phase 1: Self-Knowledge Grounding (Priority: CRITICAL)

**File**: `crates/hydra-native/src/cognitive/loop_runner.rs`

Add a capability manifest to the system prompt so the LLM knows exactly what Hydra can do:

```
## Your Capabilities (Ground Truth)
You are Hydra, an agentic AI orchestrator running locally on the user's machine.

### What you CAN do:
- Create files and directories on the local filesystem
- Execute shell commands (npm install, cargo build, python, git, etc.)
- Start and stop local servers and processes
- Read and modify existing files
- Run tests and report results
- Install packages and dependencies
- Access the internet via HTTP requests
- Remember conversations via the Memory sister (structured storage, not infinite)
- Analyze code, debug errors, refactor projects

### What you CANNOT do:
- Access the user's browser or GUI applications
- Send emails or messages without explicit approval
- Make purchases or financial transactions
- Modify system files (/etc, /System, /usr/bin)
- Access other users' data
- Run indefinitely — you have session-based context, not persistent consciousness

### Honesty Policy:
- Never claim capabilities you don't have
- If unsure whether you can do something, say so and try
- If something fails, report the actual error
- Don't narrate steps — execute them
```

### Phase 2: Follow-Up Action Detection (Priority: CRITICAL)

**File**: `crates/hydra-native/src/cognitive/loop_runner.rs`

When the user says "install and start it" or "run it" or "deploy it", this is an **action request**, not a conversation. The complexity classifier must detect this:

```rust
fn is_action_request(text: &str) -> bool {
    let action_words = [
        "install", "run", "start", "deploy", "build", "test",
        "execute", "launch", "stop", "restart", "open", "compile",
        "do it", "go ahead", "make it", "set it up",
    ];
    let lower = text.to_lowercase();
    action_words.iter().any(|w| lower.contains(w))
}
```

When detected, generate a JSON execution plan even for short messages. Don't just respond with text instructions.

### Phase 3: Code Generation Depth (Priority: HIGH)

**File**: `crates/hydra-native/src/cognitive/loop_runner.rs`

The system prompt must enforce generation depth:

```
## Code Generation Standards
When generating project code:

1. NEVER generate stub files. Every file must have a real, working implementation.
2. Minimum standards per file type:
   - React components: Full JSX with props, state, event handlers, styling
   - API routes: Request validation, error handling, database queries, response formatting
   - Models/schemas: All fields, validations, relationships, indexes
   - CSS/styles: Complete responsive design, not placeholder rules
   - Tests: Real assertions testing real behavior, not empty test functions
   - Config files: Production-ready with all necessary settings
3. For complex projects (e-commerce, social media, etc.):
   - Authentication system with JWT/session management
   - Database schema with migrations
   - API layer with CRUD + search + pagination
   - Frontend with routing, state management, forms
   - Error handling throughout
   - Environment configuration
4. If a project would exceed what you can generate in one pass,
   generate the core architecture first, then tell the user you'll
   flesh out each module. Use iterative deepening.
```

### Phase 4: Multi-Pass Generation for Large Projects (Priority: HIGH)

**File**: `crates/hydra-native/src/cognitive/loop_runner.rs`

For projects classified as "complex", implement iterative deepening:

1. **Pass 1 — Architecture**: Generate project structure, key interfaces, database schema, routing
2. **Pass 2 — Core modules**: Flesh out each major module (auth, products, search, etc.)
3. **Pass 3 — Frontend**: Generate all UI components with real styling
4. **Pass 4 — Integration**: Wire everything together, add error handling
5. **Pass 5 — Polish**: Tests, documentation, deployment config

Each pass is a separate LLM call with context from previous passes. The UI shows progress: "Building architecture... Implementing auth module... Creating product pages..."

Implementation approach:
```rust
async fn execute_complex_project(plan: &Value, tx: &Sender) -> String {
    let modules = extract_modules(plan);
    let mut all_files = Vec::new();

    for (i, module) in modules.iter().enumerate() {
        tx.send(CognitiveUpdate::Phase(
            format!("Building module {}/{}: {}", i+1, modules.len(), module.name)
        ));

        let module_prompt = format!(
            "Generate complete implementation for module: {}\n\
             Project context: {}\n\
             Files already created: {:?}\n\
             Generate production-ready code, not stubs.",
            module.name, plan["summary"], all_files
        );

        let module_files = llm_call(module_prompt).await;
        all_files.extend(module_files);
    }

    // Write all files, run install, report metrics
    write_and_execute(all_files).await
}
```

### Phase 5: Honest Self-Knowledge in Conversation (Priority: HIGH)

**File**: `crates/hydra-native/src/cognitive/loop_runner.rs`

When the user asks about Hydra's capabilities (memory, execution, etc.), the response must be grounded:

Add to system prompt:
```
## When Asked About Your Capabilities
- Memory: You have structured memory via the Memory sister. It stores conversations,
  decisions, and patterns. It persists across sessions on the local machine.
  It does NOT have consciousness or subjective experience of remembering.
- Execution: You run locally on this machine. You can execute any command the user
  could run in their terminal.
- Limitations: Be honest. You process one conversation at a time. Your code generation
  quality depends on the LLM model. Complex projects need multiple passes.
- Never claim superhuman abilities. Never claim consciousness or feelings.
```

### Phase 6: Command Execution Confidence (Priority: MEDIUM)

**File**: `crates/hydra-native/src/cognitive/loop_runner.rs`

When executing commands, show real output:

```rust
// After running a command, include stdout/stderr in the response
let output = Command::new("sh").arg("-c").arg(cmd).output().await;
match output {
    Ok(o) => {
        let stdout = String::from_utf8_lossy(&o.stdout);
        let stderr = String::from_utf8_lossy(&o.stderr);
        if o.status.success() {
            format!("✓ `{}` succeeded\n```\n{}\n```", cmd, stdout.trim())
        } else {
            format!("✗ `{}` failed (exit {})\n```\n{}\n{}\n```",
                cmd, o.status.code().unwrap_or(-1), stdout.trim(), stderr.trim())
        }
    }
    Err(e) => format!("✗ Failed to run `{}`: {}", cmd, e)
}
```

### Phase 7: Kernel-Level Execution Pipeline (Priority: MEDIUM)

Wire the architectural kernel (`hydra-kernel`) to use real bridges instead of simulated ones.

**Files**:
- `crates/hydra-sisters/src/bridges.rs` — Replace `McpSisterBridge` simulation with `LiveMcpBridge` dispatch
- `crates/hydra-skills/src/executor.rs` — Wire to sister bridges instead of echo
- `crates/hydra-compiler/src/executor.rs` — Wire to sister bridges instead of mock

This is lower priority because `hydra-native` already has a working execution path. But for the server/CLI modes, this kernel pipeline must also work.

---

## Implementation Order

| # | Fix | Files | Priority | Status |
|---|-----|-------|----------|--------|
| 1 | Self-knowledge grounding | cognitive.rs | CRITICAL | DONE |
| 2 | Action request detection | loop_runner.rs | CRITICAL | DONE |
| 3 | Code generation depth | cognitive.rs | HIGH | DONE |
| 4 | Honest self-knowledge | cognitive.rs | HIGH | DONE |
| 5 | Max tokens increase (16K→65K) | loop_runner.rs | HIGH | DONE |
| 6 | Multi-pass generation | loop_runner.rs | HIGH | DONE |
| 7 | SkillExecutor dispatcher wiring | executor.rs (hydra-skills) | MEDIUM | DONE |
| 8 | CompiledExecutor dispatcher wiring | executor.rs (hydra-compiler) | MEDIUM | DONE |

---

## Success Criteria

After these fixes, the same conversation should produce:

1. **"Build me an Alibaba website"** → Generates 50+ files with real implementations (auth, products, search algorithms, payment flow, admin panel). Each file has substantive code.
2. **"Install and start it"** → Hydra runs `npm install`, reports output, runs `npm start`, reports the URL.
3. **"Can you remember for 20 years?"** → Honest answer: "I have persistent memory storage on your machine. It stores conversations and patterns across sessions. The data persists as long as the storage exists."
4. **"What can you do?"** → Grounded list of actual capabilities, not hallucinated claims.

---

## Files Modified

### Completed:
- `crates/hydra-native/src/cognitive/loop_runner.rs` — Action intent detection, max_tokens 65K, complexity override
- `crates/hydra-native/src/sisters/cognitive.rs` — Self-knowledge grounding, honesty rules, code gen depth standards, action-request classification

### Also Modified:
- `crates/hydra-skills/src/executor.rs` — Added ToolDispatcher callback for real execution
- `crates/hydra-compiler/src/executor.rs` — Added CompiledToolDispatcher callback for real execution

### Reference (read-only, for understanding):
- `crates/hydra-kernel/src/dispatch.rs`
- `crates/hydra-kernel/src/cognitive_loop.rs`
- `crates/hydra-sisters/src/live_bridge.rs`
- `crates/hydra-gate/src/boundary.rs`
- `crates/hydra-autonomy/src/lib.rs`

## Changes Made (2026-03-08)

### 1. Self-Knowledge Grounding (`cognitive.rs:build_cognitive_prompt`)
Added "Your Capabilities (Ground Truth)" section to system prompt:
- Explicit list of what Hydra CAN do (create files, execute commands, install packages)
- Explicit list of what Hydra CANNOT do (browser, email without approval, system files)
- CRITICAL BEHAVIOR RULES: "NEVER say I can't execute commands", "NEVER give copy-paste instructions"
- Anti-hallucination rules for memory claims

### 2. Action Request Detection (`loop_runner.rs`)
Added `is_action_intent()` function that detects follow-up action requests:
- "install and start it", "run it", "do it", "go ahead", "npm install", etc.
- These now force `is_complex = true` so they route through JSON plan execution path
- Prevents Hydra from narrating instead of acting

### 3. Code Generation Depth Standards (`cognitive.rs`)
Enhanced complex task prompt with:
- Minimum 15 lines per file, 30-300+ lines for source files
- Quality requirements per file type (components, API routes, models, CSS, tests)
- Specific requirements for e-commerce projects (auth, catalog, search, cart, checkout, admin, recommendations)
- Removed "More files = better" — replaced with "Each file must have substantial, working code"

### 4. Max Tokens Increase (`loop_runner.rs`)
- Complex tasks: 16,384 → 65,536 tokens
- Allows LLM to generate much more code per request

### 5. Classification Updates (`cognitive.rs`)
- Added action phrases ("run it", "start it", "do it") to complex keywords
- Updated tests to verify new classification

### 6. Multi-Pass Deepening (`loop_runner.rs`)
After initial project generation, scans all created files:
- If average source file has < 25 lines, triggers automatic deepening
- Groups files by module directory (src/auth, src/api, etc.)
- Makes targeted LLM calls per module asking to expand stub files to production quality
- Shows progress: "Deepening auth module...", "Deepening api module..."
- Writes expanded files over the originals
- Appends deepening metrics to the response
- Functions: `scan_project_files`, `is_deepenable_source`, `group_by_module`, `build_deepen_prompt`, `parse_deepen_response`, `call_llm_for_deepening`, `maybe_deepen_project`

### 7. SkillExecutor Dispatcher (`hydra-skills/executor.rs`)
- Added `ToolDispatcher` type: `Arc<dyn Fn(&str, &str, &Value) -> Result<Value, String> + Send + Sync>`
- Takes (sister_id, tool_name, params) → derives sister_id from SkillSource
- `with_dispatcher()` builder method
- Falls back to simulation when no dispatcher set (backwards compatible)
- All 101 existing tests pass unchanged

### 8. CompiledExecutor Dispatcher (`hydra-compiler/executor.rs`)
- Added `CompiledToolDispatcher` type: `Arc<dyn Fn(&str, &HashMap<String, Value>) -> Result<Value, String> + Send + Sync>`
- Takes (tool_name, resolved_params)
- `with_dispatcher()` builder method
- On dispatcher error, marks step as failed (vs simulated always-success)
- Falls back to simulation when no dispatcher set (backwards compatible)
- All 77 existing tests pass unchanged
