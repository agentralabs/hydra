#[cfg(test)]
mod tests {
    use crate::bridge::*;

    // ── SisterId tests ─────────────────────────────────────

    #[test]
    fn test_sister_id_all_returns_14() {
        assert_eq!(SisterId::all().len(), 14);
    }

    #[test]
    fn test_sister_id_all_contains_every_variant() {
        let all = SisterId::all();
        assert!(all.contains(&SisterId::Memory));
        assert!(all.contains(&SisterId::Vision));
        assert!(all.contains(&SisterId::Codebase));
        assert!(all.contains(&SisterId::Identity));
        assert!(all.contains(&SisterId::Time));
        assert!(all.contains(&SisterId::Contract));
        assert!(all.contains(&SisterId::Comm));
        assert!(all.contains(&SisterId::Planning));
        assert!(all.contains(&SisterId::Cognition));
        assert!(all.contains(&SisterId::Reality));
        assert!(all.contains(&SisterId::Forge));
        assert!(all.contains(&SisterId::Aegis));
        assert!(all.contains(&SisterId::Veritas));
        assert!(all.contains(&SisterId::Evolve));
    }

    #[test]
    fn test_sister_id_name_memory() {
        assert_eq!(SisterId::Memory.name(), "agentic-memory");
    }

    #[test]
    fn test_sister_id_name_vision() {
        assert_eq!(SisterId::Vision.name(), "agentic-vision");
    }

    #[test]
    fn test_sister_id_name_codebase() {
        assert_eq!(SisterId::Codebase.name(), "agentic-codebase");
    }

    #[test]
    fn test_sister_id_name_identity() {
        assert_eq!(SisterId::Identity.name(), "agentic-identity");
    }

    #[test]
    fn test_sister_id_name_time() {
        assert_eq!(SisterId::Time.name(), "agentic-time");
    }

    #[test]
    fn test_sister_id_name_contract() {
        assert_eq!(SisterId::Contract.name(), "agentic-contract");
    }

    #[test]
    fn test_sister_id_name_comm() {
        assert_eq!(SisterId::Comm.name(), "agentic-comm");
    }

    #[test]
    fn test_sister_id_name_planning() {
        assert_eq!(SisterId::Planning.name(), "agentic-planning");
    }

    #[test]
    fn test_sister_id_name_cognition() {
        assert_eq!(SisterId::Cognition.name(), "agentic-cognition");
    }

    #[test]
    fn test_sister_id_name_reality() {
        assert_eq!(SisterId::Reality.name(), "agentic-reality");
    }

    #[test]
    fn test_sister_id_name_forge() {
        assert_eq!(SisterId::Forge.name(), "agentic-forge");
    }

    #[test]
    fn test_sister_id_name_aegis() {
        assert_eq!(SisterId::Aegis.name(), "agentic-aegis");
    }

    #[test]
    fn test_sister_id_name_veritas() {
        assert_eq!(SisterId::Veritas.name(), "agentic-veritas");
    }

    #[test]
    fn test_sister_id_name_evolve() {
        assert_eq!(SisterId::Evolve.name(), "agentic-evolve");
    }

    // ── Foundation classification ──────────────────────────

    #[test]
    fn test_foundation_sisters_count() {
        let count = SisterId::all().iter().filter(|s| s.is_foundation()).count();
        assert_eq!(count, 7);
    }

    #[test]
    fn test_memory_is_foundation() {
        assert!(SisterId::Memory.is_foundation());
    }

    #[test]
    fn test_vision_is_foundation() {
        assert!(SisterId::Vision.is_foundation());
    }

    #[test]
    fn test_codebase_is_foundation() {
        assert!(SisterId::Codebase.is_foundation());
    }

    #[test]
    fn test_identity_is_foundation() {
        assert!(SisterId::Identity.is_foundation());
    }

    #[test]
    fn test_time_is_foundation() {
        assert!(SisterId::Time.is_foundation());
    }

    #[test]
    fn test_contract_is_foundation() {
        assert!(SisterId::Contract.is_foundation());
    }

    #[test]
    fn test_comm_is_foundation() {
        assert!(SisterId::Comm.is_foundation());
    }

    // ── Cognitive classification ───────────────────────────

