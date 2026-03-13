//! Cargo manifest operations — create crates, add dependencies, modify workspace.
//!
//! Uses string-based TOML manipulation to preserve formatting and comments
//! in existing Cargo.toml files.

use std::path::{Path, PathBuf};

/// Crate type for scaffolding.
#[derive(Debug, Clone, PartialEq)]
pub enum CrateType {
    Lib,
    Bin,
    Both,
}

/// Result of a scaffold operation.
#[derive(Debug)]
pub struct ScaffoldResult {
    pub crate_path: PathBuf,
    pub files_created: Vec<String>,
}

pub struct CargoOps;

impl CargoOps {
    /// Scaffold a new crate: create directory, Cargo.toml, src/lib.rs or src/main.rs.
    pub fn scaffold_crate(
        project_dir: &Path,
        crate_name: &str,
        crate_type: CrateType,
        description: &str,
        dependencies: &[(&str, &str)],
    ) -> Result<ScaffoldResult, String> {
        if !is_valid_crate_name(crate_name) {
            return Err(format!("Invalid crate name: {}", crate_name));
        }

        let crate_dir = project_dir.join("crates").join(crate_name);
        if crate_dir.exists() {
            return Err(format!("Crate already exists: {}", crate_dir.display()));
        }

        let src_dir = crate_dir.join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("mkdir failed: {}", e))?;

        let mut files_created = Vec::new();

        // Generate Cargo.toml
        let cargo_toml = generate_cargo_toml(crate_name, description, &crate_type, dependencies);
        let cargo_path = crate_dir.join("Cargo.toml");
        std::fs::write(&cargo_path, &cargo_toml).map_err(|e| e.to_string())?;
        files_created.push(format!("crates/{}/Cargo.toml", crate_name));

        // Create source files based on type
        if crate_type == CrateType::Lib || crate_type == CrateType::Both {
            let lib_content = format!("//! {} — core library.\n\n", description);
            std::fs::write(src_dir.join("lib.rs"), &lib_content).map_err(|e| e.to_string())?;
            files_created.push(format!("crates/{}/src/lib.rs", crate_name));
        }
        if crate_type == CrateType::Bin || crate_type == CrateType::Both {
            let main_content = format!(
                "//! {} — CLI entry point.\n\nfn main() {{\n    println!(\"Hello from {}\");\n}}\n",
                description, crate_name
            );
            std::fs::write(src_dir.join("main.rs"), &main_content).map_err(|e| e.to_string())?;
            files_created.push(format!("crates/{}/src/main.rs", crate_name));
        }

