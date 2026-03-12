#[cfg(test)]
mod tests {
    use crate::cognitive::self_repair::*;

    #[test]
    fn test_is_self_repair_intent() {
        assert!(is_self_repair_intent("fix yourself"));
        assert!(is_self_repair_intent("Hydra, repair your memory"));
        assert!(is_self_repair_intent("run self-repair"));
        assert!(is_self_repair_intent("your memory is broken"));
        assert!(is_self_repair_intent("run diagnostics on yourself"));
        assert!(!is_self_repair_intent("hello"));
        assert!(!is_self_repair_intent("what's the weather"));
        assert!(!is_self_repair_intent("fix this bug in my code"));
    }

    #[test]
    fn test_find_spec_for_complaint() {
        assert_eq!(find_spec_for_complaint("nothing is being saved to memory"), Some("001-wire-memory-learn.json"));
        assert_eq!(find_spec_for_complaint("you don't remember anything"), Some("001-wire-memory-learn.json"));
        assert_eq!(find_spec_for_complaint("the execution gate isn't working"), Some("003-wire-execution-gate.json"));
        assert_eq!(find_spec_for_complaint("you don't know my preferences"), Some("004-wire-beliefs.json"));
        assert_eq!(find_spec_for_complaint("fix your federation"), Some("025-system-mutation.json"));
        assert_eq!(find_spec_for_complaint("hello"), None);
    }

    #[test]
    fn test_evaluate_check_found() {
        let check = AcceptanceCheck {
            name: "test".into(),
            check: "echo hello".into(),
            expect: Some("found".into()),
            expect_minimum: None,
            expect_maximum: None,
        };
        assert!(evaluate_check(&check, "hello world"));
        assert!(!evaluate_check(&check, ""));
        assert!(!evaluate_check(&check, "ERROR: failed"));
    }

    #[test]
    fn test_evaluate_check_not_found() {
        let check = AcceptanceCheck {
            name: "test".into(),
            check: "echo".into(),
            expect: Some("not_found".into()),
            expect_minimum: None,
            expect_maximum: None,
        };
        assert!(evaluate_check(&check, ""));
        assert!(evaluate_check(&check, "0"));
        assert!(!evaluate_check(&check, "some output"));
    }

    #[test]
    fn test_evaluate_check_contains() {
        let check = AcceptanceCheck {
            name: "test".into(),
            check: "echo test".into(),
            expect: Some("Finished".into()),
            expect_minimum: None,
            expect_maximum: None,
        };
        assert!(evaluate_check(&check, "Finished `dev` profile"));
        assert!(!evaluate_check(&check, "error[E0599]"));
    }

    #[test]
    fn test_evaluate_check_minimum() {
        let check = AcceptanceCheck {
            name: "test".into(),
            check: "echo 5".into(),
            expect: None,
            expect_minimum: Some(3),
            expect_maximum: None,
        };
        assert!(evaluate_check(&check, "5"));
        assert!(evaluate_check(&check, "count: 10"));
        assert!(!evaluate_check(&check, "2"));
        assert!(!evaluate_check(&check, "no numbers"));
    }

    #[test]
    fn test_generate_spec() {
        let engine = SelfRepairEngine::new("/tmp/test");
        let spec = engine.generate_spec_from_analysis(
            "Test task",
            "Test description",
            &["file1.rs", "file2.rs"],
            vec![AcceptanceCheck {
                name: "test check".into(),
                check: "echo ok".into(),
                expect: Some("found".into()),
                expect_minimum: None,
                expect_maximum: None,
            }],
            "Fix the thing",
        );
        assert_eq!(spec.task, "Test task");
        assert_eq!(spec.files_to_modify.len(), 2);
        assert_eq!(spec.acceptance_checks.len(), 1);
    }

    #[test]
    fn test_list_specs_empty_dir() {
        let engine = SelfRepairEngine::new("/tmp/nonexistent-hydra-test");
        assert!(engine.list_specs().is_empty());
    }
}
