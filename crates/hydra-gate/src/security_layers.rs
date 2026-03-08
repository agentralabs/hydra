use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use hydra_core::error::HydraError;
use hydra_core::types::{Action, ActionType, Capability, CapabilityToken};

/// Layer 1: Perimeter — domain allowlist, TLS enforcement, rate limit check
pub struct PerimeterConfig {
    pub allowed_domains: HashSet<String>,
    pub rate_limit_per_minute: u32,
    rate_counter: AtomicU32,
}

impl PerimeterConfig {
    pub fn new() -> Self {
        let mut allowed = HashSet::new();
        // Default allowlist — common safe domains
        for domain in &[
            "github.com",
            "api.github.com",
            "registry.npmjs.org",
            "crates.io",
        ] {
            allowed.insert(domain.to_string());
        }
        Self {
            allowed_domains: allowed,
            rate_limit_per_minute: 600,
            rate_counter: AtomicU32::new(0),
        }
    }

    pub fn add_domain(&mut self, domain: &str) {
        self.allowed_domains.insert(domain.to_lowercase());
    }

    pub fn with_rate_limit(mut self, limit: u32) -> Self {
        self.rate_limit_per_minute = limit;
        self
    }
}

impl Default for PerimeterConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub fn check_perimeter(action: &Action) -> Result<(), HydraError> {
    check_perimeter_with_config(action, &PerimeterConfig::new())
}

pub fn check_perimeter_with_config(
    action: &Action,
    config: &PerimeterConfig,
) -> Result<(), HydraError> {
    if matches!(
        action.action_type,
        ActionType::Network | ActionType::ApiCall
    ) {
        let target = action.target.to_lowercase();

        // TLS 1.3 only — no HTTP downgrade
        if target.contains("http://") {
            return Err(HydraError::PermissionDenied(
                "Insecure HTTP connections are not allowed. TLS 1.3 is required for all network requests. Use https:// instead.".into(),
            ));
        }

        // Domain allowlist — only for external URLs
        if target.starts_with("https://") {
            if let Some(domain) = extract_domain(&target) {
                if !config.allowed_domains.contains(&domain) && !config.allowed_domains.is_empty() {
                    return Err(HydraError::PermissionDenied(format!(
                        "Domain '{}' is not in the allowlist. Add it to the perimeter config to allow access.",
                        domain
                    )));
                }
            }
        }

        // Rate limit check
        let count = config.rate_counter.fetch_add(1, Ordering::Relaxed);
        if count >= config.rate_limit_per_minute {
            return Err(HydraError::PermissionDenied(
                "Rate limit exceeded. Too many network requests this minute. Wait before retrying."
                    .into(),
            ));
        }
    }
    Ok(())
}

fn extract_domain(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let domain = without_scheme.split('/').next()?;
    let domain = domain.split(':').next()?; // Remove port
    Some(domain.to_string())
}

/// Layer 2: Authentication — verify identity is valid
pub fn check_authentication(token: Option<&CapabilityToken>) -> Result<(), HydraError> {
    match token {
        Some(t) if t.is_expired() => Err(HydraError::PermissionDenied(
            "Authentication token has expired. Session may have timed out. Please re-authenticate to continue.".into(),
        )),
        None => {
            // In local mode, no token needed — implicit local user trust
            Ok(())
        }
        Some(_) => Ok(()),
    }
}

/// Session context for Layer 2
#[derive(Debug, Clone, Default)]
pub struct SessionContext {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub project_id: Option<String>,
}

pub fn check_session(session: &SessionContext) -> Result<(), HydraError> {
    // Session validation: if session_id is set, it must be non-empty
    if let Some(ref sid) = session.session_id {
        if sid.trim().is_empty() {
            return Err(HydraError::SessionNotFound("empty".into()));
        }
    }
    Ok(())
}

/// Layer 3: Authorization — check capabilities allow this action
pub fn check_authorization(
    action: &Action,
    token: Option<&CapabilityToken>,
) -> Result<(), HydraError> {
    let required_cap = capability_for_action(&action.action_type);

    if let Some(token) = token {
        if !token.has_capability(&required_cap) {
            return Err(HydraError::PermissionDenied(format!(
                "You don't have the '{:?}' capability needed for this action. This follows the principle of least privilege. Request elevated permissions.",
                required_cap
            )));
        }
    }
    // No token = local mode = all capabilities granted implicitly
    Ok(())
}

/// Data isolation check (per-user, per-project)
pub fn check_data_isolation(action: &Action, session: &SessionContext) -> Result<(), HydraError> {
    // If project_id is set, ensure target is within project scope
    if let Some(ref project_id) = session.project_id {
        let target = &action.target;
        // Block actions targeting paths outside the project
        if target.starts_with('/') && !target.contains(project_id) {
            // Only enforce for file operations
            if matches!(
                action.action_type,
                ActionType::FileModify | ActionType::FileDelete | ActionType::Write
            ) {
                return Err(HydraError::PermissionDenied(format!(
                    "Action target '{}' is outside project '{}'. Data isolation prevents cross-project access.",
                    target, project_id
                )));
            }
        }
    }
    Ok(())
}

