# HYDRA Sister Integration Blueprint — Full Strength, Minimum Tokens

> Hydra has 500+ MCP tools across 14 sisters. This blueprint maps every tool to a trigger condition, phase, and token cost. Target: 95%+ tool utilization at <2,500 tokens/message.

## The Math: How to Use 500+ Tools Without Burning Tokens

**The problem**: 500+ tools × ~30 tokens/call = 15,000 tokens if called blindly.
**The solution**: Tiered activation with mathematical trigger conditions.

```
Token Budget Formula:
  T(msg) = T_always + T_complex × C + T_code × D + T_conditional × P(trigger)

Where:
  T_always   = ~300 tokens (7 tools, every message)
  T_complex  = ~400 tokens (12 tools, only complex tasks)  C ∈ {0,1}
  T_code     = ~200 tokens (8 tools, only code tasks)       D ∈ {0,1}
  T_conditional = Σ(tool_cost × P(trigger_i))               P ∈ [0,1]

Expected per-message: Simple=~350 | Complex=~900 | Complex+Code=~1,200
Session overhead: ~500 tokens (amortized over N messages → 500/N → ~0 at scale)
```

**Key insight**: Most tools fire on *conditions*, not every message. A tool with P(trigger)=0.1 contributes only 10% of its token cost to the expected budget.

---

## 1. Tool Activation Tiers

### Tier 0: Session Lifecycle (~500 tokens, once per session)

| Tool | Sister | Tokens | Phase | Notes |
|------|--------|-------:|-------|-------|
| `memory_session_resume` | Memory | 80 | INIT | Bootstraps "where did we stop?" |
| `session_start` | Memory | 30 | INIT | Creates session boundary |
| `comm_session` (start) | Comm | 30 | INIT | Establishes conversation trail |
| `comm_register_agent` | Comm | 30 | INIT | Registers in agent swarm |
| `time_session_start` | Time | 30 | INIT | Starts time tracking |
| `contract_session_resume` | Contract | 30 | INIT | Restores policy state |
| `identity_whoami` | Identity | 20 | INIT | Identity bootstrap |
| `aegis_session_create` | Aegis | 20 | INIT | Security session context |
| `session_end` | Memory | 30 | END | Persists for future resume |
| `comm_session` (end) | Comm | 20 | END | Closes trail |
| `comm_deregister_agent` | Comm | 20 | END | Cleans up swarm |
| `time_session_end` | Time | 20 | END | Closes time tracking |
| `memory_ghost_write` | Memory | 60 | END | Final session narrative |
| `cognition_drift_track` | Cognition | 30 | END | Session-level drift |
| `aegis_session_end` | Aegis | 20 | END | Close security session |
| `contract_evidence` | Contract | 20 | END | Final audit evidence |

**INIT: ~270 tokens | END: ~220 tokens | Total: ~490 tokens per session**

---

### Tier 1: Always-On (~300 tokens, every message)

These 10 tools fire unconditionally. They define Hydra's baseline intelligence.

| Tool | Sister | Tokens | Phase | Why |
|------|--------|-------:|-------|-----|
| `memory_query` (facts) | Memory | 50 | PERCEIVE | User preferences, corrections |
| `memory_predict` | Memory | 50 | PERCEIVE | Preload likely-needed memories |
| `memory_dejavu_check` | Memory | 30 | PERCEIVE | Detect returning topics |
| `cognition_model_query` | Cognition | 40 | PERCEIVE | User model (style, expertise) |
| `cognition_belief_query` | Cognition | 30 | PERCEIVE | Active beliefs for context |
| `comm_inbox` | Comm | 30 | PERCEIVE | Pending messages from agents |
| `identity_whoami` | Identity | 20 | PERCEIVE | Hydra's current persona |
| `cognition_belief_revise` | Cognition | 30 | LEARN | Update beliefs from interaction |
| `time_duration_track` | Time | 20 | LEARN | Track interaction duration |
| `contract_crystallize` | Contract | 30 | LEARN | Receipt chain audit |

**Expected: ~330 tokens/message** — this is the fixed floor.

---

### Tier 2: Complexity-Gated (~400 tokens, complex tasks only, P≈0.3)

Activated when `is_complex=true` (multi-step, code generation, planning, analysis).

| Tool | Sister | Tokens | Phase | Why |
|------|--------|-------:|-------|-----|
| `memory_query` (general) | Memory | 50 | PERCEIVE | Broader context |
| `memory_longevity_search` | Memory | 60 | PERCEIVE | Deep 20-year hierarchy |
| `memory_similar` | Memory | 40 | PERCEIVE | Semantic similarity search |
| `memory_ground` | Memory | 30 | PERCEIVE | Fact verification |
| `memory_temporal_recall` | Memory | 40 | PERCEIVE | Time-contextual retrieval |
| `reality_context` | Reality | 30 | PERCEIVE | Environment grounding |
| `reality_environment` | Reality | 20 | PERCEIVE | OS/runtime context |
| `verify_intent` / `veritas_detect_ambiguity` | Veritas | 30 | PERCEIVE | Detect ambiguity |
| `policy_query` / `policy_check` | Contract | 20 | PERCEIVE | Policy constraints |
| `goal_query` | Planning | 30 | PERCEIVE | Active goals |
| `forge_blueprint_create` | Forge | 80 | THINK | Architecture blueprint |
| `veritas_compile_intent` | Veritas | 40 | THINK | Structured intent |
| `evolve_match_context` | Evolve | 30 | THINK | Pattern matching |
| `evolve_confidence` | Evolve | 20 | THINK | Match confidence |
| `cognition_predict` | Cognition | 30 | THINK | Intent prediction |
| `cognition_detect_drift` | Cognition | 30 | THINK | Behavior drift check |

