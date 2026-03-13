//! Contract Core — CRUD, policy management, risk limits, approval listing.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    /// Create a new contract with name, description, and parties.
    pub async fn contract_create(
        &self,
        name: &str,
        description: &str,
        parties: &[&str],
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_create", serde_json::json!({
            "name": safe_truncate(name, 500),
            "description": safe_truncate(description, 500),
            "parties": parties.iter().map(|p| safe_truncate(p, 500)).collect::<Vec<_>>(),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Sign a contract by ID.
    pub async fn contract_sign(&self, contract_id: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_sign", serde_json::json!({
            "contract_id": safe_truncate(contract_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Verify a contract by ID.
    pub async fn contract_verify(&self, contract_id: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_verify", serde_json::json!({
            "contract_id": safe_truncate(contract_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List contracts, optionally filtered.
    pub async fn contract_list(&self, filter: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_list", serde_json::json!({
            "filter": safe_truncate(filter, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get a contract by ID.
    pub async fn contract_get(&self, contract_id: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_get", serde_json::json!({
            "contract_id": safe_truncate(contract_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Add a policy with name and rules.
    pub async fn contract_policy_add(
        &self,
        name: &str,
        rules: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("policy_add", serde_json::json!({
            "name": safe_truncate(name, 500),
            "rules": safe_truncate(rules, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List all policies.
    pub async fn contract_policy_list(&self) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("policy_list", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Set a risk limit for a category (fire-and-forget).
    pub async fn contract_risk_limit_set(&self, category: &str, limit: &str) {
        if let Some(contract) = &self.contract {
            let _ = contract.call_tool("risk_limit_set", serde_json::json!({
                "category": safe_truncate(category, 500),
                "limit": safe_truncate(limit, 500),
            })).await;
        }
    }

    /// Check a risk limit for an action.
    pub async fn contract_risk_limit_check(&self, action: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("risk_limit_check", serde_json::json!({
            "action": safe_truncate(action, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List all risk limits.
    pub async fn contract_risk_limit_list(&self) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("risk_limit_list", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List approvals, optionally filtered by status.
    pub async fn contract_approval_list(&self, status: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("approval_list", serde_json::json!({
            "status": safe_truncate(status, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Decide on an approval request (approve or deny).
    pub async fn contract_approval_decide(
        &self,
        approval_id: &str,
        decision: &str,
        reason: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("approval_decide", serde_json::json!({
            "approval_id": safe_truncate(approval_id, 500),
            "decision": safe_truncate(decision, 500),
            "reason": safe_truncate(reason, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_contract_core_compiles() {
        assert!(true);
    }
}
