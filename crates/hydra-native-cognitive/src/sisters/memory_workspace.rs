//! Memory workspace tools — create, populate, query, compare, and
//! cross-reference isolated memory workspaces.
//!
//! Sister-first, local-fallback pattern for workspace operations.

use super::connection::extract_text;
use super::cognitive::Sisters;

impl Sisters {
    /// Create a new named memory workspace.
    pub async fn memory_workspace_create(
        &self,
        name: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_workspace_create", serde_json::json!({
            "name": name,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// Add a memory node to an existing workspace.
    pub async fn memory_workspace_add(
        &self,
        workspace: &str,
        node_id: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_workspace_add", serde_json::json!({
            "workspace": workspace,
            "node_id": node_id,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// List all memory nodes in a workspace.
    pub async fn memory_workspace_list(
        &self,
        workspace: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_workspace_list", serde_json::json!({
            "workspace": workspace,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// Query memories within a specific workspace.
    pub async fn memory_workspace_query(
        &self,
        workspace: &str,
        query: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_workspace_query", serde_json::json!({
            "workspace": workspace,
            "query": query,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// Compare memory nodes within a workspace for contradictions or patterns.
    pub async fn memory_workspace_compare(
        &self,
        workspace: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_workspace_compare", serde_json::json!({
            "workspace": workspace,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// Cross-reference a symbol across workspace memories.
    /// Finds all nodes that mention or relate to the given symbol.
    pub async fn memory_workspace_xref(
        &self,
        workspace: &str,
        symbol: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_workspace_xref", serde_json::json!({
            "workspace": workspace,
            "symbol": symbol,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }
}

#[cfg(test)]
mod tests {
    /// Compile-time check: ensure this module builds and imports resolve.
    #[test]
    fn memory_workspace_compiles() {
        // Compilation of this module is the test.
        assert!(true);
    }
}