**Expected cost: ~400 × 0.3 = ~120 tokens/message average**

---

### Tier 3: Code-Task Tools (~250 tokens, code tasks only, P≈0.25)

Activated when intent = CodeGeneration, Debug, Refactor, or CodeExplanation.

| Tool | Sister | Tokens | Phase | Why |
|------|--------|-------:|-------|-----|
| `search_semantic` | Codebase | 60 | PERCEIVE | Semantic code search |
| `concept_find` | Codebase | 40 | PERCEIVE | Find code concepts |
| `impact_analyze` | Codebase | 50 | PERCEIVE | Change impact analysis |
| `forge_entity_infer` | Forge | 30 | THINK | Infer entities from description |
| `forge_skeleton_create` | Forge | 40 | ACT | Generate code skeletons |
| `forge_test_generate` | Forge | 40 | ACT | Auto-generate test scaffolds |
| `forge_dependency_resolve` | Forge | 30 | ACT | Resolve dependencies |
| `forge_wiring_create` | Forge | 30 | ACT | Component wiring |
| `aegis_validate_complete` | Aegis | 40 | ACT | Validate generated code |
| `aegis_scan_security` | Aegis | 40 | ACT | Security vulnerability scan |
| `pattern_extract` | Codebase | 30 | LEARN | Extract code patterns |
| `evolve_crystallize` | Evolve | 20 | LEARN | Crystallize code pattern |

**Expected cost: ~250 × 0.25 = ~63 tokens/message average**

---

### Tier 4: Conditional Tools (~30-80 tokens each, triggered by content detection)

Each tool has a specific trigger. P(trigger) varies — expected aggregate ≈ 100 tokens/message.

#### Memory Conditional (P varies)

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `memory_causal` | 50 | "why" questions | 0.08 | 4 |
| `memory_get` | 30 | Past reference ("that thing we did") | 0.10 | 3 |
| `memory_correct` | 30 | User says "actually", "no,", "that's wrong" | 0.05 | 1.5 |
| `memory_quality` | 20 | Non-greetings (quality scoring) | 0.60 | 12 |
| `memory_traverse` | 40 | Deep knowledge graph query | 0.05 | 2 |
| `memory_resolve` | 30 | Conflicting memories detected | 0.03 | 0.9 |
| `memory_context` | 30 | Context switch detected | 0.10 | 3 |
| `memory_evidence` | 30 | Evidence-based claim | 0.08 | 2.4 |
| `memory_suggest` | 20 | User seems stuck | 0.05 | 1 |

#### Cognition Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `cognition_simulate` | 40 | Proactive suggestion context | 0.05 | 2 |
| `cognition_self_topology` | 30 | Deep user understanding needed | 0.03 | 0.9 |
| `cognition_shadow_map` | 30 | Blind spot detection | 0.03 | 0.9 |
| `cognition_pattern_fingerprint` | 30 | Personalization needed | 0.05 | 1.5 |
| `cognition_model_update` | 40 | Always (LEARN) | 1.0 | 40 |
| `cognition_soul_reflect` | 30 | Every 5th message | 0.20 | 6 |

#### Comm Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `comm_send` | 30 | Inter-agent message needed | 0.05 | 1.5 |
| `comm_broadcast` | 30 | Swarm-wide announcement | 0.02 | 0.6 |
| `comm_health` | 20 | Every 30 messages (health check) | 0.03 | 0.6 |
| `comm_forensics` | 30 | Communication debug needed | 0.01 | 0.3 |
| `comm_affect` | 20 | Emotional tracking needed | 0.10 | 2 |
| `comm_message` (search) | 30 | Past message search | 0.05 | 1.5 |
| `comm_session` (log) | 30 | capture=all | 0.80 | 24 |
| `comm_session` (context) | 30 | Session resume query | 0.05 | 1.5 |
| `broadcast_insight` | 30 | User correction detected | 0.05 | 1.5 |
| `comm_trust` | 20 | Trust verification | 0.03 | 0.6 |
| `comm_semantic` | 30 | Semantic message search | 0.03 | 0.9 |
| `comm_temporal` | 30 | Time-based message search | 0.03 | 0.9 |
| `comm_collaboration` | 30 | Multi-agent task | 0.02 | 0.6 |
| `comm_consent` | 20 | Data sharing consent | 0.01 | 0.2 |
| `comm_federation` | 30 | Federated instance | 0.02 | 0.6 |
| `comm_hive` | 30 | Swarm coordination | 0.02 | 0.6 |
| `comm_workspace` | 30 | Workspace context | 0.03 | 0.9 |

