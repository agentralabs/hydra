# Skill Development

Hydra's skill system allows extending its capabilities with custom tools.

## Skill Definition

```rust
use hydra_skills::{SkillDefinition, SkillParam, SandboxLevel};

let skill = SkillDefinition {
    name: "file_reader".into(),
    description: "Read file contents from disk".into(),
    parameters: vec![
        SkillParam {
            name: "path".into(),
            param_type: "string".into(),
            required: true,
            description: "File path to read".into(),
        },
    ],
    sandbox_level: SandboxLevel::Standard,
    triggers: vec!["read file".into(), "show contents".into()],
    ..Default::default()
};
```

## Sandbox Levels

| Level | Filesystem | Network | System |
|-------|-----------|---------|--------|
| `None` | Full access | Full access | Full access |
| `Standard` | Read-only | Allowed | Restricted |
| `Strict` | Temp only | Blocked | Blocked |

## Skill Sources

### MCP Adapter

Connect any MCP server as a skill source:

```rust
use hydra_skills::adapters::mcp::McpAdapter;

let adapter = McpAdapter::new("server-id", transport_config);
let skills = adapter.discover_skills().await?;
registry.register_all(skills);
```

### OpenClaw Adapter

Import OpenClaw-format skill definitions:

```rust
use hydra_skills::adapters::openclaw::OpenClawAdapter;

let adapter = OpenClawAdapter::new();
let skills = adapter.load_from_file("skills.yaml")?;
```

## Skill Validation

All skills pass through validation before execution:

1. **Parameter validation** — Required params present, types match
2. **Sandbox enforcement** — Operations checked against sandbox level
3. **Risk assessment** — High-risk skills require approval via the gate

## Registration

```rust
let registry = SkillRegistry::new();
registry.register(skill);

// Discover by trigger
let matches = registry.discover("read a file");
```
