# SKILL FORMAT SPECIFICATION
## The Hydra Skill Package Standard
**Version:** 3.0
**Date:** March 2026
**Classification:** Agentra Labs — Engineering Canonical
**Supersedes:** SKILL-FORMAT-SPEC-V2.md
**Changes from v2.0:**
  - Part 18: Updated crate dependency list (removed hydra-ledger)
  - Part 19: Cognitive loop integration (how skills interact with PROMPT-1 wiring)
  - Part 20: Layer 6/7 skill context (federated skill discovery, influence publication)
  - Updated requires.layer_1_crates (reflects actual built crates)
  - Corrected all crate names against confirmed 66-crate list

---

## PREAMBLE

```
A skill is a package that makes Hydra capable in a new domain
without touching the core.

When the agentra-settlement skill loads:
  Hydra comprehends settlement domain language.
  Hydra can execute settlement actions.
  Hydra knows the operator trust requirements.
  Hydra has initial genome entries for settlement patterns.

Nothing in the core changed.
A folder was dropped. Hydra became a settlement operator.

This spec defines:
  What is in that folder.
  How every piece registers.
  What tier of integration this skill represents.
  What Layer 1 trust configuration is required.
  What hydra-skills does when it loads it.
  How the cognitive loop uses the skill.
  How Layer 6 federation can discover and share skills.
  How Layer 7 influence can publish skill patterns.
  What hydra-automation outputs when it generates one.

The generated format and the hand-written format are IDENTICAL.
```

---

## PART 1: THE SKILL PACKAGE STRUCTURE

```
skills/
└── agentra-settlement/
    ├── skill.toml              ← manifest (required)
    ├── vocabulary.toml         ← domain vocabulary (required)
    ├── functor.toml            ← axiom mappings (required)
    ├── actions.toml            ← action primitives (required)
    ├── environment.toml        ← runtime requirements (required)
    ├── genome.toml             ← initial genome seeds (optional)
    ├── persona.toml            ← persona template (optional)
    ├── panels.toml             ← TUI panel declarations (optional)
    └── trust.toml              ← operator trust configuration
                                   REQUIRED for operator-tier skills
```

```
REQUIRED FILES: skill.toml, vocabulary.toml, functor.toml,
                actions.toml, environment.toml

OPTIONAL FILES: genome.toml, persona.toml, panels.toml

trust.toml is optional in format but MANDATORY in practice
for operator-tier skills. Without it:
  hydra-skills logs a warning.
  Skill loads in observer tier — no execution authority.
  All action calls return CapabilityUnavailable until upgraded.

File format: TOML throughout.
No code. No binaries. No compiled artifacts.
Skills are data, not code. Hydra's existing code interprets the data.
```

---

## PART 2: THE MANIFEST — `skill.toml`

```toml
[skill]
name        = "agentra-settlement"
version     = "0.1.0"
description = "Settlement execution for the Agentra platform."
author      = "Agentra Labs"
domains     = ["fintech", "settlement", "payments"]
tier        = "operator"

[requires]
# Minimum layers required — skill loads gracefully if missing,
# but with reduced capability.
layers = [1, 2, 3, 4, 5]

# CORRECTED: hydra-ledger does not exist.
# Ledger functionality is in hydra-audit and hydra-settlement.
layer_1_crates = ["hydra-genome", "hydra-trust", "hydra-axiom"]
layer_2_crates = ["hydra-comprehension", "hydra-language", "hydra-reasoning"]
layer_3_crates = ["hydra-executor", "hydra-audit"]
layer_4_crates = ["hydra-wisdom", "hydra-calibration"]
layer_5_crates = ["hydra-settlement", "hydra-exchange"]

[provides]
capabilities = [
    "settlement.execute",
    "settlement.flag_dispute",
    "settlement.pause_flow",
    "settlement.query_status",
]

[load]
priority    = 100      # higher = loads earlier
hot_reload  = true     # supports live reload without restart
auto_unload = false    # never auto-unloaded
```

