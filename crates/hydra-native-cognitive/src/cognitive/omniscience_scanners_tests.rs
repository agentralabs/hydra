//! Tests for omniscience scanner helpers.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cognitive::omniscience_scanners::*;
    use crate::cognitive::omniscience::*;

    #[test]
    fn test_is_omniscience_intent() {
        assert!(is_omniscience_intent("run omniscience loop"));
        assert!(is_omniscience_intent("read your own code"));
        assert!(is_omniscience_intent("do a full scan"));
        assert!(is_omniscience_intent("analyze your code for gaps"));
        assert!(is_omniscience_intent("scan all sisters"));
        assert!(is_omniscience_intent("repair sisters"));
        assert!(!is_omniscience_intent("hello"));
        assert!(!is_omniscience_intent("fix this bug"));
    }

    #[test]
    fn test_detect_repo_language() {
        // Non-existent paths should return "unknown"
        let p = PathBuf::from("/tmp/nonexistent-test-repo");
        assert_eq!(detect_repo_language(&p), "unknown");
    }

    #[test]
    fn test_identify_gaps_from_text() {
        let engine = OmniscienceEngine::new("/tmp/nonexistent-test");
        let target = RepoTarget {
            name: "test-repo".into(),
            path: "/tmp/nonexistent-test".into(),
            exists: false,
            language: "rust".into(),
        };
        let analysis = "Found a stub implementation in the federation module. \
                        There is also dead code in the old parser.";
        let gaps = engine.identify_gaps(&target, analysis);
        assert!(gaps.iter().any(|g| g.category == "missing_implementation"));
        assert!(gaps.iter().any(|g| g.category == "dead_code"));
        assert!(gaps.iter().all(|g| g.repo == "test-repo"));
    }

    #[test]
    fn test_health_score() {
        assert_eq!(calculate_health_score(&[], 100), 1.0);

        let gaps = vec![
            OmniscienceGap {
                repo: "test".into(),
                description: "test".into(),
                files: vec![],
                severity: "critical".into(),
                category: "missing_implementation".into(),
                suggested_fix: "fix it".into(),
            },
        ];
        let score = calculate_health_score(&gaps, 100);
        assert!(score < 1.0);
        assert!(score > 0.0);
    }

    #[test]
    fn test_multi_repo_targets() {
        let engine = OmniscienceEngine::new("/tmp/nonexistent-hydra");
        // Should have hydra + 14 sisters = 15 targets
        assert_eq!(engine.targets.len(), 15);
        assert_eq!(engine.targets[0].name, "agentic-hydra");
        assert!(engine.targets.iter().any(|t| t.name == "agentic-memory"));
        assert!(engine.targets.iter().any(|t| t.name == "agentic-aegis"));
        assert!(engine.targets.iter().any(|t| t.name == "agentic-forge"));
    }

    #[test]
    fn test_with_explicit_targets() {
        let engine = OmniscienceEngine::with_targets("/tmp/hydra", &[
            ("custom-sister", "/tmp/custom"),
        ]);
        assert_eq!(engine.targets.len(), 2);
        assert_eq!(engine.targets[0].name, "agentic-hydra");
        assert_eq!(engine.targets[1].name, "custom-sister");
    }

    #[test]
    fn test_generate_checks_rust() {
        let engine = OmniscienceEngine::new("/tmp/test");
        let target = RepoTarget {
            name: "agentic-memory".into(),
            path: "/tmp/agentic-memory".into(),
            exists: true,
            language: "rust".into(),
        };
        let gap = OmniscienceGap {
            repo: "agentic-memory".into(),
            description: "stub in lib.rs".into(),
            files: vec!["src/lib.rs".into()],
            severity: "critical".into(),
            category: "missing_implementation".into(),
            suggested_fix: "implement it".into(),
        };
        let checks = engine.generate_checks_for_gap(&target, &gap);
        assert!(checks.len() >= 2);
        assert!(checks.iter().any(|c| c.name.contains("no-stubs")));
        assert!(checks.iter().any(|c| c.name.contains("agentic-memory-compiles")));
    }

    #[test]
    fn test_generate_checks_typescript() {
        let engine = OmniscienceEngine::new("/tmp/test");
        let target = RepoTarget {
            name: "agentic-vision".into(),
            path: "/tmp/agentic-vision".into(),
            exists: true,
            language: "typescript".into(),
        };
        let gap = OmniscienceGap {
            repo: "agentic-vision".into(),
            description: "missing test".into(),
            files: vec!["src/capture.ts".into()],
            severity: "medium".into(),
            category: "missing_test".into(),
            suggested_fix: "add tests".into(),
        };
        let checks = engine.generate_checks_for_gap(&target, &gap);
        assert!(checks.iter().any(|c| c.check.contains("describe")));
    }

    #[test]
    fn test_count_source_files() {
        // Non-existent dir should return 0
        assert_eq!(count_source_files_in(&PathBuf::from("/tmp/nonexistent"), "rust"), 0);
    }

    #[test]
    fn test_false_positive_string_literal() {
        // Template generators that OUTPUT todo!() should not be flagged
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // This is what Forge's blueprint generator does — it creates skeleton code WITH todo!()
        std::fs::write(src.join("generator.rs"), r#"
fn generate_skeleton() -> String {
    let mut s = String::new();
    s.push_str("    todo!()\n");
    s
}
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert!(gaps.is_empty(), "String literal todo!() should not be flagged, got: {:?}", gaps);
    }

    #[test]
    fn test_false_positive_assert() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // Test assertions checking for todo!() presence
        std::fs::write(src.join("test_gen.rs"), r#"
fn test_skeleton_has_stubs() {
    assert!(skeleton.contains("todo!()"));
}
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert!(gaps.is_empty(), "Assert containing todo!() should not be flagged, got: {:?}", gaps);
    }

    #[test]
    fn test_false_positive_test_fixtures() {
        let dir = tempfile::tempdir().unwrap();
        let tests_dir = dir.path().join("tests");
        std::fs::create_dir_all(&tests_dir).unwrap();

        // Test fixture with template string
        std::fs::write(tests_dir.join("edge_stress.rs"), r#"
const TEMPLATE: &str = "fn {{name}}() { todo!() }";
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert!(gaps.is_empty(), "Test fixture todo!() should not be flagged, got: {:?}", gaps);
    }

    #[test]
    fn test_real_stub_still_flagged() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // This IS a real unimplemented stub
        std::fs::write(src.join("lib.rs"), r#"
fn actual_function() {
    todo!()
}

fn another_stub() -> Result<(), Error> {
    unimplemented!()
}
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert_eq!(gaps.len(), 2, "Real stubs must still be flagged, got: {:?}", gaps);
    }

    #[test]
    fn test_comment_not_flagged() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // Comments mentioning todo!() are not gaps
        std::fs::write(src.join("scanner.rs"), r#"
// Scans for todo!() and unimplemented!() patterns in source code
/// Detects todo!() stubs that need implementation
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert!(gaps.is_empty(), "Comments mentioning todo!() should not be flagged, got: {:?}", gaps);
    }
}
