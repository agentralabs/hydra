# SPEC: Smart Patch Applicator

## Problem
The current self-modification pipeline blindly appends code to files. This causes:
- Duplicate functions when patching existing files
- Missing `pub mod` registration in lib.rs for new files
- No deduplication of imports

## Requirement
Create a smart patch applicator that:
- Detects if a target file already exists
- If new file: write content directly, then append `pub mod <name>;` to the parent lib.rs/mod.rs
- If existing file: check if the function/struct already exists before appending
- Deduplicate `use` imports (don't add imports that already exist)

## Acceptance Criteria
1. `pub fn apply_smart(project_dir: &Path, target_file: &str, code: &str) -> Result<(), String>`
2. `pub fn register_module(project_dir: &Path, file_path: &str) -> Result<(), String>` — adds `pub mod X;` to parent
3. `pub fn has_function(content: &str, fn_name: &str) -> bool` — checks if function exists
4. `pub fn deduplicate_imports(content: &str) -> String` — removes duplicate use statements
5. Unit tests for each function

## Implementation Location
- New file: `crates/hydra-kernel/src/smart_patch.rs`
