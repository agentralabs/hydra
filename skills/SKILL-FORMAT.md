# How to Create a Skill for Hydra

## For a Business, Team, or Individual

Create a folder in `skills/` with your organization's name. Add any of these files:

```
skills/
  your-company/
    genome.toml         ← REQUIRED: proven approaches (situation → approach)
    functor.toml        ← RECOMMENDED: maps your domain words to axiom primitives
    context.md          ← OPTIONAL: background context Hydra should always know
    design-system.md    ← OPTIONAL: brand guidelines, colors, typography
    operations.md       ← OPTIONAL: SOPs, processes, escalation paths
    glossary.md         ← OPTIONAL: company-specific terms and meanings
```

## genome.toml — What Your Organization Knows

This is the most important file. Every entry is a situation your team encounters and the proven approach to handle it.

```toml
[[entries]]
situation    = "customer calls about a billing error"
approach     = "apologize first, investigate second — pull up their account, check last 3 invoices, if overcharge confirmed, issue credit immediately, do not make them wait for manager approval — log the error type for pattern analysis"
confidence   = 0.92
observations = 5000
notes        = "From 5 years of customer service data"

[[entries]]
situation    = "new employee needs access to production systems"
approach     = "principle of least privilege — start with read-only access to staging, after 2 weeks add dev environment, after 30 days production read-only with manager approval, never give production write access in the first 90 days"
confidence   = 0.90
observations = 2000

[[entries]]
situation    = "client requests a feature that conflicts with our roadmap"
approach     = "acknowledge the request genuinely — explain what we ARE building and why — show how our roadmap addresses the underlying need differently — if they insist, log it as a roadmap input and revisit quarterly — never say no, say 'not yet, and here is what we are doing instead'"
confidence   = 0.88
observations = 1500
```

## functor.toml — Teaching Hydra Your Language

Maps your company's domain terms to universal axiom primitives so Hydra's pattern matching, oracle, and red-team systems understand your context.

```toml
[[mappings]]
domain_concept  = "churn"
axiom_primitive = "Risk"
weight          = 0.90
notes           = "Customer churn is our highest business risk"

[[mappings]]
domain_concept  = "deployment"
axiom_primitive = "Risk"
weight          = 0.85

[[mappings]]
domain_concept  = "onboarding"
axiom_primitive = "Dependency"
weight          = 0.70
notes           = "New hire productivity depends on onboarding quality"

[[mappings]]
domain_concept  = "ARR"
axiom_primitive = "Volume"
weight          = 0.80
notes           = "Annual Recurring Revenue — our primary growth metric"
```

## context.md — What Hydra Should Always Know

Background context that informs every response. Hydra reads this on boot and keeps it in mind.

```markdown
# Acme Corp Context

We are a B2B SaaS company serving mid-market retailers (500-5000 employees).
Our product is an inventory management platform.
Our primary metric is ARR (Annual Recurring Revenue), currently $4.2M.
We have 47 employees across engineering (18), sales (12), customer success (8),
marketing (5), and operations (4).

Key priorities this quarter:
1. Reduce churn from 4.2% to below 3%
2. Launch the forecasting module (Q2 target)
3. Hire 3 senior engineers

Our competitors: RetailFlow, StockSync, InventoryHub.
Our differentiator: real-time multi-location sync (competitors batch every 15 min).
```

## design-system.md — Brand Guidelines

```markdown
# Acme Corp Design System

Primary color: #2563EB (blue)
Secondary: #10B981 (green)
Accent: #F59E0B (amber)
Background: #FFFFFF (light) / #0F172A (dark)

Typography:
  Headings: Inter, 700 weight
  Body: Inter, 400 weight
  Code: JetBrains Mono

Logo usage:
  Minimum size: 32px height
  Clear space: 1x logo height on all sides
  Never stretch, rotate, or recolor

Tone of voice:
  Professional but approachable
  Technical but not jargon-heavy
  Confident but never arrogant
```

## operations.md — Standard Procedures

```markdown
# Operations Manual

## Incident Response
1. Severity 1 (data loss, full outage): page on-call → 15 min response → war room
2. Severity 2 (degraded service): Slack alert → 30 min response → incident channel
3. Severity 3 (minor issue): ticket → next business day

## Deployment Process
1. PR approved by 2 reviewers
2. CI passes (all tests green)
3. Staging deploy + 1 hour soak
4. Production canary (5% traffic, 30 min)
5. Full rollout

## Escalation Path
Customer Success → Engineering Lead → CTO → CEO
Each level has 2 hours to respond before auto-escalation.
```

## glossary.md — Company Dictionary

```markdown
# Glossary

- **ARR**: Annual Recurring Revenue
- **MRR**: Monthly Recurring Revenue (ARR / 12)
- **NRR**: Net Revenue Retention (expansion - churn)
- **CSM**: Customer Success Manager
- **POC**: Proof of Concept (free trial for enterprise)
- **SKU**: Stock Keeping Unit (inventory item identifier)
- **Multi-loc**: Multi-location sync (our core feature)
- **Batch gap**: The 15-minute delay our competitors have
```

## What Happens After You Drop the Files

```
1. Restart Hydra
2. Hydra loads genome.toml → "skill 'acme-corp' — parsed 15 genome entries"
3. Hydra loads functor.toml → company terms map to axiom primitives
4. Hydra reads context.md → background knowledge always available
5. Hydra reads design-system.md → knows your brand
6. Hydra reads operations.md → knows your procedures
7. Hydra reads glossary.md → speaks your language

Now:
  "How should I handle a customer billing complaint?"
  → Genome hit: "apologize first, investigate second..."

  "Create a carousel about our forecasting module"
  → Design system: uses #2563EB blue, Inter font
  → Context: knows Q2 target, mid-market retailers
  → Content creation genome: 10-slide structure, hook first

  "A severity 1 incident just happened"
  → Operations: "page on-call → 15 min → war room"
  → Genome: proven incident response approaches
  → Alert action: fires notification to you

Hydra becomes your company's brain in one folder drop.
```