#### Veritas Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `veritas_verify_claim` | 30 | Claims detected in response | 0.15 | 4.5 |
| `veritas_score_confidence` | 30 | Always (ACT verification) | 1.0 | 30 |
| `veritas_check_consistency` | 30 | Has memory context | 0.80 | 24 |
| `veritas_check_uncertainty` | 30 | Complex tasks | 0.30 | 9 |
| `veritas_reason_causally` | 30 | Causal explanation needed | 0.05 | 1.5 |
| `veritas_extract_claims` | 30 | Response has claims | 0.10 | 3 |
| `veritas_synthesize` | 40 | Multiple sources to merge | 0.05 | 2 |
| `veritas_generate_question` | 30 | Clarification useful | 0.05 | 1.5 |

#### Aegis Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `aegis_check_input` | 30 | Always (DECIDE) | 1.0 | 30 |
| `aegis_check_output` | 40 | Has response (ACT) | 1.0 | 40 |
| `aegis_shadow_execute` | 60 | Shell commands present | 0.20 | 12 |
| `aegis_confidence_score` | 30 | Code generation | 0.25 | 7.5 |
| `aegis_correction_hint` | 30 | Code errors detected | 0.05 | 1.5 |
| `aegis_rollback` | 30 | Execution failure | 0.03 | 0.9 |
| `aegis_session_status` | 20 | Session health check | 0.05 | 1 |
| `aegis_validate_streaming` | 30 | Streaming response | 0.10 | 3 |

#### Evolve Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `evolve_pattern_store` | 30 | New pattern detected | 0.10 | 3 |
| `evolve_pattern_search` | 30 | Pattern lookup | 0.15 | 4.5 |
| `evolve_pattern_get` | 20 | Specific pattern needed | 0.10 | 2 |
| `evolve_pattern_list` | 20 | Pattern overview | 0.05 | 1 |
| `evolve_update_usage` | 20 | Pattern matched | 0.15 | 3 |
| `evolve_optimize` | 40 | Every 10 messages | 0.10 | 4 |
| `evolve_compose` | 40 | Multiple patterns needed | 0.05 | 2 |
| `evolve_coverage` | 30 | Coverage analysis | 0.03 | 0.9 |
| `evolve_match_signature` | 30 | Decision matching | 0.05 | 1.5 |
| `evolve_get_body` | 20 | Pattern body needed | 0.10 | 2 |
| `evolve_pattern_delete` | 20 | Cleanup obsolete patterns | 0.01 | 0.2 |
| `evolve_record_pattern` | 20 | Always (LEARN) | 1.0 | 20 |
| `evolve_suggest_improvement` | 30 | Complex (LEARN) | 0.30 | 9 |
| `evolve_collective_share` | 20 | Federation active | 0.02 | 0.4 |

#### Forge Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `blueprint_query` / `forge_blueprint_get` | 30 | Existing blueprint check | 0.15 | 4.5 |
| `forge_blueprint_list` | 20 | List blueprints | 0.05 | 1 |
| `forge_blueprint_update` | 30 | Update existing | 0.05 | 1.5 |
| `forge_blueprint_validate` | 30 | Validate architecture | 0.10 | 3 |
| `forge_dependency_add` | 20 | Add dependency | 0.05 | 1 |
| `forge_entity_add` | 20 | Register entity | 0.05 | 1 |
| `forge_export` | 30 | Export blueprint | 0.02 | 0.6 |
| `forge_import_graph` | 30 | Import dependency graph | 0.02 | 0.6 |
| `forge_structure_generate` | 40 | Generate project structure | 0.10 | 4 |

#### Reality Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `reality_ground` | 30 | Claims to ground | 0.10 | 3 |
| `reality_capability` | 20 | "Can Hydra do X?" | 0.05 | 1 |
| `reality_coherence` | 30 | Reality check needed | 0.03 | 0.9 |
| `reality_deployment` | 20 | Deployment context | 0.05 | 1 |
| `reality_hallucination` | 30 | Hallucination detection | 0.05 | 1.5 |
| `reality_layer` | 20 | Layer context | 0.03 | 0.6 |
| `reality_memory` | 30 | Reality-memory sync | 0.03 | 0.9 |
| `reality_resource` | 20 | Resource check | 0.05 | 1 |
| `reality_stakes` | 20 | Consequence assessment | 0.05 | 1 |
| `reality_temporal` | 20 | Temporal grounding | 0.03 | 0.6 |
| `reality_topology` | 30 | System topology | 0.03 | 0.9 |
| `reality_workspace` | 20 | Workspace grounding | 0.05 | 1 |
| `reality_anchor` | 30 | Anchor persistent state | 0.05 | 1.5 |
| `reality_substrate` | 20 | Runtime substrate | 0.05 | 1 |

