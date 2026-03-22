# 17 — Skill Registry

## What Hydra Knows Today and What It Still Needs

This is the living checklist. Every skill domain Hydra has, every one it needs, and what is missing. Come back here, tick what is done, add what is new.

---

## HOW TO READ THIS

```
[x] = skill exists with genome.toml entries
[F] = functor.toml also exists (maps domain words to axiom primitives)
[ ] = skill directory exists but empty (needs entries)
[-] = skill does not exist yet (needs directory + genome.toml)

Target: every skill should have 10+ entries and a functor.toml
        entries below 5 are stubs, not real skills
```

---

## ENGINEERING & TECHNOLOGY

| Status | Skill | Entries | Functor | Notes |
|--------|-------|---------|---------|-------|
| [x][F] | general | 13 | yes | Core engineering patterns — circuit breaker, interface-first, measure-first |
| [x][F] | devops | 10 | yes | K8s, CI/CD, deployment, monitoring, secrets, Docker |
| [x][F] | security | 10 | yes | SQL injection, auth, XSS, CORS, encryption, mTLS |
| [x] | architecture | 10 | **needs functor** | Microservices, scaling, event-driven, caching, sagas |
| [x] | coding | 8 | **needs functor** | README-first, testing, error handling, git, language choice |
| [x] | data-science | 5 | **needs functor** | Data cleaning, A/B testing, model overfitting, dashboards |
| [ ] | debugging | 0 | no | **EMPTY — needs entries**: reproduce-first, binary search, logging, profiling |
| [-] | databases | - | - | **MISSING**: indexing, query optimization, migrations, replication, backup |
| [-] | networking | - | - | **MISSING**: DNS, load balancing, CDN, TCP tuning, TLS |
| [-] | cloud | - | - | **MISSING**: AWS/GCP/Azure patterns, cost optimization, IAM, regions |
| [-] | mobile | - | - | **MISSING**: iOS/Android patterns, responsive design, offline-first |
| [-] | ai-ml | - | - | **MISSING**: prompt engineering, fine-tuning, RAG, embeddings, evaluation |
| [-] | api-design | - | - | **MISSING**: REST conventions, versioning, pagination, rate limiting, docs |
| [-] | testing | - | - | **MISSING**: TDD, property-based, load testing, chaos engineering, mocking |

## BUSINESS & FINANCE

| Status | Skill | Entries | Functor | Notes |
|--------|-------|---------|---------|-------|
| [x] | business | 7 | **needs functor** | Product-market fit, hiring, deadlines, build-vs-buy, churn |
| [x] | finance | 5 | **needs functor** | Investing, budgeting, pricing, debt vs investing, runway |
| [x] | management | 5 | **needs functor** | Underperformance, delegation, conflict, morale, new manager |
| [x] | legal | 5 | **needs functor** | Contracts, IP, privacy/GDPR, cease-and-desist, entity formation |
| [-] | marketing | - | - | **MISSING**: positioning, content strategy, SEO, analytics, brand |
| [-] | sales | - | - | **MISSING**: qualification, objection handling, pipeline, pricing psychology |
| [-] | strategy | - | - | **MISSING**: competitive analysis, market sizing, moats, pivoting |
| [-] | operations | - | - | **MISSING**: process design, supply chain, quality control, scaling ops |
| [-] | fundraising | - | - | **MISSING**: pitch decks, term sheets, valuation, investor relations |
| [-] | accounting | - | - | **MISSING**: cash flow, P&L, tax planning, audit preparation |

## SCIENCE & RESEARCH

| Status | Skill | Entries | Functor | Notes |
|--------|-------|---------|---------|-------|
| [x] | science | 5 | **needs functor** | Experiment design, p-values, correlation/causation, outliers, ML models |
| [x] | research | 5 | **needs functor** | Literature review, credibility, negative results, public communication |
| [-] | statistics | - | - | **MISSING**: Bayesian vs frequentist, confidence intervals, regression, sampling |
| [-] | physics | - | - | **MISSING**: estimation, dimensional analysis, conservation laws, modeling |
| [-] | biology | - | - | **MISSING**: experimental controls, bioethics, genomics basics, clinical trials |
| [-] | mathematics | - | - | **MISSING**: proof strategies, approximation, modeling, when exact vs numeric |

## HUMAN SKILLS

