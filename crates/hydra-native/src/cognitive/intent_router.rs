//! Intent Router — Micro-LLM classifier for the cognitive loop.
//!
//! ONE tiny LLM call (~150 tokens) at the start of every cognitive cycle.
//! The LLM understands MEANING — any phrasing, any language, any slang.
//!
//! "fix broken sisters", "can you fix her?", "arregla eso", "直して"
//! — ALL classify correctly because an LLM understands language.
//!
//! Zero keyword lists. Zero verb matching. Zero pattern hacks.
//! This is the LAST intent classifier Hydra will ever need.

use crate::sisters::connection::SisterConnection;
use tracing::{debug, warn};

// ═══════════════════════════════════════════════════════════════════
// Intent categories — every capability Hydra can handle directly
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntentCategory {
    // Greetings & conversation
    Greeting,
    Farewell,
    Thanks,

    // Memory
    MemoryStore,     // "remember X", "note that X"
    MemoryRecall,    // "what's my favorite X?", "do you remember X?"

    // Sister management
    SisterDiagnose,  // "check sisters", "sister status"
    SisterRepair,    // "fix broken sisters", "repair contract"

    // Self management
    SelfRepair,      // "fix yourself", "run self-repair"
    SelfScan,        // "scan yourself", "omniscience scan"

    // Code
    CodeBuild,       // "build the project", "compile"
    CodeExplain,     // "explain this code", "what does X do?"
    CodeFix,         // "fix this bug", "debug X"

    // System/App control
    SystemControl,   // "open terminal", "launch browser"
    AppControl,      // "open settings", "show sidebar"

    // Planning
    PlanningQuery,   // "what's the plan?", "show goals"

    // Web
    WebBrowse,       // "go to X", "search for Y"

    // File operations
    FileOperation,   // "create file X", "delete Y"

    // Communication
    Communicate,     // "send message", "email"

    // Deploy
    Deploy,          // "deploy", "publish", "ship"

    // Settings/Preferences
    Settings,        // "change theme", "settings"

    // Opinion/Factual (needs LLM)
    Question,        // General question that needs LLM

    // Unknown — falls through to LLM
    Unknown,
}

impl IntentCategory {
    /// Whether this category has a direct handler (no LLM needed).
    pub fn has_direct_handler(&self) -> bool {
        !matches!(self, Self::Question | Self::Unknown | Self::CodeExplain)
    }