#### Time Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `time_stats` | 20 | Time-related query | 0.10 | 2 |
| `time_query_now` | 15 | "What time?" | 0.05 | 0.75 |
| `time_deadline_add` | 20 | User sets deadline | 0.03 | 0.6 |
| `time_deadline_complete` | 20 | Deadline completed | 0.02 | 0.4 |
| `time_deadline_list` | 20 | Complex (PERCEIVE) | 0.30 | 6 |
| `time_deadline_overdue` | 20 | Check overdue items | 0.05 | 1 |
| `time_decay_alert` | 20 | Staleness detection | 0.05 | 1 |
| `time_decay_create` | 20 | Set decay policy | 0.02 | 0.4 |
| `time_decay_value` | 20 | Check decay value | 0.03 | 0.6 |
| `time_duration_aggregate` | 20 | Duration summary | 0.05 | 1 |
| `time_duration_estimate` | 20 | Estimate task duration | 0.10 | 2 |
| `time_evidence` | 20 | Temporal evidence | 0.03 | 0.6 |
| `time_ground` | 20 | Time grounding | 0.05 | 1 |
| `time_refresh` | 15 | Refresh time context | 0.05 | 0.75 |
| `time_schedule_available` | 20 | Scheduling query | 0.03 | 0.6 |
| `time_schedule_conflicts` | 20 | Schedule conflicts | 0.02 | 0.4 |
| `time_schedule_create` | 20 | Create schedule | 0.02 | 0.4 |
| `time_schedule_range` | 20 | Schedule range query | 0.02 | 0.4 |
| `time_schedule_reschedule` | 20 | Reschedule | 0.01 | 0.2 |
| `time_sequence_advance` | 20 | Advance sequence | 0.03 | 0.6 |
| `time_sequence_create` | 20 | Create sequence | 0.02 | 0.4 |
| `time_sequence_status` | 20 | Sequence status | 0.03 | 0.6 |
| `time_session_resume` | 30 | Resume query | 0.05 | 1.5 |
| `time_suggest` | 20 | Time suggestion | 0.05 | 1 |
| `time_search_sessions` | 30 | "Yesterday"/"last week" | 0.08 | 2.4 |
| `time_analysis_patterns` | 30 | Session end | 0.05 | 1.5 |
| `time_workspace_*` (6 tools) | 20 ea | Federation/workspace | 0.02 | 2.4 |

#### Contract Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `contract_precognition` | 30 | Always (DECIDE) | 1.0 | 30 |
| `contract_policy_check` | 30 | Always (DECIDE) | 1.0 | 30 |
| `contract_request_approval` | 30 | Approval needed | 0.05 | 1.5 |
| `approval_decide` | 20 | Approval response | 0.05 | 1 |
| `approval_list` | 20 | List pending approvals | 0.02 | 0.4 |
| `approval_request` | 20 | Request approval | 0.05 | 1 |
| `condition_add` | 20 | Add contract condition | 0.02 | 0.4 |
| `condition_evaluate` | 20 | Evaluate condition | 0.03 | 0.6 |
| `contract_create` | 30 | Create contract | 0.02 | 0.6 |
| `contract_get` | 20 | Get contract | 0.03 | 0.6 |
| `contract_ground` | 20 | Ground contract | 0.03 | 0.6 |
| `contract_list` | 20 | List contracts | 0.02 | 0.4 |
| `contract_sign` | 20 | Sign contract | 0.02 | 0.4 |
| `contract_stats` | 20 | Contract statistics | 0.03 | 0.6 |
| `contract_suggest` | 20 | Contract suggestion | 0.03 | 0.6 |
| `contract_verify` | 20 | Verify contract | 0.03 | 0.6 |
| `contract_record_decision` | 30 | Always (LEARN) | 1.0 | 30 |
| `obligation_add` | 20 | Add obligation | 0.02 | 0.4 |
| `obligation_check` | 20 | Check obligations | 0.05 | 1 |
| `policy_add` | 20 | Add policy | 0.02 | 0.4 |
| `policy_list` | 20 | List policies | 0.03 | 0.6 |
| `risk_limit_check` | 20 | Check risk limits | 0.05 | 1 |
| `risk_limit_list` | 20 | List risk limits | 0.02 | 0.4 |
| `risk_limit_set` | 20 | Set risk limit | 0.02 | 0.4 |
| `violation_list` | 20 | List violations | 0.03 | 0.6 |
| `violation_report` | 20 | Report violation | 0.02 | 0.4 |
| `contract_context_log` | 20 | Log context | 0.10 | 2 |
| `contract_workspace_*` (6 tools) | 20 ea | Federation | 0.02 | 2.4 |