| Status | Skill | Entries | Functor | Notes |
|--------|-------|---------|---------|-------|
| [x] | communication | 5 | **needs functor** | Feedback, presenting, conflict resolution, saying no, documentation |
| [x] | productivity | 5 | **needs functor** | Prioritization, procrastination, meetings, context switching, habits |
| [x] | learning | 5 | **needs functor** | Feynman technique, spaced repetition, plateaus, language learning |
| [x] | education | 5 | **needs functor** | Student struggles, abstract concepts, engagement, assessment, curriculum |
| [x] | humanities | 5 | **needs functor** | Ethics frameworks, history analysis, logical arguments, cross-cultural |
| [x] | health | 5 | **needs functor** | Burnout, exercise habits, sleep, desk ergonomics, anxiety |
| [-] | relationships | - | - | **MISSING**: active listening, trust building, boundary setting, conflict |
| [-] | negotiation | - | - | **MISSING**: BATNA, anchoring, win-win framing, silence as tool |
| [-] | leadership | - | - | **MISSING**: vision setting, influence without authority, decision fatigue |
| [-] | parenting | - | - | **MISSING**: age-appropriate expectations, consistency, emotional regulation |
| [-] | public-speaking | - | - | **MISSING**: structure, nerves, audience reading, storytelling, Q&A |

## CREATIVE & DESIGN

| Status | Skill | Entries | Functor | Notes |
|--------|-------|---------|---------|-------|
| [x] | design | 5 | **needs functor** | UI goals, user confusion, simplicity, accessibility, brand identity |
| [-] | writing | - | - | **MISSING**: structure, editing, voice, audience awareness, deadlines |
| [-] | storytelling | - | - | **MISSING**: narrative arc, tension, character, show-don't-tell, hook |
| [-] | music | - | - | **MISSING**: practice structure, ear training, composition, performance |
| [-] | photography | - | - | **MISSING**: composition, lighting, editing workflow, gear decisions |

## DOMAIN-SPECIFIC (ADD AS NEEDED)

| Status | Skill | Entries | Functor | Notes |
|--------|-------|---------|---------|-------|
| [-] | healthcare | - | - | **MISSING**: clinical decision-making, patient communication, triage |
| [-] | real-estate | - | - | **MISSING**: valuation, market analysis, negotiation, due diligence |
| [-] | agriculture | - | - | **MISSING**: crop planning, soil management, irrigation, market timing |
| [-] | journalism | - | - | **MISSING**: source verification, story structure, ethics, deadlines |
| [-] | nonprofit | - | - | **MISSING**: grant writing, donor relations, impact measurement, governance |
| [-] | manufacturing | - | - | **MISSING**: lean, quality control, supply chain, automation, safety |
| [-] | energy | - | - | **MISSING**: renewables integration, grid management, efficiency, storage |
| [-] | transportation | - | - | **MISSING**: logistics optimization, fleet management, routing, compliance |

---

## PRIORITY ORDER FOR FILLING

```
IMMEDIATE (fill during 30-day protocol):
  1. debugging        — 0 entries, core engineering skill
  2. Add functors to: architecture, coding, business, finance,
                      management, communication, productivity,
                      learning, science, research, data-science,
                      design, education, humanities, health, legal

MONTH 1:
  3. databases        — every engineer needs this
  4. ai-ml            — the domain Hydra itself operates in
  5. testing          — directly improves code quality advice
  6. api-design       — most common engineering task
  7. negotiation      — universal life skill
  8. writing          — universal communication skill

MONTH 2:
  9. cloud            — DevOps extension
  10. marketing       — business extension
  11. statistics      — science extension
  12. leadership      — management extension
  13. strategy        — business extension

MONTH 3+:
  Fill domain-specific skills based on actual usage patterns.
  The genome query logs will show which domains users ask about
  most. Fill those first.
```

---

## HOW TO ADD A SKILL

```bash
# 1. Create the directory
mkdir skills/databases

# 2. Create genome.toml with 5-10 entries
cat > skills/databases/genome.toml << 'EOF'
[[entries]]
situation    = "database query is slow"
approach     = "EXPLAIN the query first — look for sequential scans on large tables — add indexes on columns in WHERE and JOIN clauses — measure before and after"
confidence   = 0.92
observations = 10000
EOF

# 3. Create functor.toml mapping domain words to primitives
cat > skills/databases/functor.toml << 'EOF'
[[mappings]]
domain_concept  = "index"
axiom_primitive = "Volume"
weight          = 0.75
EOF

# 4. Restart Hydra (or it loads on next boot)
# That is it. Hydra now knows about databases.
```

---

## SCORECARD

```
Current:     123 entries across 20 skills (3 with functors)
Target:      500+ entries across 40+ skills (all with functors)
Gap:         377 entries, 20+ new skills, 17 missing functors

Skills with entries: 19/20 (debugging is empty)
Skills with functors: 3/20 (general, devops, security)
Skills needed:        25+ domains not yet created
```

---

*This registry is the single most impactful document in the catalogue.*
*Every entry added here makes Hydra measurably smarter.*
*No code required. Just TOML. Drop and go.*
