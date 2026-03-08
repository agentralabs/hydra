use std::collections::BTreeMap;

use hydra_installer::{
    auth_required, generate_token, merge_mcp_config, profile_description, steps_for_profile,
    InstallProfile, InstallStep, McpConfig, McpServerEntry,
};

#[test]
fn test_default_profile_detection() {
    // We can't fully control env in a unit test, but we can verify the
    // function returns a valid variant without panicking.
    let profile = hydra_installer::default_profile();
    // Should be one of the three variants
    assert!(
        profile == InstallProfile::Desktop
            || profile == InstallProfile::Terminal
            || profile == InstallProfile::Server
    );
}

#[test]
fn test_profile_descriptions() {
    let desktop_desc = profile_description(&InstallProfile::Desktop);
    let terminal_desc = profile_description(&InstallProfile::Terminal);
    let server_desc = profile_description(&InstallProfile::Server);

    assert!(desktop_desc.contains("GUI"));
    assert!(terminal_desc.contains("Terminal"));
    assert!(server_desc.contains("auth"));

    // All descriptions must be non-empty and distinct
    assert_ne!(desktop_desc, terminal_desc);
    assert_ne!(terminal_desc, server_desc);
}

#[test]
fn test_steps_for_desktop() {
    let steps = steps_for_profile(&InstallProfile::Desktop);
    assert!(steps.contains(&InstallStep::CheckDeps));
    assert!(steps.contains(&InstallStep::MergeMcp));
    assert!(steps.contains(&InstallStep::PrintBanner));
    // Desktop does not require auth
    assert!(!steps.contains(&InstallStep::SetupAuth));
}

#[test]
fn test_steps_for_terminal() {
    let steps = steps_for_profile(&InstallProfile::Terminal);
    assert!(steps.contains(&InstallStep::CheckDeps));
    assert!(steps.contains(&InstallStep::MergeMcp));
    assert!(steps.contains(&InstallStep::WriteCompletions));
    // Terminal does not require auth
    assert!(!steps.contains(&InstallStep::SetupAuth));
}

#[test]
fn test_steps_for_server() {
    let steps = steps_for_profile(&InstallProfile::Server);
    assert!(steps.contains(&InstallStep::CheckDeps));
    assert!(steps.contains(&InstallStep::MergeMcp));
    // Server DOES require auth
    assert!(steps.contains(&InstallStep::SetupAuth));
}

#[test]
fn test_mcp_merge_no_overwrite() {
    let mut existing_servers = BTreeMap::new();
    existing_servers.insert(
        "memory".to_string(),
        McpServerEntry {
            command: "old-binary".to_string(),
            args: vec![],
            env: BTreeMap::new(),
        },
    );
    let existing = McpConfig {
        mcp_servers: existing_servers,
    };

    let mut new_servers = BTreeMap::new();
    new_servers.insert(
        "memory".to_string(),
        McpServerEntry {
            command: "new-binary".to_string(),
            args: vec!["--flag".to_string()],
            env: BTreeMap::new(),
        },
    );
    let new_config = McpConfig {
        mcp_servers: new_servers,
    };

    let merged = merge_mcp_config(&existing, &new_config);

    // The existing "memory" entry must NOT be overwritten
    assert_eq!(merged.mcp_servers["memory"].command, "old-binary");
    assert!(merged.mcp_servers["memory"].args.is_empty());
}

#[test]
fn test_mcp_merge_adds_new() {
    let mut existing_servers = BTreeMap::new();
    existing_servers.insert(
        "memory".to_string(),
        McpServerEntry {
            command: "mem-bin".to_string(),
            args: vec![],
            env: BTreeMap::new(),
        },
    );
    let existing = McpConfig {
        mcp_servers: existing_servers,
    };

    let mut new_servers = BTreeMap::new();
    new_servers.insert(
        "vision".to_string(),
        McpServerEntry {
            command: "vis-bin".to_string(),
            args: vec!["--port".to_string(), "8080".to_string()],
            env: BTreeMap::new(),
        },
    );
    let new_config = McpConfig {
        mcp_servers: new_servers,
    };

    let merged = merge_mcp_config(&existing, &new_config);

    // Both entries should be present
    assert_eq!(merged.mcp_servers.len(), 2);
    assert_eq!(merged.mcp_servers["memory"].command, "mem-bin");
    assert_eq!(merged.mcp_servers["vision"].command, "vis-bin");
}

#[test]
fn test_generate_token_length() {
    let token = generate_token();
    // 32 bytes -> 64 hex characters
    assert_eq!(token.len(), 64);
    // Must be valid hex
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_auth_required_server_only() {
    assert!(!auth_required(&InstallProfile::Desktop));
    assert!(!auth_required(&InstallProfile::Terminal));
    assert!(auth_required(&InstallProfile::Server));
}

#[test]
fn test_install_steps_order() {
    // For every profile, CheckDeps must come first and PrintBanner must come last.
    for profile in &[
        InstallProfile::Desktop,
        InstallProfile::Terminal,
        InstallProfile::Server,
    ] {
        let steps = steps_for_profile(profile);
        assert!(!steps.is_empty());
        assert_eq!(steps.first().unwrap(), &InstallStep::CheckDeps);
        assert_eq!(steps.last().unwrap(), &InstallStep::PrintBanner);
    }
}
