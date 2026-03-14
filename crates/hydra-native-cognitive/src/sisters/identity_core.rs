//! Identity Core — identity creation, signing, receipts, trust grants, sessions, grounding.

use super::cognitive::Sisters;
use super::connection::extract_text;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    /// Create a new identity with the given name.
    pub async fn identity_create(&self, name: &str) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("identity_create", serde_json::json!({
            "name": safe_truncate(name, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Show current identity details.
    pub async fn identity_show(&self) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("identity_show", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Sign an action with identity credentials.
    pub async fn identity_action_sign(&self, action: &str, data: &str) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("action_sign", serde_json::json!({
            "action": safe_truncate(action, 500),
            "data": safe_truncate(data, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Verify a receipt by its ID.
    pub async fn identity_receipt_verify(&self, receipt_id: &str) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("receipt_verify", serde_json::json!({
            "receipt_id": safe_truncate(receipt_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List receipts with optional filter.
    pub async fn identity_receipt_list(&self, filter: &str) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("receipt_list", serde_json::json!({
            "filter": safe_truncate(filter, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Grant trust to another identity for a capability.
    pub async fn identity_trust_grant(
        &self, to_identity: &str, capability: &str, duration: &str,
    ) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("trust_grant", serde_json::json!({
            "to_identity": safe_truncate(to_identity, 500),
            "capability": safe_truncate(capability, 500),
            "duration": safe_truncate(duration, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Revoke a previously granted trust.
    pub async fn identity_trust_revoke(&self, grant_id: &str) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("trust_revoke", serde_json::json!({
            "grant_id": safe_truncate(grant_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Verify whether an identity holds a specific capability.
    pub async fn identity_trust_verify_cap(
        &self, identity: &str, capability: &str,
    ) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("trust_verify", serde_json::json!({
            "identity": safe_truncate(identity, 500),
            "capability": safe_truncate(capability, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List trust grants by direction (granted or received).
    pub async fn identity_trust_list(&self, direction: &str) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("trust_list", serde_json::json!({
            "direction": safe_truncate(direction, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Record action context for identity audit trail.
    pub async fn identity_action_context(&self, action: &str, context: &str) {
        if let Some(id) = &self.identity {
            if let Err(e) = id.call_tool("action_context", serde_json::json!({
                "action": safe_truncate(action, 500),
                "context": safe_truncate(context, 500),
            })).await {
                eprintln!("[hydra:identity] action_context FAILED: {}", e);
            }
        }
    }

    /// Start a new identity session.
    pub async fn identity_session_start(&self) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("identity_session_start", serde_json::json!({}))
            .await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// End the current identity session with a summary.
    pub async fn identity_session_end(&self, summary: &str) {
        if let Some(id) = &self.identity {
            if let Err(e) = id.call_tool("identity_session_end", serde_json::json!({
                "summary": safe_truncate(summary, 500),
            })).await {
                eprintln!("[hydra:identity] identity_session_end FAILED: {}", e);
            }
        }
    }

    /// Resume a previously started identity session.
    pub async fn identity_session_resume(&self) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("identity_session_resume", serde_json::json!({}))
            .await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Ground a claim against identity evidence.
    pub async fn identity_ground(&self, claim: &str) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("identity_ground", serde_json::json!({
            "claim": safe_truncate(claim, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query identity evidence store.
    pub async fn identity_evidence(&self, query: &str) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("identity_evidence", serde_json::json!({
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get identity-based suggestions for a query.
    pub async fn identity_suggest(&self, query: &str) -> Option<String> {
        let id = self.identity.as_ref()?;
        let result = id.call_tool("identity_suggest", serde_json::json!({
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_identity_core_compiles() {
        assert!(true);
    }
}
