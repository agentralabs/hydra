//! Contract Extended — conditions, obligations, violations, grounding, analytics.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    /// Add a condition to a contract.
    pub async fn contract_condition_add(
        &self,
        contract_id: &str,
        condition: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("condition_add", serde_json::json!({
            "contract_id": safe_truncate(contract_id, 500),
            "condition": safe_truncate(condition, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Evaluate conditions on a contract.
    pub async fn contract_condition_evaluate(
        &self,
        contract_id: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("condition_evaluate", serde_json::json!({
            "contract_id": safe_truncate(contract_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Add an obligation to a contract.
    pub async fn contract_obligation_add(
        &self,
        contract_id: &str,
        obligation: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("obligation_add", serde_json::json!({
            "contract_id": safe_truncate(contract_id, 500),
            "obligation": safe_truncate(obligation, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Check obligations on a contract.
    pub async fn contract_obligation_check(
        &self,
        contract_id: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("obligation_check", serde_json::json!({
            "contract_id": safe_truncate(contract_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List violations, optionally filtered by contract ID.
    pub async fn contract_violation_list(
        &self,
        contract_id: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("violation_list", serde_json::json!({
            "contract_id": safe_truncate(contract_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Report a violation on a contract.
    pub async fn contract_violation_report(
        &self,
        contract_id: &str,
        description: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("violation_report", serde_json::json!({
            "contract_id": safe_truncate(contract_id, 500),
            "description": safe_truncate(description, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Log context for a contract action (fire-and-forget).
    pub async fn contract_context_log(&self, context: &str, action: &str) {
        if let Some(contract) = &self.contract {
            let _ = contract.call_tool("contract_context_log", serde_json::json!({
                "context": safe_truncate(context, 500),
                "action": safe_truncate(action, 500),
            })).await;
        }
    }

    /// Get contract statistics.
    pub async fn contract_stats(&self) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_stats", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Ground a claim against contract data.
    pub async fn contract_ground(&self, claim: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_ground", serde_json::json!({
            "claim": safe_truncate(claim, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query contract evidence.
    pub async fn contract_evidence(&self, query: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_evidence", serde_json::json!({
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get suggestions based on context.
    pub async fn contract_suggest(&self, context: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_suggest", serde_json::json!({
            "context": safe_truncate(context, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_contract_extended_compiles() {
        assert!(true);
    }
}
