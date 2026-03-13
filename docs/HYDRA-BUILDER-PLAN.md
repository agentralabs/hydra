# HYDRA BUILDER — Full System Construction Pipeline

> **Goal**: Hydra can build any complete, functional system from a `.md` spec.
> Give it a spec, it builds the thing. Not a toy — a real, compilable, testable product.

---

## WHAT EXISTS TODAY

```
Spec → LLM Gap Analysis → LLM Patch Generation → Apply → Cargo Check → Retry (3x)
```

**Working:**
- Reads any `.md` spec from disk
- LLM sees full 43-crate workspace map + existing file contents
- Patches target real files in real crates
- Cargo check verifies compilation, auto-reverts on failure
- Agentic retry feeds compile errors back to LLM for correction (max 3 attempts)
- Smart patch deduplicates imports/functions, registers modules
- Human approval with challenge phrase for safety

**Broken / Missing:**
- Cannot create new crates (Cargo workspace members)
- Cannot modify Cargo.toml (add dependencies)
- Cannot create non-Rust files (configs, docs, assets)
- Single-phase only — no scaffold → build → test → verify pipeline
- 10-patch limit per run — too few for a full system
- No test execution after build
- No multi-file coordinated generation (each patch is independent)

---

## THE 5-PHASE BUILDER PIPELINE

```
User: "build specs/MY-SYSTEM.md"
  │
  Phase 0: UNDERSTAND
  ├─ Read spec, classify complexity (single-crate vs multi-crate vs new-project)
  ├─ Query code index for existing relevant code
  └─ Generate build plan with ordered phases
  │
  Phase 1: SCAFFOLD
  ├─ Create new crates if needed (Cargo.toml, src/lib.rs)
  ├─ Add workspace members to root Cargo.toml
  ├─ Add dependencies to crate Cargo.tomls
  └─ Verify: cargo check passes on empty scaffolds
  │
  Phase 2: IMPLEMENT
  ├─ Generate code patches in batches (10 per batch, multiple batches)
  ├─ Each batch: apply → cargo check → retry on failure
  ├─ LLM sees results of previous batches (accumulated context)
  └─ Obstacle resolver handles stuck situations
  │
  Phase 3: TEST
  ├─ Generate test code (inline #[cfg(test)] modules)
  ├─ Run cargo test on affected crates
  ├─ Feed test failures back to LLM for fixes
  └─ Retry until tests pass or max attempts reached
  │
  Phase 4: VERIFY
  ├─ Run file size guard (400-line check)
  ├─ Run cargo clippy for quality
  ├─ Validate all new modules are registered
  └─ Generate build report with stats
```

---

## IMPLEMENTATION PRIORITIES

### P0: Cargo Manifest Operations (BLOCKING — nothing else works without this)

**New file: `crates/hydra-kernel/src/cargo_ops.rs` (~200 lines)**

The ability to read and modify Cargo.toml files safely.

```rust
pub struct CargoOps;

impl CargoOps {
    /// Add a workspace member to root Cargo.toml.
    pub fn add_workspace_member(project_dir: &Path, member: &str) -> Result<(), String>

    /// Create a new crate skeleton: Cargo.toml + src/lib.rs.
    pub fn scaffold_crate(
        project_dir: &Path,
        crate_name: &str,
        crate_type: CrateType, // Lib, Bin, Both
        dependencies: &[(&str, &str)], // (name, version_or_workspace)
    ) -> Result<(), String>

    /// Add a dependency to a crate's Cargo.toml.
    pub fn add_dependency(
        project_dir: &Path,
        crate_path: &str, // "crates/my-crate"
        dep_name: &str,
        dep_spec: &str, // "1.0" or "{ workspace = true }"
    ) -> Result<(), String>

    /// Add a workspace dependency to root Cargo.toml.
    pub fn add_workspace_dependency(
        project_dir: &Path,
        dep_name: &str,
        dep_spec: &str, // version string
    ) -> Result<(), String>

    /// Verify a crate compiles: cargo check -p <name> -j 1.
    pub fn check_crate(project_dir: &Path, crate_name: &str) -> Result<(), String>
}

pub enum CrateType { Lib, Bin, Both }
```

**Implementation strategy**: Use `toml` crate (already in workspace) to parse, modify, and serialize Cargo.toml. This avoids regex fragility.

**Tests**: scaffold_crate in tempdir, add dependency, verify Cargo.toml output.

### P1: Multi-Phase Build Orchestrator

**New file: `crates/hydra-kernel/src/build_orchestrator.rs` (~250 lines)**

The brain that sequences phases and tracks progress.