---

## PART 3: VOCABULARY — `vocabulary.toml`

```toml
[domain]
name        = "fintech"
description = "Financial settlement and payments domain."

[[keywords]]
word = "settlement"
weight = 1.0
[[keywords]]
word = "execute settlement"
weight = 1.0
[[keywords]]
word = "batch"
weight = 0.8
[[keywords]]
word = "idempotency"
weight = 0.9
[[keywords]]
word = "payment"
weight = 0.7

[[phrases]]
phrase = "execute settlement batch"
maps_to = "settlement.execute"
confidence = 0.95

[[phrases]]
phrase = "flag this settlement"
maps_to = "settlement.flag_dispute"
confidence = 0.90

[[phrases]]
phrase = "pause settlement flow"
maps_to = "settlement.pause_flow"
confidence = 0.88
```

---

## PART 4: AXIOM MAPPINGS — `functor.toml`

```toml
# Maps domain language onto Hydra's axiom primitives.
# These are the AxiomPrimitive values that hydra-axiom and
# hydra-pattern will see when this domain is active.

[[mappings]]
domain_concept   = "settlement execution"
axiom_primitive  = "Risk"
weight           = 0.85
notes            = "Settlement carries financial risk — redteam always runs"

[[mappings]]
domain_concept   = "idempotency key"
axiom_primitive  = "Dependency"
weight           = 0.70
notes            = "Settlement depends on unique key for deduplication"

[[mappings]]
domain_concept   = "batch processing"
axiom_primitive  = "Volume"
weight           = 0.75

[[mappings]]
domain_concept   = "dispute"
axiom_primitive  = "Constraint"
weight           = 0.90
notes            = "Disputes impose hard constraints on further settlement"
```

---

## PART 5: ACTIONS — `actions.toml`

```toml
# Action primitives this skill provides.
# These map to executor approach types.
# trust_required: minimum wisdom confidence for operator actions.

[[actions]]
id               = "settlement.execute"
description      = "Execute a settlement batch."
approach_type    = "DirectExecution"
reversible       = false
receipted        = true       # write-ahead receipt ALWAYS
trust_required   = 0.75       # operator gate: see trust.toml
escalation_value = 10000.0    # above this amount → escalate

[[actions]]
id               = "settlement.flag_dispute"
description      = "Flag a settlement for manual review."
approach_type    = "DirectExecution"
reversible       = true
receipted        = true
trust_required   = 0.65

[[actions]]
id               = "settlement.pause_flow"
description      = "Pause the settlement processing pipeline."
approach_type    = "StateModification"
reversible       = true
receipted        = true
trust_required   = 0.80

[[actions]]
id               = "settlement.query_status"
description      = "Query the status of a settlement."
approach_type    = "Query"
reversible       = true
receipted        = false      # read-only actions optionally un-receipted
trust_required   = 0.0        # no trust gate for queries
```

---

## PART 6: ENVIRONMENT — `environment.toml`

```toml
[runtime]
# What environment does this skill need?
# hydra-environment validates these on load.

[[requires]]
resource = "network"
description = "Agentra settlement API endpoint."
optional = false

[[requires]]
resource = "env_var"
name = "AGENTRA_SETTLEMENT_API_KEY"
description = "Authentication key for settlement API."
optional = false

[[requires]]
resource = "env_var"
name = "AGENTRA_SETTLEMENT_ENDPOINT"
description = "Settlement API base URL."
optional = false
default = "https://settlement.agentra.io/v1"

[constraints]
max_concurrent_actions = 3
retry_budget = 5
timeout_ms = 30000
```

---

## PART 7: GENOME SEEDS — `genome.toml`