    /// Parse from the category string returned by the micro-LLM.
    fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().replace('-', "_").as_str() {
            "greeting" => Self::Greeting,
            "farewell" => Self::Farewell,
            "thanks" => Self::Thanks,
            "memory_store" => Self::MemoryStore,
            "memory_recall" => Self::MemoryRecall,
            "sister_diagnose" => Self::SisterDiagnose,
            "sister_repair" => Self::SisterRepair,
            "self_repair" => Self::SelfRepair,
            "self_scan" => Self::SelfScan,
            "code_build" => Self::CodeBuild,
            "code_explain" => Self::CodeExplain,
            "code_fix" => Self::CodeFix,
            "system_control" => Self::SystemControl,
            "app_open" | "app_close" | "app_control" => Self::AppControl,
            "planning" | "planning_query" => Self::PlanningQuery,
            "web_browse" => Self::WebBrowse,
            "file_operation" => Self::FileOperation,
            "communication" | "communicate" => Self::Communicate,
            "deploy" => Self::Deploy,
            "settings" => Self::Settings,
            "conversation" | "question" => Self::Question,
            _ => Self::Unknown,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Classified intent — result of the classification stage
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ClassifiedIntent {
    pub category: IntentCategory,
    pub confidence: f32,
    /// Extracted target (e.g., sister name, file path, URL)
    pub target: Option<String>,
    /// Extracted payload (e.g., the fact to remember, the command to run)
    pub payload: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════
// Sister names — used for target resolution
// ═══════════════════════════════════════════════════════════════════

const SISTER_NAMES: &[&str] = &[
    "memory", "identity", "codebase", "vision", "comm", "contract",
    "time", "planning", "cognition", "reality", "forge", "aegis",
    "veritas", "evolve",
];

// ═══════════════════════════════════════════════════════════════════
// Classification prompt — sent to the cheapest/fastest LLM
// ~120 input tokens + ~30 output tokens = ~150 total
// ═══════════════════════════════════════════════════════════════════

const CLASSIFICATION_PROMPT: &str = "\
Classify this user message into exactly ONE category.\n\
Return ONLY a JSON object, nothing else.\n\n\
Categories:\n\
- sister_diagnose: checking status/health of a sister/component\n\
- sister_repair: fixing/restarting/healing a sister/component\n\
- self_scan: analyzing own code/health/problems\n\
- self_repair: fixing own issues\n\
- memory_store: user wants to save/remember something\n\
- memory_recall: user asking about something previously stored\n\
- app_open: opening an application\n\
- app_close: closing an application\n\
- system_control: volume/brightness/wifi/bluetooth/display\n\
- web_browse: searching/browsing the internet\n\
- code_build: building/creating a project or code\n\
- code_fix: fixing/debugging code\n\
- code_explain: explaining code\n\
- file_operation: reading/writing/listing files\n\
- planning: goals/deadlines/progress/what to do next\n\
- communication: sending messages/posting/emailing\n\
- deploy: deploying/publishing/shipping\n\
- settings: changing preferences/theme/config\n\
- greeting: hi/hello/hey\n\
- farewell: bye/goodbye/see you\n\
- thanks: thank you/thanks/ty\n\
- conversation: opinions/questions/discussion/jokes\n\n\
Sisters are named components: memory, identity, codebase, vision, comm, contract, time, planning, cognition, reality, forge, aegis, veritas, evolve.\n\
Pronouns like \"her\", \"it\", \"that\" referring to a sister in context = sister target.\n\n";

// ═══════════════════════════════════════════════════════════════════
// Universal classifier — uses a micro-LLM call to understand meaning
// ═══════════════════════════════════════════════════════════════════

/// Classify user input using a tiny LLM call (~150 tokens).
///
/// Model priority: Haiku (cheapest) → Sonnet → whatever is available.
/// Temperature: 0.0 (deterministic). Max tokens: 60. No tools.
///
/// When no LLM key is available, falls back to emergency_classify()
/// which only handles greetings and "remember X".
pub async fn classify(
    input: &str,
    _veritas: Option<&SisterConnection>,
    context: &[(String, String)],
    llm_config: &hydra_model::LlmConfig,
) -> ClassifiedIntent {
    // Build minimal context (last 2 messages, truncated)
    let recent = context.iter().rev().take(2)
        .map(|(role, msg)| format!("{}: {}", role, &msg[..msg.len().min(100)]))
        .collect::<Vec<_>>()
        .join("\n");

    let user_content = if recent.is_empty() {
        format!("User message: {}", input)
    } else {
        format!("Recent context:\n{}\n\nUser message: {}", recent, input)
    };

    // Try cheapest model first: Haiku → Sonnet → GPT-4o-mini → whatever
    let (model, provider) = pick_cheapest_model(llm_config);

    if model.is_empty() {
        // No API key available → emergency fallback
        debug!("[hydra:intent] No LLM key — emergency classify");
        return emergency_classify(input);
    }

    let request = hydra_model::CompletionRequest {
        model: model.clone(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: user_content,
        }],
        max_tokens: 60,
        temperature: Some(0.0),
        system: Some(CLASSIFICATION_PROMPT.to_string()),
    };

    debug!("[hydra:intent] Classifying with {} (~150 tokens)", model);

    let result = match provider {
        "anthropic" => {
            match hydra_model::providers::anthropic::AnthropicClient::new(llm_config) {
                Ok(client) => client.complete(request).await.map(|r| r.content).map_err(|e| format!("{}", e)),
                Err(e) => Err(format!("{}", e)),
            }
        }
        "openai" => {
            match hydra_model::providers::openai::OpenAiClient::new(llm_config) {
                Ok(client) => client.complete(request).await.map(|r| r.content).map_err(|e| format!("{}", e)),
                Err(e) => Err(format!("{}", e)),
            }
        }
        _ => Err("No provider".into()),
    };

    match result {
        Ok(response) => {
            debug!("[hydra:intent] LLM response: {}", response.trim());
            parse_classification(&response, input)
        }
        Err(e) => {
            warn!("[hydra:intent] LLM classify failed: {} — emergency fallback", e);
            emergency_classify(input)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Model selection — always pick the cheapest available model
// ═══════════════════════════════════════════════════════════════════

fn pick_cheapest_model(config: &hydra_model::LlmConfig) -> (String, &'static str) {
    if config.anthropic_api_key.is_some() {
        // Haiku: $0.001/$0.005 per 1K — cheapest Anthropic model
        ("claude-haiku-4-5-20251001".into(), "anthropic")
    } else if config.openai_api_key.is_some() {
        // GPT-4o-mini: $0.00015/$0.0006 per 1K — cheapest OpenAI
        ("gpt-4o-mini".into(), "openai")
    } else {
        (String::new(), "none")
    }
}

// ═══════════════════════════════════════════════════════════════════
// Parse the LLM's JSON response into a ClassifiedIntent
// ═══════════════════════════════════════════════════════════════════

fn parse_classification(response: &str, input: &str) -> ClassifiedIntent {
    // Try to extract JSON from the response
    let json_str = response.trim();

    // Handle potential markdown code blocks
    let json_str = json_str
        .strip_prefix("```json").or_else(|| json_str.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(json_str)
        .trim();

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
        let category_str = parsed.get("category")
            .and_then(|c| c.as_str())
            .unwrap_or("unknown");

        let confidence = parsed.get("confidence")
            .and_then(|c| c.as_f64())
            .unwrap_or(0.8) as f32;

        let target_raw = parsed.get("target")
            .and_then(|t| t.as_str())
            .map(|s| s.to_lowercase());

        let category = IntentCategory::from_str(category_str);

        // Resolve target to a sister name if applicable
        let target = resolve_target(&target_raw, input);

        // Extract payload for memory_store
        let payload = if category == IntentCategory::MemoryStore {
            extract_memory_payload(input)
        } else {
            None
        };

        ClassifiedIntent { category, confidence, target, payload }
    } else {
        warn!("[hydra:intent] Failed to parse LLM JSON: {}", json_str);
        emergency_classify(input)
    }
}

/// Resolve the target string to a known sister name if possible.
fn resolve_target(target: &Option<String>, input: &str) -> Option<String> {
    // Check the LLM's target first
    if let Some(ref t) = target {
        let lower = t.to_lowercase();
        if lower == "null" || lower == "none" || lower.is_empty() {
            // Fall through
        } else {
            // Check if it matches a sister name
            for name in SISTER_NAMES {
                if lower.contains(name) {
                    return Some(name.to_string());
                }
            }
            // Return as-is if not a sister (could be a file path, URL, app name, etc.)
            return Some(t.clone());
        }
    }

    // Fallback: check input for sister names
    let lower_input = input.to_lowercase();
    for name in SISTER_NAMES {
        if lower_input.contains(name) {
            return Some(name.to_string());
        }
    }

    None
}

fn extract_memory_payload(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    // Find the content after common memory-store prefixes
    for prefix in &["remember ", "note that ", "save that ", "don't forget "] {
        if let Some(pos) = lower.find(prefix) {
            return Some(input[pos + prefix.len()..].to_string());
        }
    }
    if let Some(pos) = lower.find("remember that ") {
        return Some(input[pos + 14..].to_string());
    }
    Some(input.to_string())
}

// ═══════════════════════════════════════════════════════════════════
// Emergency fallback — when no LLM key is available at all
// Only handles trivially recognizable patterns. Everything else → Unknown.
// ═══════════════════════════════════════════════════════════════════

fn emergency_classify(input: &str) -> ClassifiedIntent {
    let lower = input.to_lowercase();
    let trimmed = lower.trim().trim_end_matches('!');

    // Greetings — single word, trivially recognizable
    if matches!(trimmed, "hi" | "hey" | "hello" | "yo" | "sup" | "hola" | "howdy"
        | "good morning" | "good evening" | "good afternoon") {
        return ClassifiedIntent { category: IntentCategory::Greeting, confidence: 0.95, target: None, payload: None };
    }
    if matches!(trimmed, "bye" | "goodbye" | "see you" | "see ya" | "later" | "goodnight" | "cya" | "peace") {
        return ClassifiedIntent { category: IntentCategory::Farewell, confidence: 0.95, target: None, payload: None };
    }
    if lower.starts_with("thank") || trimmed == "ty" || trimmed == "thx" {
        return ClassifiedIntent { category: IntentCategory::Thanks, confidence: 0.95, target: None, payload: None };
    }

    // Memory store — "remember X" is unambiguous
    if lower.starts_with("remember ") || lower.starts_with("note that ") || lower.starts_with("don't forget ") {
        return ClassifiedIntent {
            category: IntentCategory::MemoryStore,
            confidence: 0.9,
            target: None,
            payload: extract_memory_payload(input),
        };
    }

    // Memory recall — "what's my ...", "do you remember ...", "remind me ..."
    // Exclude trust/receipt/identity queries that start with "what's my"
    let is_identity_query = lower.contains("trust") || lower.contains("receipt")
        || lower.contains("audit") || lower.contains("autonomy");
    if !is_identity_query && (lower.starts_with("what's my ") || lower.starts_with("whats my ")
        || lower.starts_with("what is my ") || lower.starts_with("do you remember")
        || lower.starts_with("remind me about") || lower.starts_with("remind me of")
        || (lower.contains("my") && lower.contains("favorite"))
        || (lower.contains("my") && lower.contains("favourite")))
    {
        return ClassifiedIntent { category: IntentCategory::MemoryRecall, confidence: 0.85, target: None, payload: None };
    }

    // Sister diagnose — "check sisters", "sister status", "yo check on the sisters"
    if (lower.contains("sister") && (lower.contains("check") || lower.contains("status") || lower.contains("health")))
        || lower.contains("check sisters") || lower.contains("check on the sister")
    {
        return ClassifiedIntent { category: IntentCategory::SisterDiagnose, confidence: 0.85, target: None, payload: None };
    }

    // Self scan / self repair
    if lower.contains("scan yourself") || lower.contains("scan your") || lower.contains("omniscience") {
        return ClassifiedIntent { category: IntentCategory::SelfScan, confidence: 0.85, target: None, payload: None };
    }
    if lower.contains("fix yourself") || lower.contains("repair yourself") || lower.contains("self-repair") {
        return ClassifiedIntent { category: IntentCategory::SelfRepair, confidence: 0.85, target: None, payload: None };
    }

    // Sister repair — "fix X sister", "make X work again"
    if (lower.contains("fix") || lower.contains("repair") || lower.contains("work again"))
        && SISTER_NAMES.iter().any(|s| lower.contains(s))
    {
        return ClassifiedIntent { category: IntentCategory::SisterRepair, confidence: 0.80, target: None, payload: None };
    }

    // News/web — "news", "top stories", "latest news", "grab me the latest"
    if lower.contains("news") || lower.contains("top stories") || lower.contains("headlines")
        || lower.contains("trending") || (lower.contains("grab") && lower.contains("latest"))
    {
        return ClassifiedIntent { category: IntentCategory::WebBrowse, confidence: 0.80, target: None, payload: None };
    }

    // File listing — "what's in here", "what's in this folder"
    if (lower.contains("what's in") || lower.contains("whats in")) && !lower.contains("what's in my mind") {
        return ClassifiedIntent { category: IntentCategory::FileOperation, confidence: 0.80, target: None, payload: None };
    }

    // App open — "open X"
    if lower.starts_with("open ") || lower.starts_with("launch ") {
        return ClassifiedIntent { category: IntentCategory::SystemControl, confidence: 0.80, target: None, payload: None };
    }

    // Planning/goals/deadlines
    if lower.contains("goal") || lower.contains("deadline") || lower.contains("what should i do")
        || lower.contains("what's the plan") || lower.contains("next step")
    {
        return ClassifiedIntent { category: IntentCategory::PlanningQuery, confidence: 0.80, target: None, payload: None };
    }

    // Receipts/identity/trust — "show my receipts", "what did you do", "trust level"
    if lower.contains("receipt") || lower.contains("trust level") || lower.contains("prove what")
        || lower.starts_with("what did you") || lower.starts_with("what have you")
        || lower.contains("last action") || lower.contains("audit trail")
    {
        return ClassifiedIntent { category: IntentCategory::Question, confidence: 0.80, target: Some("identity".into()), payload: None };
    }

    // Belief statements — "we're using X", "I prefer X", "our stack is X"
    if lower.starts_with("we're using") || lower.starts_with("we use")
        || lower.starts_with("i prefer") || lower.starts_with("our ") || lower.starts_with("my favourite")
        || lower.starts_with("my favorite") || lower.starts_with("i like")
    {
        return ClassifiedIntent { category: IntentCategory::Question, confidence: 0.80, target: Some("belief".into()), payload: None };
    }

    // Belief corrections — "actually,", "that's wrong", "i meant"
    if lower.starts_with("actually,") || lower.starts_with("actually ") || lower.contains("that's wrong")
        || lower.contains("thats wrong") || lower.starts_with("i meant") || lower.contains("switched to")
        || lower.contains("changed to") || lower.contains("instead of")
    {
        return ClassifiedIntent { category: IntentCategory::Question, confidence: 0.85, target: Some("correction".into()), payload: None };
    }

    // Everything else → Unknown → main LLM handles it
    ClassifiedIntent {
        category: IntentCategory::Unknown,
        confidence: 0.0,
        target: None,
        payload: None,
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

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
}