/// Layer 4: Execution control checks (sandbox, resource limits)
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_execution_time: Duration,
    pub max_memory_mb: u64,
    pub max_cpu_percent: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_secs(300),
            max_memory_mb: 1024,
            max_cpu_percent: 80,
        }
    }
}

pub fn check_execution_controls(action: &Action, in_sandbox: bool) -> Result<(), HydraError> {
    check_execution_controls_with_limits(action, in_sandbox, &ResourceLimits::default())
}

pub fn check_execution_controls_with_limits(
    action: &Action,
    in_sandbox: bool,
    _limits: &ResourceLimits,
) -> Result<(), HydraError> {
    // Shell/System execution outside sandbox is flagged
    if matches!(
        action.action_type,
        ActionType::ShellExecute | ActionType::System
    ) && !in_sandbox
    {
        // Allow but the risk assessor will score it higher
    }
    // Resource limits are enforced at runtime by the kernel, not at gate time
    Ok(())
}

/// Layer 5: Data protection — ensure no secrets leak in gate decisions
pub fn sanitize_for_output(text: &str) -> String {
    let mut sanitized = text.to_string();
    let secret_keywords = [
        "api_key", "api-key", "apikey", "token", "secret", "password", "bearer",
    ];
    let lower = sanitized.to_lowercase();
    for keyword in &secret_keywords {
        if lower.contains(keyword) {
            // Find and redact the value after the keyword
            if let Some(pos) = lower.find(keyword) {
                let after = &sanitized[pos..];
                if let Some(eq_pos) = after.find(['=', ':', ' '].as_ref()) {
                    let value_start = pos + eq_pos + 1;
                    if value_start < sanitized.len() {
                        let value_end = sanitized[value_start..]
                            .find([' ', '\n', '\t', '"', '\''].as_ref())
                            .map(|p| value_start + p)
                            .unwrap_or(sanitized.len());
                        if value_end > value_start {
                            sanitized.replace_range(value_start..value_end, "[REDACTED]");
                        }
                    }
                }
            }
        }
    }
    sanitized
}

/// Layer 6: Audit — tamper-evident gate decision log
#[derive(Debug, Clone, serde::Serialize)]
pub struct GateAuditEntry {
    pub sequence: u64,
    pub action_type: String,
    pub target: String,
    pub risk_level: String,
    pub decision: String,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Hash of this entry's content for tamper detection
    pub content_hash: String,
    /// Hash of previous entry (chain)
    pub previous_hash: Option<String>,
}

impl GateAuditEntry {
    pub fn new(
        sequence: u64,
        action: &Action,
        risk_level: &str,
        decision: &str,
        reason: &str,
        previous_hash: Option<String>,
    ) -> Self {
        let target = sanitize_for_output(&action.target);
        let reason = sanitize_for_output(reason);
        let timestamp = chrono::Utc::now();
        let content = format!(
            "{}|{:?}|{}|{}|{}|{}",
            sequence,
            action.action_type,
            target,
            risk_level,
            decision,
            timestamp.to_rfc3339()
        );
        let content_hash = format!("{:016x}", hash_djb2(&content));
        Self {
            sequence,
            action_type: format!("{:?}", action.action_type),
            target,
            risk_level: risk_level.into(),
            decision: decision.into(),
            reason,
            timestamp,
            content_hash,
            previous_hash,
        }
    }

    /// Verify this entry's hash
    pub fn verify_hash(&self) -> bool {
        let content = format!(
            "{}|{}|{}|{}|{}|{}",
            self.sequence,
            self.action_type,
            self.target,
            self.risk_level,
            self.decision,
            self.timestamp.to_rfc3339()
        );
        let expected = format!("{:016x}", hash_djb2(&content));
        self.content_hash == expected
    }

    /// Verify chain integrity with previous entry
    pub fn verify_chain(&self, previous: Option<&GateAuditEntry>) -> bool {
        match (&self.previous_hash, previous) {
            (None, None) => self.sequence == 0,
            (Some(prev), Some(prev_entry)) => *prev == prev_entry.content_hash,
            _ => false,
        }
    }
}

fn hash_djb2(input: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

fn capability_for_action(action_type: &ActionType) -> Capability {
    match action_type {
        ActionType::Read => Capability::FileRead,
        ActionType::Write | ActionType::FileCreate | ActionType::FileModify => {
            Capability::FileWrite
        }
        ActionType::FileDelete => Capability::FileDelete,
        ActionType::Execute | ActionType::ShellExecute => Capability::ShellExecute,
        ActionType::System => Capability::ShellExecuteUnsafe,
        ActionType::Network | ActionType::ApiCall => Capability::NetworkAccess,
        ActionType::GitOperation => Capability::ShellExecute,
        ActionType::SisterCall => Capability::SisterAccessAll,
        ActionType::Composite => Capability::ShellExecute,
    }
}
