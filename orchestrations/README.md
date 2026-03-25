# Hydra Orchestration Registry

This is the **single source of truth** for every orchestration Hydra has or will have.

## Structure

- `00-REGISTRY.md` — Master index of all 105 orchestrations (32 complete + 73 future)
- `completed/` — One file per built orchestration (O00–O25)
- `future/` — One file per future orchestration (P1–META5)

## How to Use

**Building a new orchestration:**
1. Pick from `future/` or create a new entry
2. Read the spec in `specs/HYDRA-ORCHESTRATION-{N}-{NAME}.md` for implementation details
3. Follow the 12 laws in `specs/HYDRA-ORCHESTRATION-ROADMAP.md`
4. When complete: move file from `future/` to `completed/`, update `00-REGISTRY.md`

**Auditing an existing orchestration:**
1. Open its file in `completed/`
2. Check wiring (Law 10), edge cases, and status
3. Run verification commands listed in the file

## File Format

Every orchestration file follows the same template — status, description, crates, dependencies, wiring evidence, edge cases, key decisions. See any file in `completed/` for the format.

## Relationship to Other Documents

- `specs/` = HOW to build (implementation blueprints)
- `orchestrations/` = WHAT was built (status + summary + tracking)
- `catalogue/` = WHY it exists (conceptual/educational)
- `specs/HYDRA-EDGE-CASES-BIBLE.md` = WHAT can break (risk register)
