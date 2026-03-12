use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;

use crate::bridge::*;

/// Generic MCP-based sister bridge.
/// Each sister bridge wraps MCP tool calls to its sister's MCP server.
pub struct McpSisterBridge {
    pub(crate) id: SisterId,
    pub(crate) bridge_name: &'static str,
    pub(crate) bridge_version: &'static str,
    pub(crate) caps: Vec<String>,
    pub(crate) available: AtomicBool,
    pub(crate) timeout: Duration,
}

impl McpSisterBridge {
    pub(crate) fn new(id: SisterId, name: &'static str, version: &'static str, capabilities: &[&str]) -> Self {
        Self {
            id,
            bridge_name: name,
            bridge_version: version,
            caps: capabilities.iter().map(|s| s.to_string()).collect(),
            available: AtomicBool::new(true),
            timeout: Duration::from_secs(5),
        }
    }

    pub fn set_available(&self, available: bool) {
        self.available.store(available, Ordering::SeqCst);
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
}

#[async_trait]
impl SisterBridge for McpSisterBridge {
    fn sister_id(&self) -> SisterId {
        self.id
    }

    fn name(&self) -> &str {
        self.bridge_name
    }

    fn version(&self) -> &str {
        self.bridge_version
    }

    async fn health_check(&self) -> HealthStatus {
        if self.available.load(Ordering::SeqCst) {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unavailable
        }
    }

    async fn call(&self, action: SisterAction) -> Result<SisterResult, SisterError> {
        if !self.available.load(Ordering::SeqCst) {
            return Err(SisterError {
                sister_id: self.id,
                message: format!(
                    "{} is not available. The service may be offline or not configured.",
                    self.bridge_name
                ),
                retryable: true,
            });
        }

        // Simulate MCP tool call (real implementation uses LiveMcpBridge)
        let result = tokio::time::timeout(self.timeout, async {
            // Simulated response — in production this calls the sister's MCP server
            SisterResult {
                data: serde_json::json!({
                    "sister": self.bridge_name,
                    "tool": action.tool,
                    "status": "ok",
                }),
                tokens_used: 0,
            }
        })
        .await
        .map_err(|_| SisterError {
            sister_id: self.id,
            message: format!(
                "{} timed out after {:?}. The sister may be overloaded.",
                self.bridge_name, self.timeout
            ),
            retryable: true,
        })?;

        Ok(result)
    }

    async fn batch_call(
        &self,
        actions: Vec<SisterAction>,
    ) -> Vec<Result<SisterResult, SisterError>> {
        // Batch execution — single context, multiple operations
        let mut results = Vec::with_capacity(actions.len());
        for action in actions {
            results.push(self.call(action).await);
        }
        results
    }

    fn capabilities(&self) -> Vec<String> {
        self.caps.clone()
    }
}

// ═══════════════════════════════════════════════════════════
// FOUNDATION BRIDGE CONSTRUCTORS (7) — memory, vision, codebase, identity
// ═══════════════════════════════════════════════════════════

pub fn memory_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Memory,
        "agentic-memory",
        "0.5.0",
        &[
            // V2 core operations
            "memory_add",
            "memory_query",
            "memory_similar",
            "memory_temporal",
            "memory_context",
            "memory_traverse",
            "memory_correct",
            "memory_resolve",
            "memory_causal",
            "memory_quality",
            "memory_stats",
            // Grounding
            "memory_ground",
            "memory_evidence",
            "memory_suggest",
            // Session
            "session_start",
            "session_end",
            "memory_session_resume",
            // Conversation
            "conversation_log",
            // V3 capture tools (Universal Fix — active capture)
            "memory_capture_message",
            "memory_capture_tool",
            "memory_capture_file",
            "memory_capture_decision",
            "memory_capture_boundary",
            // V3 retrieval tools
            "memory_retrieve",
            "memory_resurrect",
            "memory_v3_session_resume",
            "memory_search_temporal",
            "memory_search_semantic",
            "memory_search_entity",
            "memory_v3_stats",
            // V4 longevity tools (20-year memory hierarchy)
            "memory_longevity_stats",
            "memory_longevity_search",
            "memory_longevity_consolidate",
            "memory_longevity_health",
            "memory_hierarchy_query",
            "memory_hierarchy_navigate",
            "memory_hierarchy_significance",
            "memory_embedding_status",
            // Capability reporting (Universal Fix — honest reporting)
            "memory_capabilities",
            // Workspace
            "memory_workspace_create",
            "memory_workspace_switch",
            "memory_workspace_list",
            "memory_workspace_delete",
            "memory_workspace_export",
            "memory_workspace_import",
        ],
    )
}

pub fn vision_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Vision,
        "agentic-vision",
        "0.3.0",
        &[
            // Core operations
            "vision_capture",
            "vision_compare",
            "vision_query",
            "vision_ocr",
            "vision_similar",
            "vision_track",
            "vision_diff",
            "vision_health",
            "vision_link",
            // Grounding
            "vision_ground",
            "vision_evidence",
            "vision_suggest",
            // Observation
            "observation_log",
            // Workspace
            "vision_workspace_create",
            "vision_workspace_switch",
            "vision_workspace_list",
            "vision_workspace_delete",
            "vision_workspace_export",
            "vision_workspace_import",
        ],
    )
}

pub fn codebase_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Codebase,
        "agentic-codebase",
        "0.3.0",
        &[
            // Core operations
            "search_semantic",
            "codebase_session",
            "concept_find",
            "concept_map",
            "impact_analyze",
            "pattern_extract",
            "genetics_dna",
            "omniscience_search",
            "prophecy_if",
            "graph_stats",
            // Analysis
            "analysis_log",
            // Workspace
            "codebase_workspace_create",
            "codebase_workspace_switch",
            "codebase_workspace_list",
            "codebase_workspace_delete",
            "codebase_workspace_export",
            "codebase_workspace_import",
        ],
    )
}

pub fn identity_bridge() -> McpSisterBridge {
    McpSisterBridge::new(
        SisterId::Identity,
        "agentic-identity",
        "0.3.0",
        &[
            // Identity
            "identity_create",
            "identity_show",
            // Actions
            "action_sign",
            "action_context",
            // Trust
            "trust_grant",
            "trust_verify",
            "trust_revoke",
            // Receipts
            "receipt_create",
            "receipt_verify",
            // Continuity
            "continuity_record",
            "continuity_verify",
            // Spawning
            "spawn_create",
            "spawn_verify",
            // Competence
            "competence_record",
            "competence_query",
            // Negative knowledge
            "negative_prove",
            "negative_verify",
        ],
    )
}