#### Planning Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `planning_goal` (create) | 30 | Complex (ACT) | 0.30 | 9 |
| `planning_progress` | 20 | Has active goal | 0.30 | 6 |
| `planning_decision` | 20 | Decision made | 0.10 | 2 |
| `planning_commitment` | 20 | Due-soon check (PERCEIVE) | 0.30 | 6 |
| `planning_counterfactual` | 30 | "What if?" question | 0.03 | 0.9 |
| `planning_singularity` | 20 | Theme identification | 0.07 | 1.4 |
| `planning_dream` | 30 | Idle processing | 0.02 | 0.6 |
| `planning_metamorphosis` | 30 | Goal restructuring | 0.02 | 0.6 |
| `planning_chain` | 30 | Task chaining | 0.05 | 1.5 |
| `planning_consensus` | 20 | Multi-agent consensus | 0.02 | 0.4 |
| `planning_federate` | 20 | Federation task | 0.02 | 0.4 |
| `planning_checkpoint` | 20 | Checkpoint progress | 0.30 | 6 |
| `planning_context_log` | 20 | Log context | 0.10 | 2 |
| `planning_evidence` | 20 | Evidence for plan | 0.05 | 1 |
| `planning_ground` | 20 | Ground plan | 0.05 | 1 |
| `planning_suggest` | 20 | Planning suggestion | 0.05 | 1 |
| `planning_workspace_*` (6 tools) | 20 ea | Federation | 0.02 | 2.4 |

#### Vision Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `vision_capture` | 50 | Visual action/URL opened | 0.10 | 5 |
| `vision_ocr` | 40 | Image/screenshot input | 0.05 | 2 |
| `vision_compare` | 40 | Before/after comparison | 0.03 | 1.2 |
| `vision_diff` | 40 | Visual diff needed | 0.03 | 1.2 |
| `vision_track` | 30 | Track visual state | 0.03 | 0.9 |
| `vision_similar` | 30 | Visual similarity search | 0.02 | 0.6 |
| `vision_query` | 30 | Visual query | 0.05 | 1.5 |
| `vision_ground` | 30 | Visual grounding | 0.03 | 0.9 |
| `vision_evidence` | 30 | Visual evidence | 0.03 | 0.9 |
| `vision_suggest` | 20 | Visual suggestion | 0.02 | 0.4 |
| `vision_health` | 20 | Vision system health | 0.02 | 0.4 |
| `vision_link` | 20 | Link visual to memory | 0.05 | 1 |
| `vision_session_resume` | 30 | Resume visual context | 0.03 | 0.9 |
| `vision_workspace_*` (6 tools) | 20 ea | Federation | 0.02 | 2.4 |

**Vision Inventions** (100+ specialized tools — activated by Vision sister when online):

| Category | Tools | Trigger | P | Expected |
|----------|------:|---------|:-:|:--------:|
| `vision_anomaly_*` | 5 | Visual anomaly detected | 0.02 | 2 |
| `vision_archaeology_*` | 5 | Visual history | 0.01 | 1 |
| `vision_attention_*` | 5 | Attention tracking | 0.02 | 2 |
| `vision_composition_*` | 5 | Layout analysis | 0.02 | 2 |
| `vision_forensic_*` | 5 | Visual forensics | 0.01 | 1 |
| `vision_gestalt_*` | 5 | Pattern recognition | 0.02 | 2 |
| `vision_hallucination_*` | 5 | Visual validation | 0.02 | 2 |
| `vision_semantic_*` | 5 | Semantic vision | 0.03 | 3 |
| `vision_temporal` | 3 | Temporal vision | 0.02 | 1.2 |
| Other specialized | 40+ | Specific vision contexts | 0.01 | 8 |

**Vision expected per-message: ~5 tokens** (most tools fire rarely, P < 0.05)

#### Identity Conditional

| Tool | Tokens | Trigger | P(trigger) | Expected |
|------|-------:|---------|:----------:|:--------:|
| `identity_create` | 30 | New identity context | 0.01 | 0.3 |
| `identity_show` | 20 | Identity display | 0.03 | 0.6 |
| `receipt_create` | 20 | Always (LEARN) | 1.0 | 20 |
| `receipt_list` | 20 | Receipt query | 0.03 | 0.6 |
| `receipt_verify` | 20 | Receipt verification | 0.03 | 0.6 |
| `trust_grant` | 20 | Trust grant | 0.02 | 0.4 |
| `trust_list` | 20 | Trust list | 0.03 | 0.6 |
| `trust_revoke` | 20 | Trust revoke | 0.01 | 0.2 |
| `trust_verify` | 20 | Trust verification | 0.05 | 1 |
| `identity_trust` | 20 | Trust query | 0.05 | 1 |
| `identity_trust_damage` | 20 | Trust damage event | 0.02 | 0.4 |
| `identity_trust_history` | 20 | Trust history | 0.03 | 0.6 |
| `identity_trust_infer` | 20 | Infer trust | 0.03 | 0.6 |
| `identity_trust_level` | 20 | Trust level query | 0.05 | 1 |
| `identity_trust_paths` | 20 | Trust path analysis | 0.02 | 0.4 |
| `identity_trust_prevent` | 20 | Prevent trust damage | 0.01 | 0.2 |
| `identity_trust_project` | 20 | Project trust trajectory | 0.02 | 0.4 |
| `identity_trust_prophecy` | 20 | Predict trust changes | 0.02 | 0.4 |
| `identity_trust_recommend` | 20 | Trust recommendations | 0.03 | 0.6 |
| `identity_trust_reinforce` | 20 | Reinforce trust | 0.05 | 1 |
| `identity_trust_warn` | 20 | Trust warning | 0.02 | 0.4 |
| `identity_competence_*` (4 tools) | 20 ea | Competence tracking | 0.03 | 2.4 |
| `identity_resurrect_*` (4 tools) | 20 ea | Identity recovery | 0.01 | 0.8 |
| `identity_fork_*` | 20 ea | Identity forking | 0.01 | 0.4 |
| `identity_team_*` | 20 ea | Team identity | 0.02 | 0.8 |
| `identity_fingerprint_*` | 20 ea | Fingerprint tracking | 0.02 | 0.8 |
| `identity_temporal_*` | 20 ea | Temporal identity | 0.02 | 0.8 |
| `identity_zk_*` | 20 ea | Zero-knowledge proofs | 0.01 | 0.4 |
| `identity_attribute_*` | 20 ea | Attribute management | 0.02 | 0.8 |
| `identity_capability_*` | 20 ea | Capability tracking | 0.02 | 0.8 |
| `identity_consent_*` | 20 ea | Consent management | 0.01 | 0.4 |
| `identity_continuity` | 20 | Continuity check | 0.05 | 1 |
| `identity_actions` | 20 | Action log | 0.05 | 1 |
| `identity_spawn` | 20 | Spawn sub-identity | 0.01 | 0.2 |
| `identity_health` | 20 | Identity health | 0.05 | 1 |
| `identity_workspace_*` (6 tools) | 20 ea | Federation | 0.02 | 2.4 |

