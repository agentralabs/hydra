# Universal Drop Gateway

## THIS IS THE ONLY SOURCE OF EXTERNAL INTEGRATION FOR HYDRA

Everything that enters Hydra from outside — credentials, skills, configs, documents,
genome exports, machine configs, certificates, learning sources — MUST go through
this gateway. There is NO other path.

## How It Works

1. User drops a file into `~/.hydra/drop/`
2. Gateway auto-classifies the file type (extension + content sniffing)
3. Security check (path traversal, size, executable detection)
4. Type-specific validation (TOML schema, JSON schema, content checks)
5. Type-specific processing (encrypt to vault, merge into config, create skill, etc.)
6. File moved to `~/.hydra/drop/processed/` (accepted) or `~/.hydra/drop/rejected/` (with .error sidecar)
7. Immutable audit record appended to `~/.hydra/drop/audit.jsonl`

## Supported Item Types

| Drop this... | Hydra does... |
|---|---|
| `api.env` or `.env` file | Encrypts credentials to vault, injects into env |
| `id_rsa` or SSH key | Stores encrypted in vault |
| `*.pem` or `*.crt` | Stores certificate in vault |
| `deploy-guide.md` | Learns as skill via /learn pipeline |
| `skill-name.tar.gz` | Extracts skill package to skills/ |
| `genome-export.json` | Merges entries into genome database |
| `sources.toml` | Merges learning sources config |
| `machines.toml` | Adds remote machines |
| `config.toml` | Overrides Hydra settings |
| `monitor.toml` | Adds monitor pollers/watchers |
| `cloud.toml` | Configures cloud backup provider |
| `immerse-rust.md` | Adds immersion domain content |
| `report.pdf` / `data.csv` | Queues for document analysis |

## Adding New Item Types (Extensibility)

Implement the `DropHandler` trait:

```rust
use crate::drop::handlers::DropHandler;
use crate::drop::classifier::DropItemType;

pub struct MyCustomHandler;

impl DropHandler for MyCustomHandler {
    fn handles(&self) -> Vec<DropItemType> {
        vec![DropItemType::Custom("my-type".into())]
    }
    fn validate(&self, path: &Path, _: &DropItemType) -> Result<(), String> {
        // Validate file contents
        Ok(())
    }
    fn process(&self, path: &Path, _: &DropItemType) -> Result<DropOutcome, String> {
        // Process the file
        Ok(DropOutcome::Accepted { ... })
    }
}
```

Then register it:
```rust
gateway.register_handler(Box::new(MyCustomHandler));
```

## Architecture

```
~/.hydra/drop/           ← User drops files here
~/.hydra/drop/processed/ ← Accepted files moved here
~/.hydra/drop/rejected/  ← Rejected files + .error sidecar
~/.hydra/drop/audit.jsonl ← Immutable audit trail
```

## Rules

1. NEVER bypass this gateway. All external data enters through `~/.hydra/drop/`.
2. Every file is hashed (SHA256) before processing for dedup and audit.
3. Credentials are ALWAYS encrypted to vault — never stored plaintext.
4. Rejected files get a `.error` sidecar explaining why.
5. The gateway polls every 5 seconds from the ambient loop.
6. Files dropped while Hydra is offline are processed on next boot.

## Files

- `mod.rs` — DropGateway orchestrator, DropRecord audit, directory management
- `classifier.rs` — DropItemType enum, classify(), security_check()
- `handlers.rs` — Built-in handlers + DropHandler trait for extensibility
