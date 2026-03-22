# HYDRA SKILLS

## What This Folder Is

This is where Hydra's knowledge lives — every domain, every proven approach, every pattern it knows. Drop a TOML file and Hydra learns a new domain. No training. No fine-tuning. No code.

## Rules

1. **One folder per skill** — named clearly (e.g., `architecture/`, `finance/`, `security/`)
2. **`genome.toml` is required** — contains `[[entries]]` with situation + approach pairs
3. **`functor.toml` is optional** — maps domain concepts to axiom primitives (Risk, Understanding, Dependency, Volume)
4. **Hydra reads on boot** — skills are loaded into the genome store at startup
5. **You can add skills at any time** — restart Hydra to load new ones
6. **Hydra writes here too** — the self-writing genome creates entries in `skills/generated/`

## Structure

```
skills/
  README.md              ← this file
  architecture/
    genome.toml          ← 10 proven approaches
    functor.toml         ← axiom mappings
  finance/
    genome.toml          ← 26 proven approaches
    functor.toml
  developer/
    genome.toml          ← 30 proven approaches
    functor.toml
  ... (29 skills total, 303 genome entries)
```

## Format

```toml
# genome.toml
[[entries]]
situation    = "choosing between microservices and monolith"
approach     = "start with a monolith — extract services only when boundaries are clear"
confidence   = 0.91
observations = 5000
```

See `SKILL-FORMAT.md` in this folder for the complete specification.

## How It Works

When you ask Hydra a question, the genome store queries all loaded skills using IDF-weighted scoring + axiom vector cosine similarity. The best matches are injected into the prompt as proven approaches — with mathematically grounded confidence intervals.

Drop a TOML file. Hydra learns a new domain. That is it.