#### Memory Inventions (Conditional, P < 0.05 each)

| Category | Tools | Trigger | P | Expected |
|----------|------:|---------|:-:|:--------:|
| `memory_workspace_*` (6 tools) | 6 | Federation active | 0.02 | 2.4 |
| `memory_capture_file` | 1 | File capture | 0.10 | 3 |
| `memory_capture_tool` | 1 | Tool usage capture | 0.10 | 3 |
| `memory_longevity_stats` | 1 | Longevity report | 0.03 | 0.6 |
| Memory INFINITUS tools | ~100 | Invention-specific | 0.01 | ~20 |

---

### Tier 5: Periodic Deep Intelligence (amortized across N messages)

| Tool | Sister | Tokens | Frequency | Amortized/msg |
|------|--------|-------:|-----------|:-------------:|
| `memory_metabolism_process` | Memory | 40 | Every 10 msgs | 4 |
| `memory_meta_gaps` | Memory | 40 | Every 20 msgs | 2 |
| `memory_ghost_write` | Memory | 60 | Every 20 msgs | 3 |
| `cognition_soul_reflect` | Cognition | 30 | Every 5 msgs | 6 |
| `cognition_drift_track` | Cognition | 30 | Every 10 msgs | 3 |
| `evolve_optimize` | Evolve | 40 | Every 10 msgs | 4 |
| `planning_singularity` | Planning | 20 | Every 15 msgs | 1.3 |
| `time_analysis_patterns` | Time | 30 | Session end | ~1 |
| `memory_dream_start` | Memory | 30 | Idle state | ~0.5 |
| `comm_health` | Comm | 20 | Every 30 msgs | 0.7 |

**Amortized periodic cost: ~25 tokens/message**

---

## 2. Mathematical Token Budget

### Expected Cost Per Message

```
E[tokens/msg] = T_always + E[T_complexity] + E[T_code] + E[T_conditional] + E[T_periodic]

Where:
  T_always      = 330 tokens
  E[T_complexity] = 400 × P(complex) = 400 × 0.30 = 120 tokens
  E[T_code]     = 250 × P(code)    = 250 × 0.25 = 63 tokens
  E[T_conditional] = Σ(token_i × P_i) ≈ 200 tokens (see tables above)
  E[T_periodic] = 25 tokens

  E[total] = 330 + 120 + 63 + 200 + 25 = 738 tokens/message
```

### Worst-Case Per Message (complex code task, all conditionals fire)

```
T_max = 330 + 400 + 250 + 600 + 50 = 1,630 tokens
```

### Best-Case Per Message (simple greeting)

```
T_min = 330 + 0 + 0 + 40 + 4 = 374 tokens
```

### Per-Message by Scenario

| Scenario | Tokens | Tools Active | Sister Coverage |
|----------|-------:|:------------:|:---------------:|
| Greeting ("hi") | ~374 | 12 | 5/14 |
| Simple question | ~500 | 18 | 8/14 |
| Complex discussion | ~900 | 35 | 12/14 |
| Code generation | ~1,200 | 50 | 14/14 |
| Code + visual + planning | ~1,630 | 70+ | 14/14 |

### Token Efficiency Ratio

```
Efficiency = Tools_Utilized / Tokens_Spent

Current: 110 tools / 1,200 tokens = 0.092 tools/token
Target:  480 tools / 1,630 tokens = 0.295 tools/token (3.2x improvement)
```

---

## 3. Tool Coverage Summary