```toml
# Initial genome entries for this domain.
# These seed GenomeStore on first load.
# They will be overwritten over time by real observed approaches.

[[entries]]
situation   = "settlement batch processing with idempotency key"
approach    = "generate idempotency key = hash(batch_id + timestamp + amount) before executing"
confidence  = 0.92
observations = 847000
notes       = "3yr production data. Zero duplicate settlements."

[[entries]]
situation   = "settlement credential approaching expiry"
approach    = "pre-provision fresh credentials at least 24h before settlement window opens"
confidence  = 0.89
observations = 450

[[entries]]
situation   = "settlement amount exceeds $10,000"
approach    = "surface to principal for review before executing — do not auto-execute"
confidence  = 0.99
observations = 1200
notes       = "Constitutional trust threshold. Non-negotiable."
```

---

## PART 8: PERSONA — `persona.toml`

```toml
[persona]
voice_style  = "precise"
tone         = "professional"
domain_register = "financial"

[[vocabulary_overrides]]
# Override Hydra's default phrasing for this domain
when_uncertain   = "This settlement amount requires verification before I proceed."
when_successful  = "Settlement executed. Receipt: {receipt_id}."
when_blocked     = "Settlement paused. Reason: {reason}. Principal review required."
```

---

## PART 9: TUI PANELS — `panels.toml`

```toml
[[panels]]
id          = "settlement-status"
title       = "Settlement Pipeline"
position    = "right"
width       = 40
refresh_ms  = 5000
data_source = "settlement.query_status"

[[panels]]
id          = "settlement-ledger"
title       = "Recent Settlements"
position    = "bottom"
height      = 10
refresh_ms  = 10000
data_source = "settlement.recent_records"
```

---

## PART 10: THE HOT-LOAD PROTOCOL

```
When a skill folder is dropped into skills/:

STEP 1:  hydra-skills detects new directory (inotify/FSEvents)
STEP 2:  Parse skill.toml → validate required fields
STEP 3:  Load vocabulary.toml → register with ComprehensionEngine
STEP 4:  Load functor.toml → register AxiomPrimitive mappings
         (used by hydra-pattern and hydra-reasoning)
STEP 5:  Load actions.toml → register with hydra-executor
STEP 6:  Load environment.toml → validate with hydra-environment
STEP 7:  Load genome.toml (if present) → seed GenomeStore
STEP 8:  Load persona.toml (if present) → register with hydra-persona
STEP 9:  Load panels.toml (if present) → register with hydra-tui
STEP 10: Load trust.toml (if present) → apply trust configuration
         If operator tier and no trust.toml → downgrade to observer,
         log warning, continue loading
STEP 11: Mark skill as loaded
STEP 12: Broadcast SkillLoaded signal to hydra-companion

Total time: < 100ms for all 8 steps.
No restart required. No core changes.
```

---

## PART 11: THE TRUST CONFIGURATION — `trust.toml`

```toml
# trust.toml — REQUIRED for operator-tier skills.
# Defines the constitutional constraints on what this skill
# is permitted to do on Hydra's behalf.

[trust]
tier = "operator"

# Minimum wisdom confidence before ANY operator action runs.
# hydra-wisdom::synthesize() is called. If confidence < this: STOP.
min_wisdom_confidence = 0.75

# Actions permitted under this trust configuration.
permitted_actions = [
    "settlement.execute",
    "settlement.flag_dispute",
    "settlement.pause_flow",
    "settlement.query_status",
]

# Actions explicitly blocked — overrides permitted_actions.
blocked_actions = []

# Escalation threshold: value above which principal review is required.
# Maps to InfluenceError::EscalationRequired in hydra-exchange.
[escalation]
threshold_value = 10000.0
threshold_currency = "USD"
escalation_message = "Settlement amount exceeds $10,000. Principal review required before execution."

# Suspension conditions — Hydra auto-suspends the skill if:
[suspension]
consecutive_failures = 3   # 3 failures in a row → suspend
failure_window_ms = 300000 # within 5 minutes
resume_condition = "manual" # only principal can resume

# Audit retention — all operator actions audited for this long.
[audit]
retention_days = 2555  # 7 years (financial record requirement)
```

---

## PART 12: HOW THE COGNITIVE LOOP USES SKILLS

