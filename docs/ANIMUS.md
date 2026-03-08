# Animus Prime

Animus is Hydra's AI-native internal language. It provides a semantic AST for cognitive operations and compiles to 6 target languages.

## Overview

Animus bridges the gap between AI reasoning and executable code. Instead of generating code directly, Hydra expresses intent in Animus, which is then compiled to the appropriate target language.

## Script Syntax

```animus
entity User {
  name: string
  email: string
  age: number
}

create User {
  name: "Alice"
  email: "alice@example.com"
  age: 30
}

read User where name == "Alice"

if age > 18 {
  return "adult"
}

api UserService {
  endpoint GET "/users" {
    read User
  }
  endpoint POST "/users" {
    create User
  }
}
```

## Prime AST

The semantic AST (`PrimeNode`) has 15 variants:

| Node | Description |
|------|-------------|
| `Entity` | Data structure definition |
| `Create` | Insert/instantiate |
| `Read` | Query/retrieve |
| `Update` | Modify existing |
| `Delete` | Remove |
| `If` | Conditional logic |
| `ForEach` | Iteration |
| `Return` | Return value |
| `Let` | Variable binding |
| `Api` | API definition |
| `Endpoint` | API route |
| `Call` | Function/service call |
| `Block` | Grouped statements |
| `Literal` | Values (string, number, bool) |
| `Identifier` | References |

## Compilation Targets

### JavaScript

```bash
# Entities → ES6 classes
# CRUD → Prisma-style calls
# API → Express.js routes
```

### Python

```bash
# Entities → @dataclass
# CRUD → SQLAlchemy-style
# API → FastAPI routes
```

### Rust

```bash
# Entities → pub struct with derive macros
# API → axum Router handlers
```

### Go

```bash
# Entities → struct with json tags
# API → gorilla/mux handlers
# PascalCase conventions
```

### SQL

```bash
# Entities → CREATE TABLE
# CRUD → INSERT/SELECT/UPDATE/DELETE
```

### Shell

```bash
# set -euo pipefail
# if/for/echo mapped to bash equivalents
```

## Usage

```rust
use animus::{AnimusEngine, compiler::Target};

let engine = AnimusEngine::new();

// Parse Animus script
let ast = engine.parse_and_validate("entity User { name: string }")?;

// Compile to target
let result = engine.compile_to(&ast, Target::Python)?;
println!("{}", result.code);
```

## Integration with Hydra

The `AnimusEngine` bridges the cognitive loop to the compilation pipeline:

1. **Process phase** — Cognitive output is expressed as Animus AST
2. **Validate** — AST is checked for semantic correctness
3. **Compile** — AST is compiled to the recommended target language
4. **Execute** — Compiled code is handed to the skill executor

The engine recommends targets based on the AST content: entities suggest Python or Rust, APIs suggest JavaScript or Go, data operations suggest SQL.