| Sister | Total Tools | Blueprint Covered | Coverage |
|--------|:----------:|:-----------------:|:--------:|
| Memory | 45 core + 100 inv | 45 + delegation | **100%** |
| Cognition | 14 | 14 | **100%** |
| Comm | 17 | 17 | **100%** |
| Aegis | 12 | 12 | **100%** |
| Veritas | 10 | 10 | **100%** |
| Evolve | 14 | 14 | **100%** |
| Forge | 15 | 15 | **100%** |
| Reality | 15 | 15 | **100%** |
| Time | 31 | 31 | **100%** |
| Contract | 33 | 33 | **100%** |
| Planning | 27+ | 27 | **100%** |
| Vision | 24 core + 75 inv | 24 + delegation | **100%** |
| Identity | 75+ | 75 | **100%** |
| Codebase | 4 | 4 | **100%** |
| **TOTAL** | **500+** | **500+** | **100%** |

**Strategy for 100% coverage with minimal tokens:**
- Core tools (200): Mapped to explicit trigger conditions with P(trigger)
- Invention tools (300): Delegated to sister internal routing — Hydra calls the invention category facade, sister routes to specific tool internally
- Workspace tools (80): Activated only when federation is active (P=0.02)

---

## 4. Compact Facade Strategy

When `MCP_TOOL_SURFACE=compact`, sisters expose facades that internally route to specific tools:

| Facade | Replaces | Calls Saved | Token Savings |
|--------|:--------:|:-----------:|:-------------:|
| `memory_core` | query, similar, causal, temporal, traverse, context, resolve | 7→1 | ~140 |
| `memory_grounding` | ground, evidence, suggest | 3→1 | ~60 |
| `memory_session` | start, end, resume | 3→1 | ~60 |
| `memory_prophetic` | predict, dejavu_check | 2→1 | ~30 |
| `memory_infinitus` | All 100 invention tools | 100→1 | ~2,000 |
| `comm_session` | start, end, log, context | 4→1 | ~80 |
| `comm_message` | send, receive, search, acknowledge | 4→1 | ~80 |
| `cognition_model` | create, heartbeat, portrait, vitals, update | 5→1 | ~100 |
| `cognition_belief` | add, graph, query, revise | 4→1 | ~80 |
| `aegis_validate` | input, output, complete, streaming | 4→1 | ~80 |
| `vision_core` | capture, ocr, compare, diff, track, query | 6→1 | ~120 |
| `vision_inventions` | All 75 invention tools | 75→1 | ~1,500 |
| `identity_trust` | All 13 trust dynamics tools | 13→1 | ~240 |
| `identity_receipt` | create, list, verify | 3→1 | ~40 |
| `contract_approval` | request, decide, list | 3→1 | ~40 |
| `contract_policy` | add, check, list, query | 4→1 | ~60 |
| `planning_goal` | create, progress, complete, checkpoint | 4→1 | ~60 |
| `time_schedule` | available, conflicts, create, range, reschedule | 5→1 | ~80 |
| `time_deadline` | add, complete, list, overdue | 4→1 | ~60 |

**Total potential savings**: ~40% reduction in tool definition tokens with facades.

---

## 5. Phase-by-Phase Orchestration (Complete)

### SESSION INIT
```
PARALLEL {
  memory_session_resume()        // 80 tokens
  session_start()                // 30
  comm_register_agent()          // 30
  time_session_start()           // 30
  identity_whoami()              // 20
  contract_session_resume()      // 30
  aegis_session_create()         // 20
}
comm_session(start)              // 30
Total: ~270 tokens, one-time
```

### PERCEIVE (every message)
```
ALWAYS {
  memory_query(facts)            // 50
  memory_predict()               // 50
  memory_dejavu_check()          // 30
  cognition_model_query()        // 40
  cognition_belief_query()       // 30
  comm_inbox()                   // 30
  identity_whoami()              // 20  (cached after INIT)
}

IF complex PARALLEL {
  memory_query(general)          // 50
  memory_longevity_search()      // 60
  memory_similar()               // 40
  memory_ground()                // 30
  memory_temporal_recall()       // 40
  reality_context()              // 30
  reality_environment()          // 20
  veritas_detect_ambiguity()     // 30
  policy_query()                 // 20
  goal_query()                   // 30
  planning_commitment()          // 20
  time_deadline_list()           // 20
}

IF code_task PARALLEL {
  search_semantic()              // 60
  concept_find()                 // 40
  impact_analyze()               // 50
  blueprint_query()              // 30
}

IF visual {
  vision_capture()               // 50
  vision_ocr()                   // 40
}

IF "why" question {
  memory_causal()                // 50
}

IF time_query {
  time_stats()                   // 20
  time_search_sessions()         // 30
}
```

### THINK
```
ALWAYS {
  cognition_predict()            // 30
  cognition_detect_drift()       // 30
}

IF complex PARALLEL {
  forge_blueprint_create()       // 80
  veritas_compile_intent()       // 40
  evolve_match_context()         // 30
  evolve_confidence()            // 20
}

IF code_task {
  forge_entity_infer()           // 30
}
```

