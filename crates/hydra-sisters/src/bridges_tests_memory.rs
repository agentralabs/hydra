#[cfg(test)]
mod tests {
    use crate::bridge::*;
    use super::*;

    // ═══════════════════════════════════════════════════════════
    // UNIVERSAL FIX — V3/V4 Memory Bridge Tests
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_memory_bridge_v3_capture_tools() {
        let b = memory_bridge();
        let v3_capture = [
            "memory_capture_message",
            "memory_capture_tool",
            "memory_capture_file",
            "memory_capture_decision",
            "memory_capture_boundary",
        ];
        for tool in &v3_capture {
            assert!(b.caps.contains(&tool.to_string()),
                "V3 capture tool '{}' missing from memory bridge", tool);
        }
    }

    #[test]
    fn test_memory_bridge_v3_retrieval_tools() {
        let b = memory_bridge();
        let v3_retrieval = [
            "memory_retrieve",
            "memory_resurrect",
            "memory_v3_session_resume",
            "memory_search_temporal",
            "memory_search_semantic",
            "memory_search_entity",
            "memory_v3_stats",
        ];
        for tool in &v3_retrieval {
            assert!(b.caps.contains(&tool.to_string()),
                "V3 retrieval tool '{}' missing from memory bridge", tool);
        }
    }

    #[test]
    fn test_memory_bridge_v4_longevity_tools() {
        let b = memory_bridge();
        let v4_longevity = [
            "memory_longevity_stats",
            "memory_longevity_search",
            "memory_longevity_consolidate",
            "memory_longevity_health",
            "memory_hierarchy_query",
            "memory_hierarchy_navigate",
            "memory_hierarchy_significance",
            "memory_embedding_status",
        ];
        for tool in &v4_longevity {
            assert!(b.caps.contains(&tool.to_string()),
                "V4 longevity tool '{}' missing from memory bridge", tool);
        }
    }

    #[test]
    fn test_memory_bridge_capability_reporting() {
        let b = memory_bridge();
        assert!(b.caps.contains(&"memory_capabilities".to_string()),
            "memory_capabilities tool missing — honest reporting not available");
    }

    #[test]
    fn test_memory_bridge_v2_backward_compat() {
        let b = memory_bridge();
        // V2 tools must still be present for backward compatibility
        let v2_tools = [
            "memory_add", "memory_query", "memory_similar",
            "memory_temporal", "memory_context", "memory_traverse",
            "memory_correct", "memory_resolve", "memory_causal",
            "memory_quality", "memory_stats",
            "memory_ground", "memory_evidence", "memory_suggest",
            "session_start", "session_end", "memory_session_resume",
            "conversation_log",
        ];
        for tool in &v2_tools {
            assert!(b.caps.contains(&tool.to_string()),
                "V2 tool '{}' removed — backward compatibility broken", tool);
        }
    }

    #[test]
    fn test_memory_bridge_workspace_tools_preserved() {
        let b = memory_bridge();
        let workspace_tools = [
            "memory_workspace_create", "memory_workspace_switch",
            "memory_workspace_list", "memory_workspace_delete",
            "memory_workspace_export", "memory_workspace_import",
        ];
        for tool in &workspace_tools {
            assert!(b.caps.contains(&tool.to_string()),
                "Workspace tool '{}' missing", tool);
        }
    }

    #[test]
    fn test_memory_bridge_total_capabilities() {
        let b = memory_bridge();
        // V2 (24) + V3 capture (5) + V3 retrieval (7) + V4 longevity (8) + capabilities (1) + workspace (6) = 51
        // But some are counted in both V2 and workspace, so just check minimum
        assert!(b.caps.len() >= 40,
            "Memory bridge has only {} capabilities, expected >= 40", b.caps.len());
    }

    #[tokio::test]
    async fn test_memory_bridge_v3_tool_call_succeeds() {
        let b = memory_bridge();
        let action = SisterAction::new(
            "memory_capture_message",
            serde_json::json!({
                "role": "user",
                "content": "test message",
                "summary": "testing V3 capture",
            }),
        );
        let result = b.call(action).await;
        assert!(result.is_ok(), "V3 memory_capture_message call failed");
        let r = result.unwrap();
        assert_eq!(r.data["tool"], "memory_capture_message");
    }

    #[tokio::test]
    async fn test_memory_bridge_v4_tool_call_succeeds() {
        let b = memory_bridge();
        let action = SisterAction::new(
            "memory_longevity_search",
            serde_json::json!({
                "query": "test query",
                "limit": 5,
            }),
        );
        let result = b.call(action).await;
        assert!(result.is_ok(), "V4 memory_longevity_search call failed");
        let r = result.unwrap();
        assert_eq!(r.data["tool"], "memory_longevity_search");
    }

    #[tokio::test]
    async fn test_memory_bridge_capabilities_tool_call() {
        let b = memory_bridge();
        let action = SisterAction::new("memory_capabilities", serde_json::json!({}));
        let result = b.call(action).await;
        assert!(result.is_ok(), "memory_capabilities tool call failed");
    }

    #[tokio::test]
    async fn test_memory_bridge_v3_batch_capture() {
        let b = memory_bridge();
        let actions = vec![
            SisterAction::new("memory_capture_message", serde_json::json!({
                "role": "user", "content": "msg1"
            })),
            SisterAction::new("memory_capture_decision", serde_json::json!({
                "decision": "use Rust", "reasoning": "performance"
            })),
            SisterAction::new("memory_capture_boundary", serde_json::json!({
                "boundary_type": "session_end"
            })),
        ];
        let results = b.batch_call(actions).await;
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()),
            "Batch V3 capture failed: {:?}", results);
    }

    #[test]
    fn test_memory_bridge_no_duplicate_tools() {
        let b = memory_bridge();
        let mut seen = std::collections::HashSet::new();
        for cap in &b.caps {
            assert!(seen.insert(cap.clone()),
                "Duplicate tool in memory bridge: {}", cap);
        }
    }

    #[test]
    fn test_other_bridges_unchanged() {
        // Verify non-memory bridges weren't accidentally modified
        assert_eq!(vision_bridge().bridge_version, "0.3.0");
        assert_eq!(codebase_bridge().bridge_version, "0.3.0");
        assert_eq!(identity_bridge().bridge_version, "0.3.0");
        assert_eq!(time_bridge().bridge_version, "0.2.0");
        assert_eq!(contract_bridge().bridge_version, "0.2.0");
        assert_eq!(comm_bridge().bridge_version, "0.2.0");
        assert_eq!(planning_bridge().bridge_version, "0.2.0");
        assert_eq!(cognition_bridge().bridge_version, "0.2.0");
        assert_eq!(reality_bridge().bridge_version, "0.2.0");
        assert_eq!(forge_bridge().bridge_version, "0.1.0");
        assert_eq!(aegis_bridge().bridge_version, "0.1.0");
        assert_eq!(veritas_bridge().bridge_version, "0.1.0");
        assert_eq!(evolve_bridge().bridge_version, "0.1.0");
    }
}
