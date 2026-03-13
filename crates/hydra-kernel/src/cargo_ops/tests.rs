//! Tests for cargo_ops module.

use super::*;
use tempfile::tempdir;

fn setup_workspace(dir: &Path) {
    let content = r#"[workspace]
resolver = "2"
members = [
    "crates/existing-crate",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
"#;
    std::fs::write(dir.join("Cargo.toml"), content).unwrap();

    // Create an existing crate so the workspace is valid
    let src = dir.join("crates/existing-crate/src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        dir.join("crates/existing-crate/Cargo.toml"),
        "[package]\nname = \"existing-crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(src.join("lib.rs"), "// existing\n").unwrap();
}

#[test]
fn test_valid_crate_names() {
    assert!(is_valid_crate_name("my-crate"));
    assert!(is_valid_crate_name("hydra-core"));
    assert!(is_valid_crate_name("a"));
    assert!(is_valid_crate_name("crate_name"));
    assert!(is_valid_crate_name("x123"));
}

#[test]
fn test_invalid_crate_names() {
    assert!(!is_valid_crate_name(""));
    assert!(!is_valid_crate_name("123abc")); // starts with digit
    assert!(!is_valid_crate_name("My-Crate")); // uppercase
    assert!(!is_valid_crate_name("-bad")); // starts with hyphen
    assert!(!is_valid_crate_name("has space"));
    assert!(!is_valid_crate_name("has.dot"));
    // 65 chars
    let long = "a".repeat(65);
    assert!(!is_valid_crate_name(&long));
}

#[test]
fn test_scaffold_lib_crate() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    let result = CargoOps::scaffold_crate(
        dir.path(),
        "my-lib",
        CrateType::Lib,
        "A test library",
        &[("serde", "{ workspace = true }")],
    )
    .unwrap();

    assert_eq!(result.files_created.len(), 2); // Cargo.toml + lib.rs
    assert!(result.crate_path.join("Cargo.toml").exists());
    assert!(result.crate_path.join("src/lib.rs").exists());
    assert!(!result.crate_path.join("src/main.rs").exists());

    let toml = std::fs::read_to_string(result.crate_path.join("Cargo.toml")).unwrap();
    assert!(toml.contains("name = \"my-lib\""));
    assert!(toml.contains("serde = { workspace = true }"));
}

#[test]
fn test_scaffold_bin_crate() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    let result = CargoOps::scaffold_crate(
        dir.path(),
        "my-bin",
        CrateType::Bin,
        "A test binary",
        &[],
    )
    .unwrap();

    assert_eq!(result.files_created.len(), 2); // Cargo.toml + main.rs
    assert!(result.crate_path.join("src/main.rs").exists());
    assert!(!result.crate_path.join("src/lib.rs").exists());

    let main = std::fs::read_to_string(result.crate_path.join("src/main.rs")).unwrap();
    assert!(main.contains("Hello from my-bin"));
}

#[test]
fn test_scaffold_both_crate() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    let result = CargoOps::scaffold_crate(
        dir.path(),
        "my-both",
        CrateType::Both,
        "Lib and bin",
        &[],
    )
    .unwrap();

    assert_eq!(result.files_created.len(), 3); // Cargo.toml + lib.rs + main.rs
    assert!(result.crate_path.join("src/lib.rs").exists());
    assert!(result.crate_path.join("src/main.rs").exists());
}

#[test]
fn test_scaffold_duplicate_fails() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    CargoOps::scaffold_crate(dir.path(), "dup-crate", CrateType::Lib, "First", &[]).unwrap();
    let err = CargoOps::scaffold_crate(dir.path(), "dup-crate", CrateType::Lib, "Second", &[])
        .unwrap_err();
    assert!(err.contains("already exists"));
}

#[test]
fn test_scaffold_invalid_name_fails() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    let err =
        CargoOps::scaffold_crate(dir.path(), "Bad-Name", CrateType::Lib, "Bad", &[]).unwrap_err();
    assert!(err.contains("Invalid crate name"));
}

#[test]
fn test_add_workspace_member() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    CargoOps::add_workspace_member(dir.path(), "crates/new-crate").unwrap();

    let content = std::fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    assert!(content.contains("\"crates/new-crate\""));
    // Original member still there
    assert!(content.contains("\"crates/existing-crate\""));
}

#[test]
fn test_add_workspace_member_idempotent() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    CargoOps::add_workspace_member(dir.path(), "crates/new-crate").unwrap();
    CargoOps::add_workspace_member(dir.path(), "crates/new-crate").unwrap();

    let content = std::fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    let count = content.matches("crates/new-crate").count();
    assert_eq!(count, 1, "Member should appear exactly once");
}

#[test]
fn test_add_dependency() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    // Scaffold a crate first so we have a Cargo.toml to modify
    CargoOps::scaffold_crate(dir.path(), "dep-test", CrateType::Lib, "Test", &[]).unwrap();

    CargoOps::add_dependency(
        dir.path(),
        "crates/dep-test",
        "tokio",
        "{ workspace = true }",
    )
    .unwrap();

    let content =
        std::fs::read_to_string(dir.path().join("crates/dep-test/Cargo.toml")).unwrap();
    assert!(content.contains("tokio = { workspace = true }"));
}

#[test]
fn test_add_dependency_idempotent() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    CargoOps::scaffold_crate(dir.path(), "dep-idem", CrateType::Lib, "Test", &[]).unwrap();

    CargoOps::add_dependency(dir.path(), "crates/dep-idem", "tokio", "{ workspace = true }")
        .unwrap();
    CargoOps::add_dependency(dir.path(), "crates/dep-idem", "tokio", "{ workspace = true }")
        .unwrap();

    let content =
        std::fs::read_to_string(dir.path().join("crates/dep-idem/Cargo.toml")).unwrap();
    let count = content.matches("tokio =").count();
    assert_eq!(count, 1);
}

#[test]
fn test_add_workspace_dependency() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    CargoOps::add_workspace_dependency(dir.path(), "tracing", "\"0.1\"").unwrap();

    let content = std::fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    assert!(content.contains("tracing = \"0.1\""));
}

#[test]
fn test_add_workspace_dependency_idempotent() {
    let dir = tempdir().unwrap();
    setup_workspace(dir.path());

    // serde already exists in setup_workspace
    CargoOps::add_workspace_dependency(
        dir.path(),
        "serde",
        "{ version = \"2.0\" }",
    )
    .unwrap();

    let content = std::fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    // Should still have original, not the new one
    assert!(content.contains("serde = { version = \"1.0\""));
}

#[test]
fn test_generate_cargo_toml_format() {
    let toml = generate_cargo_toml(
        "test-crate",
        "A test crate",
        &CrateType::Lib,
        &[("serde", "{ workspace = true }"), ("tokio", "\"1.0\"")],
    );

    assert!(toml.contains("[package]"));
    assert!(toml.contains("name = \"test-crate\""));
    assert!(toml.contains("description = \"A test crate\""));
    assert!(toml.contains("version.workspace = true"));
    assert!(toml.contains("[dependencies]"));
    assert!(toml.contains("serde = { workspace = true }"));
    assert!(toml.contains("tokio = \"1.0\""));
    assert!(toml.contains("[dev-dependencies]"));
}