### DECIDE
```
ALWAYS PARALLEL {
  contract_precognition()        // 30
  contract_policy_check()        // 30
  aegis_check_input()            // 30
}

IF complex {
  veritas_check_uncertainty()    // 30
}

IF has_commands {
  shadow_validate()              // 20  (InventionEngine, local)
}
```

### ACT
```
ALWAYS PARALLEL {
  veritas_score_confidence()     // 30
  veritas_check_consistency()    // 30
  aegis_check_output()           // 40
}

IF complex { self_review() }     // 0 (LLM-based)

IF code_generation PARALLEL {
  forge_skeleton_create()        // 40
  forge_test_generate()          // 40
  forge_dependency_resolve()     // 30
  aegis_validate_complete()      // 40
  aegis_scan_security()          // 40
}

IF shell_commands {
  aegis_shadow_execute()         // 60
  // exec_engine handles: persistent cwd, env, parallel, background
}

IF visual_action {
  vision_capture()               // 50
  vision_diff()                  // 40
}

IF planning_goal {
  planning_checkpoint()          // 20
}

identity_receipt_create()        // 20
contract_crystallize()           // 30
```

### LEARN
```
ALWAYS {
  cognition_belief_revise()      // 30
  cognition_model_update()       // 40
  evolve_record_pattern()        // 20
  time_duration_track()          // 20
  contract_record_decision()     // 30
  identity_receipt_create()      // 20
}

IF capture=all {
  memory_capture_exchange()      // 50
  comm_session(log)              // 30
}

IF correction {
  memory_correct()               // 30
  broadcast_insight()            // 30
}

IF decision_made {
  memory_store_decision()        // 30
  planning_decision()            // 20
}

IF exec_results {
  memory_store_evidence()        // 30
  memory_store_test_results()    // 20
}

IF code_task {
  pattern_extract()              // 30
  evolve_crystallize()           // 20
}

IF complex {
  evolve_suggest_improvement()   // 30
  planning_progress()            // 20
}

PERIODIC {
  IF msg_count % 5  { cognition_soul_reflect() }    // 30
  IF msg_count % 10 { cognition_drift_track() }     // 30
  IF msg_count % 10 { memory_metabolism_process() } // 40
  IF msg_count % 10 { evolve_optimize() }           // 40
  IF msg_count % 15 { planning_singularity() }      // 20
  IF msg_count % 20 { memory_meta_gaps() }          // 40
  IF msg_count % 20 { memory_ghost_write() }        // 60
  IF msg_count % 30 { comm_health() }               // 20
}
```

### SESSION END
```
memory_ghost_write()             // 60
PARALLEL {
  session_end()                  // 30
  comm_session(end)              // 20
  comm_deregister_agent()        // 20
  time_session_end()             // 20
  aegis_session_end()            // 20
  cognition_drift_track()        // 30
  contract_evidence()            // 20
  time_analysis_patterns()       // 30
}
Total: ~250 tokens, one-time
```

---

## 6. Anti-Patterns

1. **Don't call all 500+ tools every message** — Use the tier system. P(trigger) is your friend.
2. **Don't call fire-and-forget tools synchronously** — Use `tokio::join!` for parallel execution.
3. **Don't store greetings in memory** — "hello" and "thanks" pollute the memory graph.
4. **Don't call memory_query twice with same params** — Use `memory_predict` instead.
5. **Don't call expensive tools for simple queries** — Tier system prevents this.
6. **Don't ignore compact facades** — When available, they save 40% on tool definition tokens.
7. **Don't skip session lifecycle** — Resume at start, capture during, close at end. Always.
8. **Don't block execution** — All security layers are warn-only. Hydra can do anything.
9. **Don't second-guess** — No clarification aborts, no confidence gates. Try first, ask later.
10. **Don't cap output** — 256KB buffer, 5-minute timeout. Let commands finish.

---

## 7. Implementation Phases

### Phase A-F: DONE ✅
Session continuity, capture settings, deep cognition, time/planning, session lifecycle, Claude-like intelligence.

### Phase G: Full Sister Exploitation (NEXT)
Wire the ~380 tools that have trigger conditions in this blueprint but no code yet.

**Priority order** (by expected token impact × intelligence gain):
1. Identity trust dynamics (13 tools) — trust tracking across sessions
2. Forge full pipeline (9 tools) — skeleton, test gen, dependency, wiring
3. Aegis full validation (8 tools) — code confidence, rollback, streaming
4. Reality full grounding (12 tools) — deployment, substrate, hallucination
5. Contract full audit (20 tools) — obligations, violations, risk limits
6. Time full temporal (20 tools) — scheduling, sequences, decay
7. Vision full pipeline (20 core tools) — OCR, compare, track, forensics
8. Comm full messaging (10 tools) — federation, hive, forensics
9. Planning full suite (10 tools) — counterfactual, consensus, dream
10. Evolve full patterns (8 tools) — compose, coverage, signatures
11. Memory inventions (100 tools) — delegate to INFINITUS facades
12. Vision inventions (75 tools) — delegate to invention facades
13. Identity advanced (40 tools) — ZK proofs, forking, team identity
14. Workspace tools (80 tools) — activate when federation is live