    #[test]
    fn test_cognitive_sisters_count() {
        let count = SisterId::all().iter().filter(|s| s.is_cognitive()).count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_planning_is_cognitive() {
        assert!(SisterId::Planning.is_cognitive());
    }

    #[test]
    fn test_cognition_is_cognitive() {
        assert!(SisterId::Cognition.is_cognitive());
    }

    #[test]
    fn test_reality_is_cognitive() {
        assert!(SisterId::Reality.is_cognitive());
    }

    // ── Astral classification ──────────────────────────────

    #[test]
    fn test_astral_sisters_count() {
        let count = SisterId::all().iter().filter(|s| s.is_astral()).count();
        assert_eq!(count, 4);
    }

    #[test]
    fn test_forge_is_astral() {
        assert!(SisterId::Forge.is_astral());
    }

    #[test]
    fn test_aegis_is_astral() {
        assert!(SisterId::Aegis.is_astral());
    }

    #[test]
    fn test_veritas_is_astral() {
        assert!(SisterId::Veritas.is_astral());
    }

    #[test]
    fn test_evolve_is_astral() {
        assert!(SisterId::Evolve.is_astral());
    }

    // ── Mutual exclusivity ─────────────────────────────────

    #[test]
    fn test_foundation_is_not_cognitive() {
        for s in SisterId::all() {
            if s.is_foundation() {
                assert!(!s.is_cognitive(), "{:?} is both foundation and cognitive", s);
            }
        }
    }

    #[test]
    fn test_foundation_is_not_astral() {
        for s in SisterId::all() {
            if s.is_foundation() {
                assert!(!s.is_astral(), "{:?} is both foundation and astral", s);
            }
        }
    }

    #[test]
    fn test_cognitive_is_not_astral() {
        for s in SisterId::all() {
            if s.is_cognitive() {
                assert!(!s.is_astral(), "{:?} is both cognitive and astral", s);
            }
        }
    }

    #[test]
    fn test_every_sister_has_exactly_one_category() {
        for s in SisterId::all() {
            let cats = [s.is_foundation(), s.is_cognitive(), s.is_astral()];
            let count = cats.iter().filter(|&&b| b).count();
            assert_eq!(count, 1, "{:?} has {} categories", s, count);
        }
    }

    // ── SisterAction tests ─────────────────────────────────

    #[test]
    fn test_sister_action_new() {
        let action = SisterAction::new("memory_add", serde_json::json!({"content": "test"}));
        assert_eq!(action.tool, "memory_add");
        assert_eq!(action.params["content"], "test");
    }

    #[test]
    fn test_sister_action_new_from_string() {
        let action = SisterAction::new(String::from("vision_capture"), serde_json::json!({}));
        assert_eq!(action.tool, "vision_capture");
    }

    #[test]
    fn test_sister_action_serialization() {
        let action = SisterAction::new("test_tool", serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&action).unwrap();
        let restored: SisterAction = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tool, "test_tool");
        assert_eq!(restored.params["key"], "value");
    }

    // ── SisterResult tests ─────────────────────────────────

    #[test]
    fn test_sister_result_serialization() {
        let result = SisterResult {
            data: serde_json::json!({"status": "ok"}),
            tokens_used: 42,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: SisterResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tokens_used, 42);
        assert_eq!(restored.data["status"], "ok");
    }

    // ── SisterError tests ──────────────────────────────────

    #[test]
    fn test_sister_error_display_retryable() {
        let err = SisterError {
            sister_id: SisterId::Memory,
            message: "timeout".into(),
            retryable: true,
        };
        let display = format!("{}", err);
        assert!(display.contains("agentic-memory"));
        assert!(display.contains("timeout"));
        assert!(display.contains("temporary"));
    }

    #[test]
    fn test_sister_error_display_non_retryable() {
        let err = SisterError {
            sister_id: SisterId::Vision,
            message: "config error".into(),
            retryable: false,
        };
        let display = format!("{}", err);
        assert!(display.contains("agentic-vision"));
        assert!(display.contains("Check sister status"));
    }

    #[test]
    fn test_sister_error_is_std_error() {
        let err = SisterError {
            sister_id: SisterId::Memory,
            message: "test".into(),
            retryable: false,
        };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_sister_error_serialization() {
        let err = SisterError {
            sister_id: SisterId::Forge,
            message: "not found".into(),
            retryable: true,
        };
        let json = serde_json::to_string(&err).unwrap();
        let restored: SisterError = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.sister_id, SisterId::Forge);
        assert!(restored.retryable);
    }

    // ── HealthStatus tests ─────────────────────────────────

    #[test]
    fn test_health_status_serialization() {
        let healthy = HealthStatus::Healthy;
        let json = serde_json::to_string(&healthy).unwrap();
        assert_eq!(json, "\"healthy\"");

        let degraded = HealthStatus::Degraded;
        let json = serde_json::to_string(&degraded).unwrap();
        assert_eq!(json, "\"degraded\"");

        let unavailable = HealthStatus::Unavailable;
        let json = serde_json::to_string(&unavailable).unwrap();
        assert_eq!(json, "\"unavailable\"");
    }

    #[test]
    fn test_health_status_deserialization() {
        let h: HealthStatus = serde_json::from_str("\"healthy\"").unwrap();
        assert_eq!(h, HealthStatus::Healthy);
    }

    // ── SisterId serde ─────────────────────────────────────

    #[test]
    fn test_sister_id_serde_roundtrip() {
        for s in SisterId::all() {
            let json = serde_json::to_string(s).unwrap();
            let restored: SisterId = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, restored);
        }
    }

    #[test]
    fn test_sister_id_serde_snake_case() {
        let json = serde_json::to_string(&SisterId::Memory).unwrap();
        assert_eq!(json, "\"memory\"");
    }

    #[test]
    fn test_sister_id_hash_eq() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SisterId::Memory);
        set.insert(SisterId::Memory);
        assert_eq!(set.len(), 1);
    }
}