```
With PROMPT-1 (cognitive loop wiring) active:

STEP 1: Input arrives at CognitiveLoop::cycle()

STEP 2: Perceiver runs hydra-comprehension::comprehend()
        The vocabulary.toml entries are now registered —
        "settlement" triggers Domain::Finance
        ComprehendedInput.primary_domain = Finance

STEP 3: Router checks domain → skill loaded for this domain?
        If skill with matching domain is loaded AND
        comprehension confidence >= 0.60:
          PatternEngine gets ComprehendedInput.primitives
          Primitives include Risk (from functor.toml mapping)
          If pattern warnings found → PromptBuilder adds them

STEP 4: If action is identified:
        Before execution: trust gate checks wisdom confidence
        If < min_wisdom_confidence → surface to principal
        If >= threshold → executor.execute_skill_action()

STEP 5: After execution:
        SettlementEngine::settle_skill_action() records cost
        AuditEngine::audit_manual() writes receipt
        SoulEngine::record_exchange() updates orientation
```

---

## PART 13: INTEGRATION TIERS

```
TIER 1: READER
  Read-only. No side effects. No trust.toml needed.
  Examples: Cosmost (query world state), analytics tools.
  trust.toml: not required
  Actions: query-only (receipted=false permitted)
  Escalation: none

TIER 2: OBSERVER
  Monitor + signal. No writes to external systems.
  Can write to Hydra's internal state (genome, memory).
  trust.toml: optional but recommended
  Actions: state reads + internal writes
  Escalation: none

TIER 3: OPERATOR
  Full execution authority. Writes to external systems.
  trust.toml: REQUIRED (skill loads in observer tier without it)
  Actions: all action types
  Escalation: configurable threshold
  Suspension: configurable consecutive-failure trigger

DEFAULT TIER (if tier omitted from skill.toml): observer
```

---

## PART 14: AGENTRA SYSTEM INTEGRATION GUIDE

```
For each Agentra system that Hydra will control:

SETTLEMENT SYSTEM (already built):
  Tier: operator
  Trust.toml: yes — $10K escalation threshold
  3-failure suspension within 5 minutes
  Genome seeds: 3 seeded (idempotency key pattern highest)

SUPPLY INFRASTRUCTURE (future):
  Tier: observer first → operator later (phased)
  Phase 1: observer — monitor stock, signal anomalies
  Phase 2: operator — reorder, adjust levels
  Trust.toml: added in Phase 2

COSMOST (future):
  Tier: reader
  No trust.toml needed
  Query world state, feed into hydra-noticing

PATTERN:
  Every new Agentra system = one skill package.
  Start at observer tier.
  Observe and feed genome for 30 days.
  Then promote to operator tier with trust.toml.
  No core changes required either time.
```

---

## PART 15: LAYER 6 — FEDERATED SKILL DISCOVERY

```
After Layer 6 (hydra-federation) is active:

A peer Hydra can OFFER a skill it has mastered:
  OfferKind::SkillExecution { skill_name: "cobol-migration" }
  This is published via hydra-exchange.

Another Hydra can DISCOVER and REQUEST that skill:
  DiscoveryQuery { domain: "migration" } → finds the offer
  Trust gate: requesting peer trust score >= 0.65
  On approval: skill action executed on behalf of requester

Skill sharing across federation = the first step toward
distributed capability. Not "downloading" the skill.
The capability is executed by the owning instance.
Provenance preserved. The owner's genome is not transferred.
```

---

## PART 16: LAYER 7 — SKILL PATTERNS AS INFLUENCE

