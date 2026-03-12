use serde::{Deserialize, Serialize};

pub(crate) mod handlers;
mod tests;

// Re-export all handlers so routes/mod.rs can reference system::handler_name
pub use handlers::{
    approve_approval, approve_run, cancel_run, deny_approval, get_budget, get_inventions,
    get_offline, get_receipts, get_trust, kill_run, list_approvals, list_steps, run_status,
    system_status,
};

// ═══════════════════════════════════════════════════════════
// ROUTE PATHS
// ═══════════════════════════════════════════════════════════

pub struct SystemRoutes;

impl SystemRoutes {
    /// GET: system status overview
    pub fn system_status() -> &'static str {
        "/api/system/status"
    }

    /// GET: list steps for a run
    pub fn list_steps() -> &'static str {
        "/api/steps"
    }

    /// GET: list pending approvals
    pub fn list_approvals() -> &'static str {
        "/api/approvals"
    }

    /// POST: approve an approval request
    pub fn approve() -> &'static str {
        "/api/approvals/:id/approve"
    }

    /// POST: deny an approval request
    pub fn deny() -> &'static str {
        "/api/approvals/:id/deny"
    }

    /// POST: cancel a run
    pub fn cancel_run() -> &'static str {
        "/api/runs/:id/cancel"
    }

    /// POST: approve a run
    pub fn approve_run() -> &'static str {
        "/api/runs/:id/approve"
    }

    /// GET: get run status
    pub fn run_status() -> &'static str {
        "/api/runs/:id/status"
    }

    /// POST: kill a run
    pub fn kill_run() -> &'static str {
        "/api/runs/:id/kill"
    }

    /// GET: current trust levels
    pub fn trust() -> &'static str {
        "/api/system/trust"
    }

    /// GET: invention stats
    pub fn inventions() -> &'static str {
        "/api/system/inventions"
    }

    /// GET: budget usage stats
    pub fn budget() -> &'static str {
        "/api/system/budget"
    }

    /// GET: receipt ledger stats
    pub fn receipts() -> &'static str {
        "/api/system/receipts"
    }

    /// GET: offline status
    pub fn offline() -> &'static str {
        "/api/system/offline"
    }
}

// ═══════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct SystemStatusResponse {
    pub uptime_secs: u64,
    pub degradation_level: String,
    pub kill_switch_active: bool,
    pub kill_switch_frozen: bool,
    pub kill_switch_reason: Option<String>,
    pub pending_approvals: usize,
    pub server_mode: bool,
    pub active_runs: usize,
    pub total_runs: usize,
    pub sisters: SistersStatus,
    pub autonomy_level: String,
    pub federation: FederationStatus,
    pub events_published: u64,
}

#[derive(Debug, Serialize)]
pub struct SistersStatus {
    pub memory: &'static str,
    pub identity: &'static str,
    pub codebase: &'static str,
    pub vision: &'static str,
    pub time: &'static str,
}

#[derive(Debug, Serialize)]
pub struct FederationStatus {
    pub enabled: bool,
    pub peers_connected: usize,
}

#[derive(Debug, Deserialize)]
pub struct StepsQuery {
    pub run_id: String,
}

#[derive(Debug, Serialize)]
pub struct RunStatusResponse {
    pub id: String,
    pub status: String,
    pub intent: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DenyRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApprovalActionResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct RunActionResponse {
    pub run_id: String,
    pub status: String,
}
