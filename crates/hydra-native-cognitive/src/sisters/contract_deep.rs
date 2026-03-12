//! Priority 2: Deep Contract Integration — sister-first risk assessment,
//! precognition, crystallized receipts, approval audit trail.
//!
//! Replaces homegrown keyword-based gate with Contract sister's policy engine.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

/// Contract sister risk assessment result.
#[derive(Debug, Clone)]
pub struct ContractAssessment {
    pub risk_level: String,
    pub allowed: bool,
    pub reason: String,
    pub policy_id: Option<String>,
}

impl Sisters {
    /// DECIDE: Pre-action risk assessment via Contract sister.
    /// Falls back to None if Contract sister is offline (caller uses local gate).
    pub async fn contract_precognition(&self, action: &str) -> Option<ContractAssessment> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_precognition", serde_json::json!({
            "planned_action": action,
            "context": "cognitive_loop",
        })).await.ok()?;

        let text = extract_text(&result);
        if text.is_empty() {
            return None;
        }

        // Parse Contract sister response
        let risk_level = result.get("risk_level")
            .and_then(|v| v.as_str())
            .unwrap_or("medium")
            .to_string();
        let allowed = result.get("allowed")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let reason = result.get("reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| text);
        let policy_id = result.get("policy_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Some(ContractAssessment { risk_level, allowed, reason, policy_id })
    }

    /// ACT: Crystallize a receipt after action execution.
    /// Records the action, outcome, and risk level in Contract sister's audit trail.
    pub async fn contract_crystallize(
        &self,
        action: &str,
        outcome: &str,
        risk_level: &str,
        success: bool,
    ) {
        if let Some(contract) = &self.contract {
            let _ = contract.call_tool("contract_crystallize", serde_json::json!({
                "action": safe_truncate(action, 200),
                "outcome": safe_truncate(outcome, 200),
                "risk_level": risk_level,
                "success": success,
                "source": "cognitive_loop",
            })).await;
        }
    }

    /// DECIDE: Create an approval request via Contract sister.
    /// Returns an approval_id that can be tracked.
    pub async fn contract_request_approval(
        &self,
        action: &str,
        risk_level: &str,
        reason: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_approval_request", serde_json::json!({
            "action": safe_truncate(action, 200),
            "risk_level": risk_level,
            "reason": reason,
        })).await.ok()?;
        result.get("approval_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// DECIDE: Record an approval decision via Contract sister.
    pub async fn contract_record_decision(
        &self,
        approval_id: &str,
        approved: bool,
        reason: &str,
    ) {
        if let Some(contract) = &self.contract {
            let _ = contract.call_tool("contract_approval_decide", serde_json::json!({
                "approval_id": approval_id,
                "approved": approved,
                "reason": reason,
            })).await;
        }
    }

    /// PERCEIVE: Query approval history for "what have I approved" questions.
    pub async fn contract_query_approvals(
        &self,
        time_range: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_query", serde_json::json!({
            "type": "approval",
            "time_range": time_range,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// PERCEIVE: Query policy constraints for the current context.
    pub async fn contract_policy_check(&self, context: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_policy_check", serde_json::json!({
            "context": context,
            "action_type": "query",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_assessment_fields() {
        let a = ContractAssessment {
            risk_level: "high".into(),
            allowed: false,
            reason: "Destructive operation".into(),
            policy_id: Some("policy-001".into()),
        };
        assert!(!a.allowed);
        assert_eq!(a.risk_level, "high");
        assert!(a.policy_id.is_some());
    }
}
