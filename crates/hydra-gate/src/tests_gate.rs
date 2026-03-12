use hydra_core::types::{Action, ActionType};

use crate::boundary::{BoundaryEnforcer, BoundaryResult};
use crate::kill_switch::KillSwitch;

// ── BoundaryEnforcer Tests ──

#[test]
fn boundary_blocks_system_paths() {
    let enforcer = BoundaryEnforcer::new();
    let blocked_paths = vec![
        "/System/Library/config",
        "~/.ssh/id_rsa",
        ".ssh/authorized_keys",
        "~/.gnupg/keys",
        ".gnupg/pubring.kbx",
        "/etc/shadow",
        "/usr/bin/bash",
        "/usr/sbin/service",
        "/sbin/init",
        "/boot/vmlinuz",
        "/proc/1/status",
        "/sys/class/net",
    ];
    for path in blocked_paths {
        assert!(
            matches!(enforcer.check(path), BoundaryResult::Blocked(_)),
            "Path '{}' should be blocked",
            path
        );
    }
}

#[test]
fn boundary_blocks_destructive_commands() {
    let enforcer = BoundaryEnforcer::new();
    assert!(
        matches!(enforcer.check("rm -rf /"), BoundaryResult::Blocked(_)),
        "rm -rf / should be blocked"
    );
}

#[test]
fn boundary_blocks_email_and_payment() {
    let enforcer = BoundaryEnforcer::new();
    assert!(matches!(
        enforcer.check("send_email user@example.com"),
        BoundaryResult::Blocked(_)
    ));
    assert!(matches!(
        enforcer.check("process_payment $500"),
        BoundaryResult::Blocked(_)
    ));
}

#[test]
fn boundary_blocks_self_modification() {
    let enforcer = BoundaryEnforcer::new();
    let self_mod_targets = vec![
        "hydra-gate/src/gate.rs",
        "hydra-kernel/src/cognitive_loop.rs",
        "hydra-core/src/types.rs",
    ];
    for target in self_mod_targets {
        assert!(
            matches!(enforcer.check(target), BoundaryResult::Blocked(_)),
            "Self-modification target '{}' should be blocked",
            target
        );
    }
}

#[test]
fn boundary_allows_safe_paths() {
    let enforcer = BoundaryEnforcer::new();
    let safe_paths = vec![
        "src/main.rs",
        "/home/user/project/lib.rs",
        "/tmp/test.txt",
        "Cargo.toml",
        "README.md",
        "/Users/dev/project/app.py",
    ];
    for path in safe_paths {
        assert!(
            matches!(enforcer.check(path), BoundaryResult::Allowed),
            "Path '{}' should be allowed",
            path
        );
    }
}

#[test]
fn boundary_case_insensitive() {
    let enforcer = BoundaryEnforcer::new();
    assert!(matches!(
        enforcer.check("/ETC/PASSWD"),
        BoundaryResult::Blocked(_)
    ));
    assert!(matches!(
        enforcer.check("/SYSTEM/Library"),
        BoundaryResult::Blocked(_)
    ));
    assert!(matches!(
        enforcer.check("SEND_EMAIL"),
        BoundaryResult::Blocked(_)
    ));
}

#[test]
fn boundary_custom_blocked_path() {
    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_blocked_path("/custom/secret/");
    assert!(matches!(
        enforcer.check("/custom/secret/data.json"),
        BoundaryResult::Blocked(_)
    ));
    // Original blocks still work
    assert!(matches!(
        enforcer.check("/etc/hosts"),
        BoundaryResult::Blocked(_)
    ));
}

#[test]
fn boundary_check_action_with_hard_boundary() {
    use crate::boundary::HardBoundary;

    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_boundary(HardBoundary {
        description: "No database drops".into(),
        blocked_actions: vec!["drop_database".into()],
    });

    let result = enforcer.check_action("drop_database", "production_db");
    assert!(
        matches!(result, BoundaryResult::Blocked(_)),
        "Hard boundary should block drop_database action"
    );

    let result = enforcer.check_action("query_database", "production_db");
    assert!(
        matches!(result, BoundaryResult::Allowed),
        "Non-blocked action should be allowed"
    );
}

#[test]
fn boundary_violation_display() {
    let enforcer = BoundaryEnforcer::new();
    if let BoundaryResult::Blocked(violation) = enforcer.check("/etc/passwd") {
        let display = format!("{}", violation);
        assert!(display.contains("blocked_path"), "Display should show rule name");
        assert!(display.contains("/etc/passwd"), "Display should show target");
    } else {
        panic!("/etc/passwd should be blocked");
    }
}

// ── KillSwitch Tests ──

#[test]
fn kill_switch_instant_halt() {
    let ks = KillSwitch::new();
    assert!(!ks.is_halted(), "Should not be halted initially");

    let record = ks.instant_halt("test emergency", "test_user");
    assert!(ks.is_halted(), "Should be halted after instant_halt");
    assert_eq!(record.reason, "test emergency");
    assert_eq!(record.halted_by, "test_user");
}

#[test]
fn kill_switch_resume() {
    let ks = KillSwitch::new();
    ks.instant_halt("test halt", "test_user");
    assert!(ks.is_halted());

    ks.resume();
    assert!(!ks.is_halted(), "Should not be halted after resume");
    assert!(
        ks.halt_reason().is_none(),
        "Halt reason should be cleared after resume"
    );
}

#[test]
fn kill_switch_halt_reason_recorded() {
    let ks = KillSwitch::new();
    assert!(ks.halt_reason().is_none(), "No reason before halt");

    ks.instant_halt("critical bug found", "admin");
    let reason = ks.halt_reason().expect("Should have halt reason");
    assert_eq!(reason.reason, "critical bug found");
    assert_eq!(reason.halted_by, "admin");
}

#[test]
fn kill_switch_concurrent_halt_check() {
    use std::sync::Arc;
    use std::thread;

    let ks = Arc::new(KillSwitch::new());
    let ks_clone = ks.clone();

    // Halt from another thread
    let handle = thread::spawn(move || {
        ks_clone.instant_halt("concurrent halt", "thread");
    });
    handle.join().unwrap();

    // Check from main thread
    assert!(ks.is_halted(), "Halt should be visible across threads");
    let reason = ks.halt_reason().unwrap();
    assert_eq!(reason.reason, "concurrent halt");
}

#[test]
fn kill_switch_multiple_halt_resume_cycles() {
    let ks = KillSwitch::new();

    for i in 0..5 {
        ks.instant_halt(format!("halt {}", i), "cycle_test");
        assert!(ks.is_halted());
        ks.resume();
        assert!(!ks.is_halted());
    }
}