        Ok(ScaffoldResult {
            crate_path: crate_dir,
            files_created,
        })
    }

    /// Add a workspace member to the root Cargo.toml `members = [...]` array.
    pub fn add_workspace_member(project_dir: &Path, member_path: &str) -> Result<(), String> {
        let cargo_path = project_dir.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_path).map_err(|e| e.to_string())?;

        // Already a member — idempotent
        if content.contains(&format!("\"{}\"", member_path)) {
            return Ok(());
        }

        let insert_line = format!("    \"{}\",", member_path);
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let mut in_members = false;
        let mut insert_idx = None;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("members") && trimmed.contains('[') {
                in_members = true;
                continue;
            }
            if in_members && trimmed.starts_with(']') {
                insert_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = insert_idx {
            lines.insert(idx, insert_line);
            let new_content = lines.join("\n") + "\n";
            std::fs::write(&cargo_path, new_content).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Could not find members array in Cargo.toml".into())
        }
    }

    /// Add a dependency to a crate's Cargo.toml.
    ///
    /// `dep_spec` can be: `"{ workspace = true }"`, `"\"1.0\""`, etc.
    pub fn add_dependency(
        project_dir: &Path,
        crate_path: &str,
        dep_name: &str,
        dep_spec: &str,
    ) -> Result<(), String> {
        let cargo_path = project_dir.join(crate_path).join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_path).map_err(|e| e.to_string())?;

        // Idempotent
        if content.contains(&format!("{} =", dep_name))
            || content.contains(&format!("{}=", dep_name))
        {
            return Ok(());
        }

        let dep_line = format!("{} = {}", dep_name, dep_spec);
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let mut dep_section_end = None;
        let mut in_deps = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed == "[dependencies]" {
                in_deps = true;
                continue;
            }
            if in_deps {
                if trimmed.starts_with('[') {
                    dep_section_end = Some(i);
                    break;
                }
                dep_section_end = Some(i + 1);
            }
        }

        if let Some(idx) = dep_section_end {
            lines.insert(idx, dep_line);
        } else {
            lines.push("[dependencies]".to_string());
            lines.push(dep_line);
        }

        let new_content = lines.join("\n") + "\n";
        std::fs::write(&cargo_path, new_content).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Add a workspace-level dependency to root Cargo.toml `[workspace.dependencies]`.
    pub fn add_workspace_dependency(
        project_dir: &Path,
        dep_name: &str,
        dep_spec: &str,
    ) -> Result<(), String> {
        let cargo_path = project_dir.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_path).map_err(|e| e.to_string())?;

        if content.contains(&format!("{} =", dep_name)) {
            return Ok(());
        }

        let dep_line = format!("{} = {}", dep_name, dep_spec);
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let mut in_ws_deps = false;
        let mut insert_idx = None;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed == "[workspace.dependencies]" {
                in_ws_deps = true;
                continue;
            }
            if in_ws_deps && trimmed.starts_with('[') {
                insert_idx = Some(i);
                break;
            }
            if in_ws_deps {
                insert_idx = Some(i + 1);
            }
        }

        if let Some(idx) = insert_idx {
            lines.insert(idx, dep_line);
            let new_content = lines.join("\n") + "\n";
            std::fs::write(&cargo_path, new_content).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Could not find [workspace.dependencies] in Cargo.toml".into())
        }
    }

    /// Verify a crate compiles via `cargo check`.
    pub fn check_crate(project_dir: &Path, crate_name: &str) -> Result<(), String> {
        crate::self_modify_pipeline::run_cargo_check(crate_name, project_dir)
    }

    /// Full scaffold + register: create crate, add to workspace, verify compilation.
    pub fn scaffold_and_register(
        project_dir: &Path,
        crate_name: &str,
        crate_type: CrateType,
        description: &str,
        dependencies: &[(&str, &str)],
    ) -> Result<ScaffoldResult, String> {
        let result =
            Self::scaffold_crate(project_dir, crate_name, crate_type, description, dependencies)?;
        let member_path = format!("crates/{}", crate_name);
        Self::add_workspace_member(project_dir, &member_path)?;

        if let Err(e) = Self::check_crate(project_dir, crate_name) {
            // Cleanup on failure
            let _ = std::fs::remove_dir_all(&result.crate_path);
            let cargo_path = project_dir.join("Cargo.toml");
            if let Ok(content) = std::fs::read_to_string(&cargo_path) {
                let filtered: String = content
                    .lines()
                    .filter(|l| !l.contains(&member_path))
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n";
                let _ = std::fs::write(&cargo_path, filtered);
            }
            return Err(format!("Scaffold failed cargo check: {}", e));
        }

        eprintln!(
            "[hydra:cargo] Scaffolded crate '{}' ({} files)",
            crate_name,
            result.files_created.len()
        );
        Ok(result)
    }
}

/// Generate Cargo.toml content for a new crate.
fn generate_cargo_toml(
    name: &str,
    description: &str,
    _crate_type: &CrateType,
    deps: &[(&str, &str)],
) -> String {
    let mut content = format!(
        "[package]\nname = \"{}\"\nversion.workspace = true\nedition.workspace = true\n\
         rust-version.workspace = true\ndescription = \"{}\"\n",
        name, description
    );

    content.push_str("\n[dependencies]\n");
    for (dep_name, dep_spec) in deps {
        content.push_str(&format!("{} = {}\n", dep_name, dep_spec));
    }

    content.push_str(
        "\n[dev-dependencies]\ntokio-test = { workspace = true }\n\
         pretty_assertions = { workspace = true }\n",
    );

    content
}

/// Validate a Rust crate name: lowercase alphanumeric + hyphens/underscores,
/// starts with a letter, max 64 chars.
fn is_valid_crate_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && name
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests;
