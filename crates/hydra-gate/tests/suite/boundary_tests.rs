use hydra_gate::boundary::{BoundaryEnforcer, BoundaryResult, HardBoundary};

#[test]
fn test_hard_boundary_blocks_action() {
    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_boundary(HardBoundary {
        description: "No database drops".into(),
        blocked_actions: vec!["drop_database".into()],
    });

    match enforcer.check_action("drop_database", "production_db") {
        BoundaryResult::Blocked(v) => {
            assert_eq!(v.rule_name, "hard_boundary");
            assert!(v.reason.contains("database drops"));
        }
        BoundaryResult::Allowed => panic!("drop_database should be blocked"),
    }
}

#[test]
fn test_hard_boundary_allows_unrelated_action() {
    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_boundary(HardBoundary {
        description: "No database drops".into(),
        blocked_actions: vec!["drop_database".into()],
    });

    assert!(matches!(
        enforcer.check_action("read_file", "src/main.rs"),
        BoundaryResult::Allowed
    ));
}

#[test]
fn test_hard_boundary_case_insensitive() {
    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_boundary(HardBoundary {
        description: "No deploy".into(),
        blocked_actions: vec!["deploy_production".into()],
    });

    assert!(matches!(
        enforcer.check_action("DEPLOY_PRODUCTION", "my-app"),
        BoundaryResult::Blocked(_)
    ));
}

#[test]
fn test_hard_boundary_check_target_also() {
    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_boundary(HardBoundary {
        description: "No secret access".into(),
        blocked_actions: vec!["read_secrets".into()],
    });

    // check() (single arg) should also match hard boundary actions in the target
    assert!(matches!(
        enforcer.check("read_secrets"),
        BoundaryResult::Blocked(_)
    ));
}

#[test]
fn test_multiple_hard_boundaries() {
    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_boundary(HardBoundary {
        description: "No drops".into(),
        blocked_actions: vec!["drop_table".into(), "drop_database".into()],
    });
    enforcer.add_boundary(HardBoundary {
        description: "No truncate".into(),
        blocked_actions: vec!["truncate_table".into()],
    });

    assert!(matches!(
        enforcer.check_action("drop_table", "users"),
        BoundaryResult::Blocked(_)
    ));
    assert!(matches!(
        enforcer.check_action("truncate_table", "logs"),
        BoundaryResult::Blocked(_)
    ));
    assert!(matches!(
        enforcer.check_action("select", "users"),
        BoundaryResult::Allowed
    ));
}

#[test]
fn test_hard_boundaries_accessor() {
    let mut enforcer = BoundaryEnforcer::new();
    assert!(enforcer.hard_boundaries().is_empty());

    enforcer.add_boundary(HardBoundary {
        description: "Test".into(),
        blocked_actions: vec!["test_action".into()],
    });
    assert_eq!(enforcer.hard_boundaries().len(), 1);
}

#[test]
fn test_check_action_still_enforces_paths() {
    let enforcer = BoundaryEnforcer::new();
    // Even with check_action, blocked paths should still apply
    assert!(matches!(
        enforcer.check_action("read_file", "/etc/passwd"),
        BoundaryResult::Blocked(_)
    ));
}

#[test]
fn test_check_action_still_enforces_patterns() {
    let enforcer = BoundaryEnforcer::new();
    // Built-in blocked patterns should still fire via check_action
    assert!(matches!(
        enforcer.check_action("execute", "send_email to user@example.com"),
        BoundaryResult::Blocked(_)
    ));
}