```rust
pub struct BuildOrchestrator {
    project_dir: PathBuf,
    spec: String,
    plan: BuildPlan,
    state: BuildState,
}

pub struct BuildPlan {
    pub phases: Vec<BuildPhase>,
    pub complexity: Complexity, // SingleCrate, MultiCrate, NewProject
    pub estimated_patches: usize,
}

pub enum BuildPhase {
    Scaffold { crates: Vec<CrateSpec> },
    Implement { batch: usize, total_batches: usize },
    Test { crate_name: String },
    Verify,
}

pub struct CrateSpec {
    pub name: String,
    pub crate_type: CrateType,
    pub dependencies: Vec<(String, String)>,
    pub description: String,
}

pub enum BuildState {
    Planning,
    Scaffolding { completed: usize, total: usize },
    Implementing { batch: usize, total: usize, patches_applied: usize },
    Testing { passed: usize, failed: usize },
    Verifying,
    Complete { report: BuildReport },
    Failed { phase: String, error: String, can_retry: bool },
}

pub struct BuildReport {
    pub crates_created: usize,
    pub files_created: usize,
    pub patches_applied: usize,
    pub tests_passing: usize,
    pub total_lines: usize,
    pub duration_ms: u64,
}

impl BuildOrchestrator {
    /// Create a build plan from a spec using LLM analysis.
    pub async fn plan(spec: &str, llm_config: &LlmConfig, project_dir: &Path) -> Result<Self, String>

    /// Execute the next phase. Returns the updated state.
    pub async fn execute_next(&mut self, llm_config: &LlmConfig) -> BuildState

    /// Run all phases to completion.
    pub async fn run_all(&mut self, llm_config: &LlmConfig, tx: &Sender<CognitiveUpdate>) -> BuildReport
}
```

### P2: Build Plan Generation (LLM prompt)

**New file: `crates/hydra-kernel/src/build_planner.rs` (~150 lines)**

LLM prompt that reads a spec and generates a structured build plan.

```rust
const BUILD_PLAN_PROMPT: &str = r#"You are planning a software build for Hydra, a Rust AI agent.

Given the spec, generate a build plan as JSON:
{
  "complexity": "single_crate" | "multi_crate" | "new_project",
  "crates": [
    {
      "name": "crate-name",
      "type": "lib" | "bin" | "both",
      "new": true/false,
      "dependencies": [["serde", "{ workspace = true }"]],
      "description": "what this crate does"
    }
  ],
  "implementation_order": [
    {"crate": "crate-name", "files": ["src/lib.rs", "src/types.rs"], "description": "core types and API"}
  ],
  "test_strategy": "inline" | "suite" | "integration"
}

Rules:
- Prefer extending existing crates over creating new ones
- New crates only when the spec explicitly describes a new system
- Keep crate count minimal (1-3 for most features)
- Use workspace dependencies when the dep already exists in root Cargo.toml
"#;

pub async fn generate_build_plan(
    spec: &str,
    llm_config: &LlmConfig,
    project_dir: &Path,
) -> Result<BuildPlan, String>
```

### P3: Batched Implementation

**Modify: `crates/hydra-native-cognitive/src/cognitive/handlers/sister_ops/implement_diagnose.rs`**

Replace single-shot patch generation with batched implementation:

```
For each batch in build plan:
  1. Generate up to 10 patches for this batch
  2. Apply with checkpoint
  3. Cargo check
  4. If fail → retry with error correction (3 attempts)
  5. If still fail → revert batch, report partial success
  6. If pass → move to next batch
  7. After all batches → run tests
```

The key change: the LLM sees the **accumulated state** from previous batches. Each batch prompt includes what was already built, so it can write code that uses types/functions from earlier batches.

### P4: Test Generation and Execution

**New file: `crates/hydra-kernel/src/build_tester.rs` (~120 lines)**

After implementation, generate and run tests:

```rust
const TEST_GEN_PROMPT: &str = r#"Generate Rust tests for the code that was just built.
- Use #[cfg(test)] mod tests { } inline in each file
- Test public API functions
- Test error cases
- Use assert_eq!, assert!, and #[should_panic] where appropriate
- No external test dependencies beyond what's in Cargo.toml
"#;

pub async fn generate_tests(
    implemented_files: &[(String, String)], // (path, content)
    spec: &str,
    llm_config: &LlmConfig,
) -> Result<Vec<Patch>, String>

pub fn run_tests(project_dir: &Path, crate_name: &str) -> Result<TestReport, String> {
    // cargo test -p <crate> -j 1
}

pub struct TestReport {
    pub passed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
```

### P5: Handler Wiring

**Modify: `implement_diagnose.rs`** — route "build" commands to the orchestrator:

```
"implement spec X.md" → existing single-phase pipeline (backward compat)
"build specs/X.md"    → new multi-phase orchestrator
"build system X.md"   → new multi-phase orchestrator
```

Add new intent detection: `IntentCategory::BuildSystem` for specs that need new crates.

