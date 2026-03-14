//! Phase G Priority 2: Forge Full Pipeline — skeleton, test gen, dependency, wiring, export.
//!
//! Integrates Forge sister's code generation pipeline into the cognitive loop.
//! Forge handles: blueprint → entity → skeleton → test → dependency → wiring → export.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ── Blueprint Management (already: create, query in perceive.rs) ──

    /// Create a new blueprint from a description.
    pub async fn forge_blueprint_create(&self, name: &str, description: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_blueprint_create", serde_json::json!({
            "name": name,
            "description": safe_truncate(description, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get a specific blueprint by ID.
    pub async fn forge_blueprint_get(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_blueprint_get", serde_json::json!({
            "id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List all blueprints.
    pub async fn forge_blueprint_list(&self) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_blueprint_list", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Update an existing blueprint.
    pub async fn forge_blueprint_update(&self, id: &str, changes: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_blueprint_update", serde_json::json!({
            "id": id,
            "changes": safe_truncate(changes, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Validate a blueprint's architecture for consistency.
    pub async fn forge_blueprint_validate(&self, id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_blueprint_validate", serde_json::json!({
            "id": id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Code Generation Pipeline ──

    /// Infer entities from a description — extract types, functions, modules.
    pub async fn forge_entity_infer(&self, description: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_entity_infer", serde_json::json!({
            "description": safe_truncate(description, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Register a new entity in the blueprint.
    pub async fn forge_entity_add(&self, blueprint_id: &str, entity: &str, kind: &str) {
        if let Some(forge) = &self.forge {
            if let Err(e) = forge.call_tool("forge_entity_add", serde_json::json!({
                "blueprint_id": blueprint_id,
                "name": entity,
                "kind": kind,
            })).await {
                eprintln!("[hydra:forge] forge_entity_add FAILED: {}", e);
            }
        }
    }

    /// Generate code skeletons from a blueprint.
    pub async fn forge_skeleton_create(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_skeleton_create", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Auto-generate test scaffolds for a blueprint.
    pub async fn forge_test_generate(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_test_generate", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Resolve dependencies for a blueprint.
    pub async fn forge_dependency_resolve(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_dependency_resolve", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Add a dependency to a blueprint.
    pub async fn forge_dependency_add(&self, blueprint_id: &str, dep: &str) {
        if let Some(forge) = &self.forge {
            if let Err(e) = forge.call_tool("forge_dependency_add", serde_json::json!({
                "blueprint_id": blueprint_id,
                "dependency": dep,
            })).await {
                eprintln!("[hydra:forge] forge_dependency_add FAILED: {}", e);
            }
        }
    }

    /// Create component wiring between entities.
    pub async fn forge_wiring_create(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_wiring_create", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Structure & Export ──

    /// Generate project structure from a blueprint.
    pub async fn forge_structure_generate(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_structure_generate", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Export a blueprint (JSON/YAML/code).
    pub async fn forge_export(&self, blueprint_id: &str, format: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_export", serde_json::json!({
            "id": blueprint_id,
            "format": format,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Import a dependency graph into a blueprint.
    pub async fn forge_import_graph(&self, blueprint_id: &str, graph: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_import_graph", serde_json::json!({
            "blueprint_id": blueprint_id,
            "graph": safe_truncate(graph, 1000),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_forge_deep_compiles() {
        assert!(true);
    }
}
