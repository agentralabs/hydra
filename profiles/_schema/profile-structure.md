# Profile Directory Structure

Each profile lives in `profiles/<name>/` with this layout:

```
profiles/<name>/
  ├── profile.toml          ← Profile metadata (name, version, domain)
  ├── identity.toml         ← Persona, tone, constraints
  ├── model.toml            ← Model selection (default, fast, provider)
  ├── permissions.toml      ← Permission overrides
  ├── sisters.toml          ← Sister emphasis and rationale
  ├── goals.toml            ← Profile goals
  ├── connections.toml      ← (Optional) SSH/remote connections
  ├── prompt_overlay.md     ← (Optional) Extra system prompt
  ├── beliefs/
  │   ├── factory/          ← Factory beliefs (replaced on update)
  │   │   └── <domain>/    ← Grouped by domain
  │   │       └── <topic>.toml
  │   └── learned/          ← User-learned beliefs (never touched by updates)
  │       └── <topic>.toml
  └── skills/
      ├── factory/          ← Factory skills (replaced on update)
      │   └── <skill>.toml
      └── custom/           ← User-created skills (never touched by updates)
          └── <skill>.toml
```

## Factory vs Learned/Custom

- `factory/` directories are owned by Hydra and replaced entirely during profile updates
- `learned/` and `custom/` directories are owned by the user and never modified by updates
- This separation ensures user customizations survive profile version upgrades

## Belief File Format

See `belief-schema.toml` for the full schema. Each file contains `[[beliefs]]` arrays.

## Skill File Format

See `skill-schema.toml` for the full schema. Each file has `[metadata]`, `[trigger]`, `[steps]`, `[sisters]` sections.