**New CognitiveUpdate variants:**
```rust
BuildPhaseStarted { phase: String, detail: String },
BuildPhaseComplete { phase: String, duration_ms: u64 },
BuildProgress { phase: String, completed: usize, total: usize },
BuildComplete { report: String },
```

---

## FILE PLAN

| Priority | File | Lines | What |
|----------|------|-------|------|
| P0 | `hydra-kernel/src/cargo_ops.rs` | ~200 | Cargo.toml read/write, crate scaffold, dependency management |
| P0 | `hydra-kernel/src/cargo_ops_tests.rs` | ~150 | Tests for Cargo operations |
| P1 | `hydra-kernel/src/build_orchestrator.rs` | ~250 | Multi-phase build sequencer |
| P2 | `hydra-kernel/src/build_planner.rs` | ~150 | LLM-powered build plan generation |
| P3 | `implement_diagnose.rs` | modify | Route "build" to orchestrator, batched implementation |
| P4 | `hydra-kernel/src/build_tester.rs` | ~120 | Test generation and execution |
| P5 | `cognitive_update.rs` | modify | New Build* variants |
| P5 | `cognitive_handler.rs` | modify | Handle Build* variants in CLI |
| P5 | `app_handlers_cognitive.rs` | modify | Handle Build* variants in Desktop |

**Total new code: ~870 lines across 5 new files + modifications to 4 existing files.**

---

## ARCHITECTURE

```
User: "build specs/NEW-PAYMENT-SYSTEM.md"
  │
  ▼
IntentRouter → BuildSystem
  │
  ▼
BuildOrchestrator::plan(spec, llm_config, project_dir)
  │ LLM reads spec + workspace structure
  │ Returns: BuildPlan { complexity: MultiCrate, crates: [...], order: [...] }
  │
  ▼
Phase 0: SCAFFOLD
  │ CargoOps::scaffold_crate("hydra-payments", Lib, deps)
  │ CargoOps::add_workspace_member("crates/hydra-payments")
  │ cargo check → verify empty crate compiles
  │
  ▼
Phase 1: IMPLEMENT (batch 1 of N)
  │ analyze_spec_gaps(spec, batch_context, llm_config, project_dir)
  │ generate_patches(gaps, spec, llm_config, project_dir)
  │ apply_with_retry(patches, max_retries=3)
  │   ├─ apply patches
  │   ├─ cargo check
  │   ├─ if fail → fix_compile_errors() → retry
  │   └─ if pass → next batch
  │
  ▼
Phase 1: IMPLEMENT (batch 2 of N)
  │ LLM sees: spec + what batch 1 already built
  │ Generates patches that USE types/functions from batch 1
  │ Same apply → check → retry flow
  │
  ▼
Phase 2: TEST
  │ generate_tests(implemented_files, spec)
  │ cargo test -p hydra-payments -j 1
  │ if fail → feed failures to LLM → fix → retry
  │
  ▼
Phase 3: VERIFY
  │ check-file-size-guard.sh (all files < 400 lines)
  │ All modules registered
  │ Build report generated
  │
  ▼
BuildReport { crates: 1, files: 5, patches: 23, tests: 12, lines: 890, duration: 45s }
```

---

## WHAT THIS ENABLES

After implementation, Hydra can:

1. **Build new sisters** — `build specs/HYDRA-SISTER-FACTORY-VALIDATION.md`
2. **Build new features** — `build specs/REAL-TIME-COLLAB.md`
3. **Build standalone tools** — `build specs/LOG-ANALYZER.md`
4. **Build API services** — `build specs/WEBHOOK-SERVICE.md`
5. **Extend itself** — `build specs/NEW-COGNITIVE-PHASE.md`

The only dependency is Claude's reasoning (via LLM API calls). Everything else — scaffolding, patching, compiling, testing, retrying — is Hydra's own infrastructure.

---

## CONSTRAINTS

- **400-line max** per file — pipeline enforces this and splits automatically
- **Max 10 patches per batch** — prevents overwhelming the LLM context
- **Max 3 retries per batch** — prevents infinite loops
- **Human approval required** — challenge phrase for any build operation
- **Cargo check after every batch** — nothing proceeds if it doesn't compile
- **Checkpoint + revert** — every batch is atomic, reverted on failure
- **No unsafe code** — LLM prompt explicitly forbids it
- **No external network calls in generated code** — unless spec requires it

---

## BUILD ORDER

```
Week 1: P0 (Cargo ops) + P1 (Orchestrator)
         → Hydra can scaffold crates and sequence phases
Week 2: P2 (Planner) + P3 (Batched implementation)
         → Hydra can plan and build multi-file systems
Week 3: P4 (Testing) + P5 (Wiring)
         → Hydra can test what it builds and report results
```

After Week 3, run the Sister Factory Validation test again.
If it passes all 5 phases, register `/build` as a first-class capability.
