//! Contract Workspace — multi-contract workspaces, session management.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    /// Create a new contract workspace.
    pub async fn contract_workspace_create(&self, name: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_workspace_create", serde_json::json!({
            "name": safe_truncate(name, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Add a contract to a workspace.
    pub async fn contract_workspace_add(
        &self,
        workspace_id: &str,
        contract_id: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_workspace_add", serde_json::json!({
            "workspace_id": safe_truncate(workspace_id, 500),
            "contract_id": safe_truncate(contract_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List contracts in a workspace.
    pub async fn contract_workspace_list(
        &self,
        workspace_id: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_workspace_list", serde_json::json!({
            "workspace_id": safe_truncate(workspace_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query a workspace.
    pub async fn contract_workspace_query(
        &self,
        workspace_id: &str,
        query: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_workspace_query", serde_json::json!({
            "workspace_id": safe_truncate(workspace_id, 500),
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Compare contracts in a workspace by aspect.
    pub async fn contract_workspace_compare(
        &self,
        workspace_id: &str,
        aspect: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_workspace_compare", serde_json::json!({
            "workspace_id": safe_truncate(workspace_id, 500),
            "aspect": safe_truncate(aspect, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Cross-reference contracts in a workspace.
    pub async fn contract_workspace_xref(
        &self,
        workspace_id: &str,
        reference: &str,
    ) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_workspace_xref", serde_json::json!({
            "workspace_id": safe_truncate(workspace_id, 500),
            "reference": safe_truncate(reference, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Start a contract session, optionally for a specific agent.
    pub async fn contract_session_start(&self, agent_id: &str) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("session_start", serde_json::json!({
            "agent_id": safe_truncate(agent_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// End a contract session with a summary (fire-and-forget).
    pub async fn contract_session_end(&self, summary: &str) {
        if let Some(contract) = &self.contract {
            if let Err(e) = contract.call_tool("session_end", serde_json::json!({
                "summary": safe_truncate(summary, 500),
            })).await {
                eprintln!("[hydra:contract] session_end FAILED: {}", e);
            }
        }
    }

    /// Resume a previous contract session.
    pub async fn contract_session_resume(&self) -> Option<String> {
        let contract = self.contract.as_ref()?;
        let result = contract.call_tool("contract_session_resume", serde_json::json!({}))
            .await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_contract_workspace_compiles() {
        assert!(true);
    }
}
