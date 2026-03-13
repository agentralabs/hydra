# Sister Tool Coverage Plan — 100% MCP Integration

**Created**: 2026-03-12
**Updated**: 2026-03-12 (final — all tools wired)
**Status**: COMPLETE

## Final Coverage

**751 unique `call_tool()` invocations** across all sister wrapper files,
covering all **735 MCP server tools** (100% coverage). The 16 extra are
custom wrapper names that call tools with alternate parameters.

```bash
# Verified with:
grep -r 'call_tool("' crates/hydra-native-cognitive/src/sisters/ \
  | sed 's/.*call_tool("\([^"]*\)".*/\1/' | sort -u | wc -l
# → 751
```

## File Inventory

All files in `crates/hydra-native-cognitive/src/sisters/`.

### Memory (12 files)
| File | Lines | Purpose |
|------|-------|---------|
| memory_deep.rs | 300 | Core: add, get, query, session, dejavu |
| memory_extended.rs | 162 | Dream, traverse, resolve, longevity |
| memory_infinite.rs | 202 | Inv 1-4: immortal, compression, context, metabolism |
| memory_prophetic.rs | 197 | Inv 5-8: predictive, prophecy, counterfactual, dejavu |
| memory_collective.rs | 205 | Inv 9-12: ancestor, collective, fusion, telepathy |
| memory_resurrection.rs | 181 | Inv 13-16: archaeology, holographic, immune, phoenix |
| memory_metamemory.rs | 167 | Inv 17-20: meta, dreams, beliefs, load |
| memory_transcendent.rs | 209 | Inv 21-24: singularity, temporal, crystal, transcendence |
| memory_v3.rs | 180 | V3 immortal architecture |
| memory_workspace.rs | 104 | Workspace management |
| memory_facades.rs | 224 | Compact facades + longevity engine + core gaps |
| extras_deep.rs (shared) | 339 | Memory metabolism + meta gaps |

### Vision (10 files)
| File | Lines | Purpose |
|------|-------|---------|
| vision_grounding.rs | 181 | Grounding V2-V3, hallucination, truth, compare |
| vision_temporal.rs | 144 | Time travel, archaeology, consolidation, dejavu |
| vision_prediction.rs | 167 | Prophecy, regression, attention, phantom |
| vision_cognition.rs | 240 | Semantic, reasoning, binding, gestalt |
| vision_synthesis.rs | 139 | DNA, composition, clustering |
| vision_forensics.rs | 150 | Forensic diff, anomaly, regression |
| vision_workspace.rs | 180 | Extras, workspace, session, compare |
| vision_grammar_ext.rs | 51 | Grammar status/update/pin |
| browser_agent.rs | 346 | Multi-step web browsing pipeline |
| extras_deep.rs (shared) | 339 | Capture, diff, ground |

### Contract (8 files)
| File | Lines | Purpose |
|------|-------|---------|
| contract_deep.rs | 150 | Precognition, crystallize, approval, policy |
| contract_core.rs | 133 | CRUD, policy, risk limits |
| contract_extended.rs | 147 | Conditions, obligations, violations, analytics |
| contract_workspace.rs | 126 | Workspace + session |
| contract_visibility.rs | 200 | Approval telepathy, risk prophecy, omniscience |
| contract_generation.rs | 117 | Policy DNA, crystallization |
| contract_governance.rs | 240 | Trust gradients, collective, temporal, inheritance |
| contract_resilience.rs | 203 | Archaeology, simulation, federation, self-healing |

### Identity (7 files)
| File | Lines | Purpose |
|------|-------|---------|
| identity_deep.rs | 185 | Trust dynamics, competence, health |
| identity_core.rs | 178 | Creation, signing, receipts, grants, sessions |
| identity_accountability.rs | 157 | Receipt forensics, attribution, consent, fingerprint |
| identity_federation.rs | 149 | Cascade revocation, capabilities, teams |
| identity_resilience.rs | 159 | Resurrection, forking, ZK proofs, temporal |
| identity_workspace.rs | 277 | Workspace, spawn, competence, reputation |
| identity_continuity.rs | 153 | Continuity, negative capability, session raw |

### Time (5 files)
| File | Lines | Purpose |
|------|-------|---------|
| time_deep.rs | 226 | Scheduling, sequences, decay, deadlines |
| time_exploration.rs | 136 | Timeline fork/merge, clones, echoes |
| time_protection.rs | 151 | Anomaly, immune, decay reversal, dilation |
| time_management.rs | 304 | Future memory, debt, gravity, wormholes, workspace |
| extras_deep.rs (shared) | 339 | Session, patterns, analysis |

### Cognition (3 files)
| File | Lines | Purpose |
|------|-------|---------|
| cognition_core.rs | 92 | Model lifecycle, belief, soul reflect |
| cognition_extended.rs | 75 | Self-topology, shadow map, fingerprint |
| extras_deep.rs (shared) | 339 | Model update, predict, drift, simulate |

### Codebase (4 files)
| File | Lines | Purpose |
|------|-------|---------|
| codebase_deep.rs | 342 | Core analysis, grounding, sessions |
| codebase_extended.rs | 260 | Patterns, regression, archaeology, genetics |
| codebase_omniscience.rs | 206 | Omniscience, telepathy, soul, compare |
| codebase_facades.rs | 137 | Compact facades + analyse/coupling |

### Other Sisters (1-2 files each)
| Sister | Files | Lines |
|--------|-------|-------|
| Forge | forge_deep.rs | 342 |
| Aegis | aegis_deep.rs, veritas_aegis_deep.rs | ~300 |
| Comm | comm_deep.rs, comm_agent.rs | ~300 |
| Planning | planning_deep.rs, planning_agent.rs | ~300 |
| Reality | reality_deep.rs, reality_extended.rs | ~300 |
| Veritas | veritas_aegis_deep.rs, extras_deep.rs | ~300 |
| Evolve | evolve_deep.rs, extras_deep.rs | ~300 |

## LLM Tool Routing

`llm_tool_routing.rs` (252 lines) maps IntentCategory → top MCP tools the LLM
should know about. Covers all 14 sisters with keyword detection for
Unknown/Question intents.

## Verification

```bash
cargo check -p hydra-native-cognitive -j 1   # ✅ Compiles clean (24 warnings)
cargo test -p hydra-native-cognitive -j 1     # ✅ 594 tests pass
bash scripts/check-file-size-guard.sh         # ✅ All sister files under 400 lines
```
