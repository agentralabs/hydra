use super::*;

#[test]
fn test_system_status_path() {
    assert_eq!(SystemRoutes::system_status(), "/api/system/status");
}

#[test]
fn test_list_steps_path() {
    assert_eq!(SystemRoutes::list_steps(), "/api/steps");
}

#[test]
fn test_list_approvals_path() {
    assert_eq!(SystemRoutes::list_approvals(), "/api/approvals");
}

#[test]
fn test_approve_path() {
    assert_eq!(SystemRoutes::approve(), "/api/approvals/:id/approve");
}

#[test]
fn test_deny_path() {
    assert_eq!(SystemRoutes::deny(), "/api/approvals/:id/deny");
}

#[test]
fn test_cancel_run_path() {
    assert_eq!(SystemRoutes::cancel_run(), "/api/runs/:id/cancel");
}

#[test]
fn test_approve_run_path() {
    assert_eq!(SystemRoutes::approve_run(), "/api/runs/:id/approve");
}

#[test]
fn test_run_status_path() {
    assert_eq!(SystemRoutes::run_status(), "/api/runs/:id/status");
}

#[test]
fn test_kill_run_path() {
    assert_eq!(SystemRoutes::kill_run(), "/api/runs/:id/kill");
}

#[test]
fn test_system_status_response_serialization() {
    let resp = SystemStatusResponse {
        uptime_secs: 120,
        degradation_level: "normal".into(),
        kill_switch_active: false,
        kill_switch_frozen: false,
        kill_switch_reason: None,
        pending_approvals: 3,
        server_mode: true,
        active_runs: 2,
        total_runs: 10,
        sisters: SistersStatus {
            memory: "not_connected",
            identity: "not_connected",
            codebase: "not_connected",
            vision: "not_connected",
            time: "not_connected",
        },
        autonomy_level: "supervised".into(),
        federation: FederationStatus {
            enabled: false,
            peers_connected: 0,
        },
        events_published: 42,
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["uptime_secs"], 120);
    assert_eq!(json["pending_approvals"], 3);
    assert_eq!(json["server_mode"], true);
    assert_eq!(json["active_runs"], 2);
    assert_eq!(json["total_runs"], 10);
    assert_eq!(json["sisters"]["memory"], "not_connected");
    assert_eq!(json["autonomy_level"], "supervised");
    assert_eq!(json["federation"]["enabled"], false);
    assert_eq!(json["events_published"], 42);
}

#[test]
fn test_run_status_response_serialization() {
    let resp = RunStatusResponse {
        id: "run-123".into(),
        status: "running".into(),
        intent: "test".into(),
        created_at: "2026-01-01".into(),
        updated_at: "2026-01-01".into(),
        completed_at: None,
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["id"], "run-123");
    assert!(json["completed_at"].is_null());
}

#[test]
fn test_approval_action_response_serialization() {
    let resp = ApprovalActionResponse {
        id: "appr-1".into(),
        status: "approved".into(),
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["id"], "appr-1");
    assert_eq!(json["status"], "approved");
}

#[test]
fn test_run_action_response_serialization() {
    let resp = RunActionResponse {
        run_id: "run-456".into(),
        status: "cancelled".into(),
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["run_id"], "run-456");
    assert_eq!(json["status"], "cancelled");
}

#[test]
fn test_steps_query_deserialization() {
    let json = serde_json::json!({"run_id": "run-789"});
    let q: StepsQuery = serde_json::from_value(json).unwrap();
    assert_eq!(q.run_id, "run-789");
}

#[test]
fn test_deny_request_deserialization() {
    let json = serde_json::json!({"reason": "too risky"});
    let d: DenyRequest = serde_json::from_value(json).unwrap();
    assert_eq!(d.reason, Some("too risky".into()));
}

#[test]
fn test_deny_request_no_reason() {
    let json = serde_json::json!({});
    let d: DenyRequest = serde_json::from_value(json).unwrap();
    assert!(d.reason.is_none());
}

#[test]
fn test_budget_path() {
    assert_eq!(SystemRoutes::budget(), "/api/system/budget");
}

#[test]
fn test_receipts_path() {
    assert_eq!(SystemRoutes::receipts(), "/api/system/receipts");
}

#[test]
fn test_offline_path() {
    assert_eq!(SystemRoutes::offline(), "/api/system/offline");
}
