# Contributing to Hydra

Thank you for your interest in contributing to Hydra.

## Getting Started

```bash
git clone git@github.com:agentralabs/hydra.git
cd hydra
cargo build --workspace
cargo test --workspace
```

## Development

### Requirements

- Rust 1.75+
- Node.js 20+ (for desktop app)

### Running Tests

```bash
# All tests (1,395+)
cargo test --workspace

# Specific crate
cargo test -p hydra-runtime

# With real LLM (requires API key)
cargo test --workspace --features live-llm
```

### Code Style

- Run `cargo fmt --all` before committing
- Run `cargo clippy --workspace` and fix warnings
- Use conventional commit prefixes: `feat:`, `fix:`, `chore:`, `docs:`

### Crate Structure

| Layer | Crates |
|-------|--------|
| Interface | `hydra-server`, `hydra-cli`, `hydra-native` |
| Runtime | `hydra-runtime`, `hydra-kernel`, `hydra-pulse` |
| Intelligence | `hydra-model`, `hydra-compiler`, `hydra-belief`, `animus` |
| Safety | `hydra-gate`, `hydra-ledger`, `hydra-observability` |
| Integration | `hydra-sisters`, `hydra-mcp`, `hydra-federation`, `hydra-skills` |
| Innovation | `hydra-inventions` |

### Adding a New Crate

1. Create `crates/hydra-<name>/`
2. Add to `Cargo.toml` workspace members
3. Add workspace dependency reference
4. Write tests (aim for full coverage of public API)

## Pull Requests

- Keep PRs focused on a single change
- Include tests for new functionality
- Update docs if changing user-facing behavior
- All CI checks must pass

## Reporting Issues

Open an issue at [github.com/agentralabs/hydra/issues](https://github.com/agentralabs/hydra/issues).

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
