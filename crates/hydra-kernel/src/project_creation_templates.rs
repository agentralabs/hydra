//! Templates for scaffolding a new sister project.
//!
//! Renders Cargo.toml files and short source templates.
//! Code-heavy templates (core lib.rs, MCP registry) are in project_creation_code.rs.

use super::project_creation::ProjectConfig;

/// All rendered template content for a new sister project.
pub struct ProjectTemplates {
    pub workspace_cargo: String,
    pub core_cargo: String,
    pub core_lib: String,
    pub mcp_cargo: String,
    pub mcp_main: String,
    pub mcp_tools_mod: String,
    pub mcp_registry: String,
    pub cli_cargo: String,
    pub cli_main: String,
    pub ffi_cargo: String,
    pub ffi_lib: String,
}

impl ProjectTemplates {
    /// Render all templates from a ProjectConfig.
    pub fn render(config: &ProjectConfig) -> Self {
        Self {
            workspace_cargo: render_workspace_cargo(config),
            core_cargo: render_core_cargo(config),
            core_lib: super::project_creation_code::render_core_lib(config),
            mcp_cargo: render_mcp_cargo(config),
            mcp_main: render_mcp_main(config),
            mcp_tools_mod: render_mcp_tools_mod(config),
            mcp_registry: super::project_creation_code::render_mcp_registry(config),
            cli_cargo: render_cli_cargo(config),
            cli_main: render_cli_main(config),
            ffi_cargo: render_ffi_cargo(config),
            ffi_lib: render_ffi_lib(config),
        }
    }
}

fn render_workspace_cargo(c: &ProjectConfig) -> String {
    format!(
        r#"[workspace]
resolver = "2"
members = [
    "crates/agentic-{key}",
    "crates/agentic-{key}-mcp",
    "crates/agentic-{key}-cli",
    "crates/agentic-{key}-ffi",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = "https://github.com/agentralabs/agentic-{key}"
authors = ["Agentra Labs <contact@agentralabs.tech>"]

[workspace.dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.35", features = ["full"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
tracing = "0.1"
rusqlite = {{ version = "0.31", features = ["bundled"] }}
tempfile = "3"
"#,
        key = c.key,
    )
}

fn render_core_cargo(c: &ProjectConfig) -> String {
    format!(
        r#"[package]
name = "agentic-{key}"
version.workspace = true
edition.workspace = true
description = "{desc}"
license = "MIT"
repository = "https://github.com/agentralabs/agentic-{key}"

[dependencies]
serde = {{ workspace = true }}
serde_json = {{ workspace = true }}
chrono = {{ workspace = true }}
rusqlite = {{ workspace = true }}

[dev-dependencies]
tempfile = {{ workspace = true }}
"#,
        key = c.key,
        desc = c.description,
    )
}

fn render_mcp_cargo(c: &ProjectConfig) -> String {
    format!(
        r#"[package]
name = "agentic-{key}-mcp"
version.workspace = true
edition.workspace = true
description = "MCP server for {name}"
license = "MIT"

[[bin]]
name = "agentic-{key}-mcp"
path = "src/main.rs"

[dependencies]
agentic-{key} = {{ version = "0.1.0", path = "../agentic-{key}" }}
serde = {{ workspace = true }}
serde_json = {{ workspace = true }}
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}
"#,
        key = c.key,
        name = c.name,
    )
}

fn render_mcp_main(c: &ProjectConfig) -> String {
    format!(
        r#"//! {name} MCP server — JSON-RPC stdio transport.

use std::io::{{self, BufRead, Write}};

mod tools;

fn main() {{
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_writer(std::io::stderr)
        .init();

    let store = agentic_{key_under}::Store::open_memory()
        .expect("Failed to create store");

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {{
        let line = match line {{
            Ok(l) => l,
            Err(_) => break,
        }};
        if line.trim().is_empty() {{ continue; }}
        let response = tools::registry::handle_request(&line, &store);
        writeln!(out, "{{}}", response).ok();
        out.flush().ok();
    }}
}}
"#,
        name = c.name,
        key_under = c.key.replace('-', "_"),
    )
}

fn render_mcp_tools_mod(_c: &ProjectConfig) -> String {
    "pub mod registry;\n".to_string()
}

fn render_cli_cargo(c: &ProjectConfig) -> String {
    format!(
        r#"[package]
name = "agentic-{key}-cli"
version.workspace = true
edition.workspace = true
description = "CLI for {name}"
license = "MIT"

[[bin]]
name = "{cli}"
path = "src/main.rs"

[dependencies]
agentic-{key} = {{ version = "0.1.0", path = "../agentic-{key}" }}
serde_json = {{ workspace = true }}
"#,
        key = c.key,
        name = c.name,
        cli = c.cli_binary,
    )
}

fn render_cli_main(c: &ProjectConfig) -> String {
    format!(
        r#"//! {name} CLI.

fn main() {{
    let store = agentic_{key_under}::Store::open_memory()
        .expect("Failed to open store");
    println!("{name} CLI ready. Store opened.");
    let _ = store;
}}
"#,
        name = c.name,
        key_under = c.key.replace('-', "_"),
    )
}

fn render_ffi_cargo(c: &ProjectConfig) -> String {
    format!(
        r#"[package]
name = "agentic-{key}-ffi"
version.workspace = true
edition.workspace = true
description = "FFI bindings for {name}"
license = "MIT"

[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
agentic-{key} = {{ version = "0.1.0", path = "../agentic-{key}" }}
serde_json = {{ workspace = true }}
"#,
        key = c.key,
        name = c.name,
    )
}

fn render_ffi_lib(c: &ProjectConfig) -> String {
    format!(
        r#"//! {name} FFI bindings — C-compatible interface.

/// FFI placeholder — version string.
#[no_mangle]
pub extern "C" fn {key_under}_version() -> *const std::os::raw::c_char {{
    b"{name} 0.1.0\0".as_ptr() as *const std::os::raw::c_char
}}
"#,
        name = c.name,
        key_under = c.key.replace('-', "_"),
    )
}
