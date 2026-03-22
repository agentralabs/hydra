# 15 — The Skill Drop

## You Do Not Need to Code. You Drop a TOML File.

This is the capability that changes the relationship between humans and software forever.

To teach Hydra a new skill, you do not write code. You do not call an API. You do not train a model. You create a folder with two TOML files and drop it into `skills/`. Hydra loads it on next boot. Done.

---

## What a Skill Looks Like

```
skills/
  devops/
    genome.toml       ← proven approaches for DevOps situations
    functor.toml      ← maps DevOps concepts to universal primitives
```

That is it. Two text files. No Rust. No Python. No JavaScript. No compilation. No deployment. No API integration.

---

## genome.toml — What Hydra Knows

Each entry is a situation-approach pair:

```toml
[[entries]]
situation    = "Kubernetes pod keeps crashing with OOMKilled"
approach     = "check resource requests vs limits — requests should be 80% of limits, not 50% — set memory limit to 2x the actual peak usage observed in monitoring"
confidence   = 0.91
observations = 3200
notes        = "Covers 90% of OOMKill cases in production clusters"

[[entries]]
situation    = "CI pipeline takes more than 30 minutes"
approach     = "parallelize test suites first — split by module, not by file — cache dependencies between runs — do not optimize individual test speed until parallel execution is maxed"
confidence   = 0.88
observations = 1500

[[entries]]
situation    = "database migration fails in production but works in staging"
approach     = "check for data-dependent migrations — staging has clean data, production has 5 years of edge cases — run migration against a production data snapshot before deploying"
confidence   = 0.93
observations = 800
```

When someone asks Hydra about a crashing Kubernetes pod, the genome surfaces this approach with 91% confidence from 3,200 observations. The LLM sees it in the system prompt as a proven approach and incorporates it into the response.

---

## functor.toml — How Hydra Understands

Functors map domain-specific words to Hydra's universal axiom primitives:

```toml
[[mappings]]
domain_concept  = "deployment"
axiom_primitive = "Risk"
weight          = 0.85
notes           = "Every deployment carries risk — surface risk context"

[[mappings]]
domain_concept  = "rollback"
axiom_primitive = "Risk"
weight          = 0.90

[[mappings]]
domain_concept  = "terraform"
axiom_primitive = "Dependency"
weight          = 0.80
notes           = "Infrastructure-as-code creates infrastructure dependencies"

[[mappings]]
domain_concept  = "kubernetes"
axiom_primitive = "Dependency"
weight          = 0.75

[[mappings]]
domain_concept  = "monitoring"
axiom_primitive = "Understanding"
weight          = 0.70

[[mappings]]
domain_concept  = "incident"
axiom_primitive = "Risk"
weight          = 0.95
notes           = "Incidents are maximum risk context"
```

When someone says "the Kubernetes deployment caused an incident," Hydra maps:
- "kubernetes" → Dependency (0.75)
- "deployment" → Risk (0.85)
- "incident" → Risk (0.95)

Three primitives fire. The pattern engine checks for anti-patterns matching Risk + Dependency. The oracle projects adverse scenarios. The genome surfaces relevant proven approaches. All of this happens because of a TOML file.

---

## How Skills Load

On boot, the kernel calls `GenomeStore::load_from_skills()`:

1. Scan the `skills/` directory for folders
2. Each folder that contains `genome.toml` is a skill
3. Parse entries from TOML into `GenomeEntry` structs
4. Deduplicate by situation signature (no duplicates)
5. Add to the genome store
6. Entries persist to `genome.db` (SQLite)

```
hydra: skill 'devops' — parsed 15 genome entries
hydra: skill 'security' — parsed 22 genome entries
hydra: skill 'general' — parsed 13 genome entries
hydra: loaded 50 genome entries from skills/
```

Constitutional gating: every skill load is checked against the 7 laws. A skill that violates the constitution is rejected.

---

## Skills Anyone Can Create

You do not need to be an engineer. You need to know your domain.

### A Nurse Creates a Medical Triage Skill

```toml
# skills/triage/genome.toml

[[entries]]
situation    = "patient presenting with chest pain and shortness of breath"
approach     = "IMMEDIATE: 12-lead ECG, troponin, SpO2 — do not wait for attending — this is time-critical — activate STEMI protocol if ST elevation present"
confidence   = 0.97
observations = 50000

[[entries]]
situation    = "patient with fever and confusion in elderly"
approach     = "suspect sepsis until proven otherwise — blood cultures BEFORE antibiotics — lactate level — start broad spectrum within 1 hour of recognition"
confidence   = 0.94
observations = 30000
```

### A Trader Creates a Risk Assessment Skill

```toml
# skills/trading/genome.toml

[[entries]]
situation    = "portfolio concentrated in a single sector above 40%"
approach     = "rebalance to max 25% per sector — concentration above 40% has 3x drawdown risk in sector corrections — diversify across uncorrelated sectors first"
confidence   = 0.89
observations = 15000

[[entries]]
situation    = "volatility index above 30 with portfolio fully invested"
approach     = "raise cash to 20% — high VIX with full exposure is the highest-risk state — reduce first, analyze later — the cost of being wrong about reducing is small, the cost of being wrong about holding is large"
confidence   = 0.91
observations = 8000
```

### A Teacher Creates a Student Assessment Skill

```toml
# skills/assessment/genome.toml

[[entries]]
situation    = "student consistently scores high on tests but low on projects"
approach     = "test for surface learning vs deep understanding — the student may be memorizing patterns without conceptual transfer — assign an unfamiliar problem that requires applying learned concepts to a novel context"
confidence   = 0.86
observations = 5000
```

---

## What Happens After the Drop

```
1. You create skills/devops/genome.toml
2. You restart Hydra (or Hydra hot-reloads on next boot)
3. Hydra: "skill 'devops' — parsed 15 genome entries"
4. Someone asks: "My pods keep crashing"
5. Genome IDF scoring: "crashing" + "pods" → matches "Kubernetes pod keeps crashing with OOMKilled"
6. Proven approach surfaces in the LLM prompt:
   "PROVEN APPROACHES — You MUST incorporate:
    check resource requests vs limits (conf=91%, obs=3200)"
7. The LLM incorporates this into its response
8. The response is better than any LLM could produce alone
   because it contains real operational experience
```

No code was written. No model was trained. No API was called. A text file was dropped into a folder. Hydra became an expert.

---

## Why This Matters

Every organization in the world has operational knowledge trapped in:
- People's heads (leaves when they leave)
- Runbooks that nobody reads (stale within months)
- Slack threads that nobody can find (buried in noise)
- Documentation that was accurate once (then the system changed)

Skills files are different because:
- They are **structured** (situation → approach, not prose)
- They have **confidence scores** (not all advice is equal)
- They are **Bayesian** (confidence updates with real use)
- They are **searchable by relevance** (IDF, not keyword match)
- They are **permanent** (genome store is append-only)
- They are **version-controlled** (TOML files in git)

Drop a file. Hydra learns. Forever.

---

## The Social Media Version

```
To teach ChatGPT something, you paste it into every conversation.
To teach Hydra something, you drop a TOML file once.

ChatGPT forgets next conversation.
Hydra remembers for 20 years.

ChatGPT treats all advice equally.
Hydra tracks confidence with Bayesian statistics.

ChatGPT cannot tell you why it is confident.
Hydra says: "91% confidence from 3,200 observations."

The file is 10 lines of text.
No code. No training. No API.
Drop and go.
```
