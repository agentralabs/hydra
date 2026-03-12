#[cfg(test)]
mod tests {
    use crate::self_modify::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_fn_name() {
        assert_eq!(extract_fn_name("pub fn hello()"), "hello");
        assert_eq!(extract_fn_name("fn world(x: i32)"), "world");
        assert_eq!(extract_fn_name("pub async fn run()"), "run");
        assert_eq!(extract_fn_name("async fn tick<T>()"), "tick");
        assert_eq!(extract_fn_name("not a function"), "");
    }

    #[test]
    fn test_mod_result_summary() {
        let success = ModResult::Success {
            gaps_filled: 3,
            patches_applied: 2,
            tests_passing: 10,
        };
        assert!(success.summary().contains("3 gaps"));
        assert!(success.summary().contains("2 patches"));

        let already = ModResult::AlreadyImplemented;
        assert!(already.summary().contains("already exists"));
    }

    #[test]
    fn test_patch_requires_extra_approval() {
        let safe_patch = Patch {
            target_file: "src/helpers.rs".into(),
            gap: SpecGap {
                description: "test".into(),
                target_file: "src/helpers.rs".into(),
                gap_type: GapType::MissingFunction,
                priority: 1,
            },
            diff_content: "fn helper() {}".into(),
            description: "Add helper".into(),
            touches_critical: false,
        };
        assert!(!safe_patch.requires_extra_approval());

        let critical_patch = Patch {
            target_file: "src/execution_gate.rs".into(),
            gap: SpecGap {
                description: "test".into(),
                target_file: "src/execution_gate.rs".into(),
                gap_type: GapType::MissingFunction,
                priority: 1,
            },
            diff_content: "fn bypass() {}".into(),
            description: "Modify gate".into(),
            touches_critical: true,
        };
        assert!(critical_patch.requires_extra_approval());
    }

    #[test]
    fn test_file_checkpoint_capture_and_revert() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, "original content").unwrap();

        let checkpoint = FileCheckpoint::capture(&[file_path.clone()]);
        assert_eq!(checkpoint.files.len(), 1);
        assert!(checkpoint.files[0].existed);

        // Modify the file
        std::fs::write(&file_path, "modified content").unwrap();
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "modified content");

        // Revert
        let reverted = checkpoint.revert().unwrap();
        assert_eq!(reverted, 1);
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "original content");
    }

    #[test]
    fn test_file_checkpoint_new_file_revert() {
        let dir = TempDir::new().unwrap();
        let new_file = dir.path().join("new.rs");

        // Checkpoint when file doesn't exist
        let checkpoint = FileCheckpoint::capture(&[new_file.clone()]);
        assert!(!checkpoint.files[0].existed);

        // Create the file
        std::fs::write(&new_file, "new content").unwrap();
        assert!(new_file.exists());

        // Revert — should remove the file
        checkpoint.revert().unwrap();
        assert!(!new_file.exists());
    }

    #[test]
    fn test_pipeline_too_many_patches() {
        let dir = TempDir::new().unwrap();
        let pipeline = SelfModificationPipeline::new(dir.path());

        let gaps = vec![SpecGap {
            description: "test".into(),
            target_file: String::new(),
            gap_type: GapType::MissingFunction,
            priority: 1,
        }];

        // Create 6 patches (over limit of 5)
        let patches: Vec<Patch> = (0..6)
            .map(|i| Patch {
                target_file: format!("src/file{}.rs", i),
                gap: gaps[0].clone(),
                diff_content: format!("fn f{}() {{}}", i),
                description: format!("Patch {}", i),
                touches_critical: false,
            })
            .collect();

        let result = pipeline.run_from_gaps(gaps, patches);
        assert!(matches!(result, ModResult::PipelineError { .. }));
    }

    #[test]
    fn test_pipeline_blocks_critical_patches() {
        let dir = TempDir::new().unwrap();
        let pipeline = SelfModificationPipeline::new(dir.path());

        let gaps = vec![SpecGap {
            description: "test".into(),
            target_file: String::new(),
            gap_type: GapType::MissingFunction,
            priority: 1,
        }];

        let patches = vec![Patch {
            target_file: "src/execution_gate.rs".into(),
            gap: gaps[0].clone(),
            diff_content: "fn bypass() {}".into(),
            description: "Bypass gate".into(),
            touches_critical: true,
        }];

        let result = pipeline.run_from_gaps(gaps, patches);
        assert!(matches!(result, ModResult::ShadowFailed { .. }));
    }

    #[test]
    fn test_pipeline_already_implemented() {
        let dir = TempDir::new().unwrap();
        let pipeline = SelfModificationPipeline::new(dir.path());
        let result = pipeline.run_from_gaps(vec![], vec![]);
        assert!(matches!(result, ModResult::AlreadyImplemented));
    }

    #[test]
    fn test_pipeline_rejects_over_400_lines() {
        let dir = TempDir::new().unwrap();
        let pipeline = SelfModificationPipeline::new(dir.path());

        // Create a file already at 390 lines
        let file_path = dir.path().join("src");
        std::fs::create_dir_all(&file_path).unwrap();
        let target = file_path.join("big.rs");
        let existing = (0..390).map(|i| format!("// line {}", i)).collect::<Vec<_>>().join("\n");
        std::fs::write(&target, &existing).unwrap();

        // Patch that would push it over 400
        let patch = Patch {
            target_file: "src/big.rs".into(),
            gap: SpecGap {
                description: "test".into(),
                target_file: "src/big.rs".into(),
                gap_type: GapType::MissingFunction,
                priority: 1,
            },
            diff_content: (0..20).map(|i| format!("fn f{}() {{}}", i)).collect::<Vec<_>>().join("\n"),
            description: "Big patch".into(),
            touches_critical: false,
        };

        let result = pipeline.apply_patch(&patch);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("400"));
    }
}
