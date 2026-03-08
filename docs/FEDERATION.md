# Federation

Hydra supports peer-to-peer federation for multi-agent collaboration.

## Overview

Federation allows multiple Hydra instances to:

- **Delegate tasks** to peers with specific capabilities
- **Share skills** across the network
- **Synchronize state** with conflict resolution
- **Discover peers** via manual or multicast methods

## Setup

### Register a Peer

```rust
use hydra_federation::{PeerRegistry, PeerInfo, PeerCapabilities};

let registry = PeerRegistry::new();
registry.add(PeerInfo {
    id: "peer-001".into(),
    name: "Analysis Node".into(),
    endpoint: "http://192.168.1.10:7777".into(),
    capabilities: PeerCapabilities {
        sisters: vec!["memory".into(), "codebase".into()],
        skills: vec!["code_analysis".into()],
        ..Default::default()
    },
    ..Default::default()
});
```

### Delegate a Task

```rust
use hydra_federation::delegation::DelegationManager;

let manager = DelegationManager::new(&registry);
let result = manager.delegate(DelegatedTask {
    description: "Analyze security vulnerabilities".into(),
    requirements: vec!["code_analysis".into()],
    max_duration_secs: 300,
    ..Default::default()
}).await?;
```

## Trust Levels

| Level | Description | Permissions |
|-------|-------------|-------------|
| `Untrusted` | Unknown peer | Read-only, no delegation |
| `Basic` | Verified peer | Limited delegation |
| `Trusted` | Established peer | Full delegation, skill sharing |
| `Full` | Fully trusted | All operations |

Trust can be promoted or demoted dynamically based on behavior.

## Sync Conflict Resolution

When peers synchronize state, conflicts are resolved by strategy:

- **LastWriteWins** — Most recent change wins
- **KeepLocal** — Local changes take priority
- **HigherVersion** — Higher version number wins

## Discovery Methods

- **Manual** — Explicitly add peer endpoints
- **Multicast** — Auto-discover peers on local network
