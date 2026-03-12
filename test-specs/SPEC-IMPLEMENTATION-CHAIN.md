# SPEC: Implementation Chain Runner

## Problem
Currently Hydra can only implement one spec at a time. There's no way to chain implementations: "implement A, then B, then C". Also, after implementing a spec, Hydra can't automatically generate and implement the next logical improvement.

## Requirement
Create an implementation chain runner that:
- Accepts a list of spec file paths
- Executes them sequentially (one at a time for safety)
- Tracks results per spec (success/failure/skipped)
- Stops on first failure (fail-fast)
- Generates a chain report showing all results

## Acceptance Criteria
1. `pub struct ChainStep` with fields: spec_path, status (pending/success/failed/skipped), gaps_found, patches_applied, error_message
2. `pub struct ChainResult` with fields: steps (Vec of ChainStep), total_gaps, total_patches, success_count, failure_count
3. `pub fn plan_chain(spec_paths: &[&str]) -> Vec<ChainStep>` — initialize steps as pending
4. `pub fn record_step_result(step: &mut ChainStep, success: bool, gaps: usize, patches: usize, error: Option<String>)`
5. `pub fn chain_summary(result: &ChainResult) -> String` — human-readable report
6. `pub fn should_continue(result: &ChainResult) -> bool` — false if any step failed
7. Unit tests

## Implementation Location
- New file: `crates/hydra-kernel/src/implementation_chain.rs`
