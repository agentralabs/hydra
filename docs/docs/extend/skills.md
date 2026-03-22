---
title: "Skills"
description: "Teach Hydra new domains with genome.toml and functor.toml -- no code, no compilation, just TOML."
---

## Drop a File, Hydra Learns

To teach Hydra a new skill, create a folder with two TOML files and drop it into `skills/`. No code. No compilation. No deployment.

```
skills/
  devops/
    genome.toml       <- proven approaches for DevOps situations
    functor.toml      <- maps DevOps concepts to universal primitives
```

## genome.toml -- What Hydra Knows

Each entry is a situation-approach pair with Bayesian confidence:

```toml
[[entries]]
situation    = "Kubernetes pod keeps crashing with OOMKilled"
approach     = "check resource requests vs limits -- requests should be
               80% of limits, not 50% -- set memory limit to 2x the
               actual peak usage observed in monitoring"
confidence   = 0.91
observations = 3200
notes        = "Covers 90% of OOMKill cases in production clusters"
```


:::tip

Confidence updates automatically via Bayesian Beta distribution as entries are used. An entry with 91% confidence and 3,200 observations is not a guess -- it is statistical evidence.

:::


## functor.toml -- How Hydra Understands

Functors map domain-specific words to Hydra's universal axiom primitives:

```toml
[[mappings]]
domain_concept  = "deployment"
axiom_primitive = "Risk"
weight          = 0.85

[[mappings]]
domain_concept  = "monitoring"
axiom_primitive = "Understanding"
weight          = 0.70
```

When someone says "the Kubernetes deployment caused an incident," three primitives fire: Dependency, Risk, Risk. The pattern engine checks for anti-patterns. The genome surfaces proven approaches. All from a TOML file.

## How Skills Load

On boot, the kernel calls `GenomeStore::load_from_skills()`:

1. Scan `skills/` directory for folders
2. Parse `genome.toml` entries into `GenomeEntry` structs
3. Deduplicate by situation signature
4. Add to the genome store
5. Persist to `genome.db` (SQLite)

```
hydra: skill 'devops' -- parsed 15 genome entries
hydra: skill 'security' -- parsed 22 genome entries
hydra: loaded 50 genome entries from skills/
```


:::warning

Every skill load is checked against the 7 constitutional laws. A skill that violates the constitution is rejected.

:::


## Hot-Loading and the Self-Writing Genome

Skills are loaded on boot. The self-writing genome extends this by creating entries automatically:

- Every 20 ambient steps, the dream loop checks for detected patterns
- A pattern becomes a genome entry when observed **5+ times** with **>= 75% success**
- The genome grows without anyone writing TOML

## Create Your Own Skill


  
**For Engineers:**

    ```toml
    # skills/databases/genome.toml
    [[entries]]
    situation    = "database query is slow"
    approach     = "EXPLAIN the query first -- add indexes on WHERE
                   and JOIN columns -- measure before and after"
    confidence   = 0.92
    observations = 10000
    ```
  
  
**For Anyone:**

    ```toml
    # skills/triage/genome.toml
    [[entries]]
    situation    = "patient with chest pain and shortness of breath"
    approach     = "IMMEDIATE: 12-lead ECG, troponin, SpO2 --
                   activate STEMI protocol if ST elevation present"
    confidence   = 0.97
    observations = 50000
    ```
  


## Current Skill Inventory

Hydra ships with **184 genome entries** across **24 skill domains** including general engineering, DevOps, security, architecture, coding, finance, business, science, health, and more. See the full [Skill Registry](/skills) for the complete list.

## The SKILL-FORMAT Spec

The complete specification for `genome.toml` and `functor.toml` lives in `skills/SKILL-FORMAT.md`. Key rules:

- Each entry must have `situation`, `approach`, `confidence`, and `observations`
- Confidence is a float between 0.0 and 1.0
- Observations is the number of real-world cases backing the approach
- Optional fields: `notes`, `source`, `tags`
