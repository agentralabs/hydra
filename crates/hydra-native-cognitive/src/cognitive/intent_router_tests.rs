//! Tests for the intent router — split from intent_router.rs for file-size hygiene.

#[cfg(test)]
mod tests {
    use crate::cognitive::intent_router::{IntentCategory, ClassifiedIntent};
    use crate::cognitive::intent_router_classify::{
        parse_classification, emergency_classify,
    };

    /// Test parse_classification with simulated LLM responses.
    /// This is the core of the micro-LLM classifier — if parsing works,
    /// the classifier works (because the LLM understands meaning).

    fn parse(json: &str, input: &str) -> ClassifiedIntent {
        parse_classification(json, input)
    }

    // ── Sister repair ──

    #[test]
    fn test_fix_contract_sister() {
        let c = parse(r#"{"category": "sister_repair", "target": "contract", "confidence": 0.95}"#, "fix contract sister");
        assert_eq!(c.category, IntentCategory::SisterRepair);
        assert_eq!(c.target.as_deref(), Some("contract"));
    }

    #[test]
    fn test_can_you_fix_her() {
        // LLM resolves "her" from context → sister target
        let c = parse(r#"{"category": "sister_repair", "target": "contract", "confidence": 0.92}"#, "can you fix her?");
        assert_eq!(c.category, IntentCategory::SisterRepair);
        assert_eq!(c.target.as_deref(), Some("contract"));
    }

    #[test]
    fn test_repair_broken_sisters() {
        let c = parse(r#"{"category": "sister_repair", "target": "all", "confidence": 0.9}"#, "repair broken sisters");
        assert_eq!(c.category, IntentCategory::SisterRepair);
    }

    #[test]
    fn test_bring_memory_online() {
        let c = parse(r#"{"category": "sister_repair", "target": "memory", "confidence": 0.93}"#, "bring memory back online");
        assert_eq!(c.category, IntentCategory::SisterRepair);
        assert_eq!(c.target.as_deref(), Some("memory"));
    }

    #[test]
    fn test_yo_hydra_make_it_work() {
        let c = parse(r#"{"category": "sister_repair", "target": null, "confidence": 0.85}"#, "yo hydra make that thing work again");
        assert_eq!(c.category, IntentCategory::SisterRepair);
    }

    // ── Sister diagnostics ──

    #[test]
    fn test_check_sisters() {
        let c = parse(r#"{"category": "sister_diagnose", "target": "all", "confidence": 0.95}"#, "check sisters");
        assert_eq!(c.category, IntentCategory::SisterDiagnose);
    }

    #[test]
    fn test_what_is_problem_with_contract() {
        let c = parse(r#"{"category": "sister_diagnose", "target": "contract", "confidence": 0.9}"#, "what is the problem with contract?");
        assert_eq!(c.category, IntentCategory::SisterDiagnose);
        assert_eq!(c.target.as_deref(), Some("contract"));
    }

    #[test]
    fn test_is_memory_online() {
        let c = parse(r#"{"category": "sister_diagnose", "target": "memory", "confidence": 0.92}"#, "is memory online?");
        assert_eq!(c.category, IntentCategory::SisterDiagnose);
        assert_eq!(c.target.as_deref(), Some("memory"));
    }

    // ── Self repair / scan ──

    #[test]
    fn test_fix_yourself() {
        let c = parse(r#"{"category": "self_repair", "target": "self", "confidence": 0.95}"#, "fix yourself");
        assert_eq!(c.category, IntentCategory::SelfRepair);
    }

    #[test]
    fn test_scan_yourself() {
        let c = parse(r#"{"category": "self_scan", "target": "self", "confidence": 0.95}"#, "scan yourself");
        assert_eq!(c.category, IntentCategory::SelfScan);
    }

    // ── Memory ──

    #[test]
    fn test_remember_favorite_color() {
        let c = parse(r#"{"category": "memory_store", "target": null, "confidence": 0.95}"#, "remember my favorite color is blue");
        assert_eq!(c.category, IntentCategory::MemoryStore);
        assert!(c.payload.as_deref().unwrap().contains("favorite color"));
    }

    #[test]
    fn test_whats_my_favorite_color() {
        let c = parse(r#"{"category": "memory_recall", "target": null, "confidence": 0.92}"#, "what's my favorite color?");
        assert_eq!(c.category, IntentCategory::MemoryRecall);
    }

    // ── Code ──

    #[test]
    fn test_build_project() {
        let c = parse(r#"{"category": "code_build", "target": "project", "confidence": 0.9}"#, "build the project");
        assert_eq!(c.category, IntentCategory::CodeBuild);
    }

    #[test]
    fn test_fix_bug_is_code_not_sister() {
        let c = parse(r#"{"category": "code_fix", "target": "main.rs", "confidence": 0.9}"#, "fix the bug in main.rs");
        assert_eq!(c.category, IntentCategory::CodeFix);
    }

    // ── Greetings ──

    #[test]
    fn test_greeting() {
        let c = parse(r#"{"category": "greeting", "target": null, "confidence": 0.99}"#, "hello");
        assert_eq!(c.category, IntentCategory::Greeting);
    }

    // ── Conversation (goes to LLM) ──

    #[test]
    fn test_question_goes_to_llm() {
        let c = parse(r#"{"category": "conversation", "target": null, "confidence": 0.85}"#, "how do neural networks work?");
        assert_eq!(c.category, IntentCategory::Question);
    }

    // ── Emergency fallback ──

    #[test]
    fn test_emergency_remember() {
        let c = emergency_classify("remember my favorite color is blue");
        assert_eq!(c.category, IntentCategory::MemoryStore);
        assert!(c.payload.as_deref().unwrap().contains("favorite color"));
    }

    #[test]
    fn test_emergency_greeting() {
        let c = emergency_classify("hello");
        assert_eq!(c.category, IntentCategory::Greeting);
    }

    #[test]
    fn test_emergency_sister_repair() {
        // "fix contract sister" should now correctly classify as SisterRepair
        let c = emergency_classify("fix contract sister");
        assert_eq!(c.category, IntentCategory::SisterRepair);
    }

    #[test]
    fn test_emergency_truly_unknown() {
        let c = emergency_classify("what is the meaning of life?");
        assert_eq!(c.category, IntentCategory::Unknown);
    }

    // ── Category parsing ──

    #[test]
    fn test_category_from_str() {
        assert_eq!(IntentCategory::from_str("sister_repair"), IntentCategory::SisterRepair);
        assert_eq!(IntentCategory::from_str("sister-repair"), IntentCategory::SisterRepair);
        assert_eq!(IntentCategory::from_str("SISTER_REPAIR"), IntentCategory::SisterRepair);
        assert_eq!(IntentCategory::from_str("conversation"), IntentCategory::Question);
        assert_eq!(IntentCategory::from_str("gibberish"), IntentCategory::Unknown);
    }

    // ── JSON parsing edge cases ──

    #[test]
    fn test_parse_markdown_wrapped_json() {
        let c = parse("```json\n{\"category\": \"sister_repair\", \"target\": \"contract\", \"confidence\": 0.9}\n```", "fix contract");
        assert_eq!(c.category, IntentCategory::SisterRepair);
    }

    #[test]
    fn test_parse_bad_json_falls_to_emergency() {
        let c = parse("I think this is sister_repair", "fix contract");
        // Bad JSON → emergency_classify → SisterRepair (contains "fix" + sister name "contract")
        assert_eq!(c.category, IntentCategory::SisterRepair);
    }

    // ── Target resolution ──

    #[test]
    fn test_resolve_null_target_finds_sister_in_input() {
        let c = parse(r#"{"category": "sister_repair", "target": null, "confidence": 0.9}"#, "fix the contract sister");
        assert_eq!(c.target.as_deref(), Some("contract"));
    }

    #[test]
    fn test_resolve_pronoun_target() {
        // LLM said target is "her" but we can find "contract" in input (won't match sister names)
        // In real usage, the LLM would resolve "her" to "contract" from context
        let c = parse(r#"{"category": "sister_repair", "target": "her", "confidence": 0.9}"#, "can you fix her?");
        // "her" doesn't match any sister name, returns as-is
        assert_eq!(c.target.as_deref(), Some("her"));
    }

    // ── Identity & Receipt queries (Block 5) ──

    #[test]
    fn test_emergency_what_did_you_do() {
        let c = emergency_classify("what did you just do?");
        assert_eq!(c.category, IntentCategory::Question);
        assert_eq!(c.target.as_deref(), Some("identity"));
    }

    #[test]
    fn test_emergency_prove_what_you_did() {
        let c = emergency_classify("prove what you did in the last hour");
        assert_eq!(c.category, IntentCategory::Question);
        assert_eq!(c.target.as_deref(), Some("identity"));
    }

    #[test]
    fn test_emergency_trust_level() {
        let c = emergency_classify("what's my trust level?");
        assert_eq!(c.category, IntentCategory::Question);
        assert_eq!(c.target.as_deref(), Some("identity"));
    }

    #[test]
    fn test_emergency_show_receipts() {
        let c = emergency_classify("show my receipts");
        assert_eq!(c.category, IntentCategory::Question);
        assert_eq!(c.target.as_deref(), Some("identity"));
    }

    // ── Planning & Time queries (Block 6) ──

    #[test]
    fn test_emergency_create_goal() {
        let c = emergency_classify("create a goal: deploy v2.0 by Friday");
        assert_eq!(c.category, IntentCategory::PlanningQuery);
    }

    #[test]
    fn test_emergency_what_are_my_goals() {
        let c = emergency_classify("what are my goals?");
        assert_eq!(c.category, IntentCategory::PlanningQuery);
    }

    #[test]
    fn test_emergency_any_deadlines() {
        let c = emergency_classify("any deadlines?");
        assert_eq!(c.category, IntentCategory::PlanningQuery);
    }

    // ── Belief queries (Block 7) ──

    #[test]
    fn test_emergency_belief_statement() {
        let c = emergency_classify("we're using PostgreSQL and Express for this project");
        assert_eq!(c.category, IntentCategory::Question);
        assert_eq!(c.target.as_deref(), Some("belief"));
    }

    #[test]
    fn test_emergency_belief_correction() {
        let c = emergency_classify("actually, we switched to FastAPI instead of Express");
        assert_eq!(c.category, IntentCategory::Question);
        assert_eq!(c.target.as_deref(), Some("correction"));
    }

    // ── Spanish query (Block 9, T34) — emergency fallback goes to Unknown,
    //    which is correct because the main LLM handles it with beliefs in context ──

    #[test]
    fn test_emergency_spanish_query_falls_to_unknown() {
        let c = emergency_classify("cuál es mi base de datos favorita?");
        // Spanish query → Unknown in emergency mode (no keyword match)
        // This is OK: main LLM handles it with beliefs injected into system prompt
        assert_eq!(c.category, IntentCategory::Unknown);
    }

    // ── T16-T20: Intent classification for any-phrasing tests ──

    #[test]
    fn test_emergency_yo_check_on_sisters() {
        let c = emergency_classify("yo check on the sisters");
        assert_eq!(c.category, IntentCategory::SisterDiagnose);
    }

    #[test]
    fn test_emergency_make_contract_work_again() {
        let c = emergency_classify("make contract work again");
        assert_eq!(c.category, IntentCategory::SisterRepair);
    }

    #[test]
    fn test_emergency_grab_me_latest_news() {
        let c = emergency_classify("grab me the latest news");
        assert_eq!(c.category, IntentCategory::WebBrowse);
    }

    #[test]
    fn test_emergency_whats_in_here() {
        let c = emergency_classify("whats in here");
        assert_eq!(c.category, IntentCategory::FileOperation);
    }

    #[test]
    fn test_emergency_remind_me_database_choice() {
        let c = emergency_classify("remind me about my database choice");
        assert_eq!(c.category, IntentCategory::MemoryRecall);
    }

    // ── Self-implement (Phase 4, Part E) ──

    #[test]
    fn test_self_implement_spec() {
        let c = parse(r#"{"category": "self_implement", "target": null, "confidence": 0.92}"#, "implement this spec");
        assert_eq!(c.category, IntentCategory::SelfImplement);
    }

    #[test]
    fn test_emergency_implement_this_spec() {
        let c = emergency_classify("implement this spec");
        assert_eq!(c.category, IntentCategory::SelfImplement);
    }

    #[test]
    fn test_emergency_build_this_yourself() {
        let c = emergency_classify("build this yourself");
        assert_eq!(c.category, IntentCategory::SelfImplement);
    }

    #[test]
    fn test_emergency_add_capability() {
        let c = emergency_classify("add this capability to yourself");
        assert_eq!(c.category, IntentCategory::SelfImplement);
    }
}