```
After Layer 7 (hydra-influence) is active:

Proven skill patterns can be published to the influence network:

InfluenceEngine::publish(
  source_lineage: "hydra-agentra-lineage",
  title: "COBOL Soul Extraction — Enterprise Standard",
  category: PatternCategory::Migration,
  domain_tags: ["cobol", "enterprise"],
  evidence_count: 23,    // must be >= 5
  confidence: 0.94,      // must be >= 0.70
  source_days: 7300,     // 20yr operational history
)

Other instances discover this pattern.
They evaluate it via their own wisdom engine.
They integrate into their genome with provenance.
They report outcomes back — confidence grows.

Skill patterns → influence patterns → standards.
Not documentation. Not training data.
Proven operational intelligence, outcome-tracked.
```

---

## PART 17: WHAT hydra-automation GENERATES

```
When hydra-automation crystallizes a skill from observed behavior:

DETECTION (3 observations → proposal):
  Observed 3 times: user asks about settlement status
  Pattern detected: "settlement-status-query" behavior
  Proposal generated with confidence = 0.72

PROPOSAL FORMAT (identical to hand-written):
  skill.toml     — generated from observed context
  vocabulary.toml — keywords extracted from observed intents
  functor.toml   — axiom mappings inferred from ComprehendedInput
  actions.toml   — action IDs from observed executor calls
  environment.toml — requirements from observed environment

USER REVIEWS PROPOSAL:
  Hydra: "I've noticed you check settlement status 3 times.
          Want me to make this a skill? [approve/modify/reject]"

ON APPROVAL:
  Skill package written to skills/ directory.
  hydra-skills hot-loads it.
  Next time: recognized, receipted, genome-updated.

THE IDENTITY:
  Hydra-generated skill = hand-written skill.
  Same format. Same loading. Same audit trail.
  The generation is invisible to anything that consumes the skill.
```

---

## PART 18: FILE COUNT AND SIZE STANDARDS

```
Every skill package:
  Required files: 5
  Optional files: 4 (including trust.toml)
  Maximum total: 9 files

No file in a skill package should exceed 200 lines of TOML.
If a vocabulary.toml approaches 200 lines:
  Split into multiple domain sections.
  Or use multiple vocabulary files (vocabulary-primary.toml,
  vocabulary-extended.toml) if hydra-skills supports it.

Genome seeds: maximum 20 entries per genome.toml.
  More than 20 seeds = too much pre-loading.
  Let the genome grow from real observations.
  Seeds are starting points, not complete libraries.

Actions: maximum 20 actions per actions.toml.
  More than 20 = skill scope is too broad.
  Split into two skills with separate concerns.
```

---

## APPENDIX: COMPLETE FILE REFERENCE

```
FILE               REQUIRED  PURPOSE
──────────────────────────────────────────────────────────────────
skill.toml         YES       Identity, tier, dependencies, provides
vocabulary.toml    YES       Domain language → ComprehensionEngine
functor.toml       YES       Domain concepts → AxiomPrimitive map
actions.toml       YES       Action primitives → hydra-executor
environment.toml   YES       Runtime requirements → hydra-environment
genome.toml        NO        Initial genome seeds → GenomeStore
persona.toml       NO        Voice and tone → hydra-persona
panels.toml        NO        TUI panels → hydra-tui
trust.toml         NO*       Operator constraints → trust gate
                             *Required in practice for operator tier

CRATES THAT USE EACH FILE:
  skill.toml      → hydra-skills (loader), hydra-reflexive (self-model)
  vocabulary.toml → hydra-comprehension, hydra-language
  functor.toml    → hydra-axiom, hydra-pattern, hydra-reasoning
  actions.toml    → hydra-executor, hydra-audit
  environment.toml→ hydra-environment, hydra-plastic
  genome.toml     → hydra-genome (GenomeStore.add())
  persona.toml    → hydra-persona, hydra-soul
  panels.toml     → hydra-tui
  trust.toml      → hydra-wisdom (confidence gate),
                    hydra-exchange (escalation threshold),
                    hydra-settlement (operator authority)
```

---

*SKILL FORMAT SPECIFICATION — VERSION 3*
*The Hydra Skill Package Standard*
*Updated for 66-crate architecture and PROMPT-1 cognitive loop*
*Agentra Labs — March 2026 — Engineering Canonical*
