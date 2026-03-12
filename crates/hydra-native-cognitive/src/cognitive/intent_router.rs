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
//!
//! Classification logic lives in `intent_router_classify.rs`.
//! Tests live in `intent_router_tests.rs`.

// Re-export the public classify entry-point so callers keep using
// `intent_router::classify` unchanged.
pub use super::intent_router_classify::classify;

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
    SisterImprove,   // "improve the memory sister", "make codebase better"

    // Self management
    SelfRepair,      // "fix yourself", "run self-repair"
    SelfScan,        // "scan yourself", "omniscience scan"
    SelfImplement,   // "implement this spec", "build this yourself"

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

    // Threat intelligence
    ThreatQuery,     // "what's the threat level?", "show threats"

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
    pub(crate) fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().replace('-', "_").as_str() {
            "greeting" => Self::Greeting,
            "farewell" => Self::Farewell,
            "thanks" => Self::Thanks,
            "memory_store" => Self::MemoryStore,
            "memory_recall" => Self::MemoryRecall,
            "sister_diagnose" => Self::SisterDiagnose,
            "sister_repair" => Self::SisterRepair,
            "sister_improve" => Self::SisterImprove,
            "self_repair" => Self::SelfRepair,
            "self_scan" => Self::SelfScan,
            "self_implement" => Self::SelfImplement,
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
            "threat" | "threat_query" => Self::ThreatQuery,
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

pub(crate) const SISTER_NAMES: &[&str] = &[
    "memory", "identity", "codebase", "vision", "comm", "contract",
    "time", "planning", "cognition", "reality", "forge", "aegis",
    "veritas", "evolve",
];

// ═══════════════════════════════════════════════════════════════════
// Classification prompt — sent to the cheapest/fastest LLM
// ~120 input tokens + ~30 output tokens = ~150 total
// ═══════════════════════════════════════════════════════════════════

pub(crate) const CLASSIFICATION_PROMPT: &str = "\
Classify this user message into exactly ONE category.\n\
Return ONLY a JSON object, nothing else.\n\n\
Categories:\n\
- sister_diagnose: checking status/health of a sister/component\n\
- sister_repair: fixing/restarting/healing a sister/component\n\
- sister_improve: improving/enhancing/upgrading a sister's capabilities\n\
- self_scan: analyzing own code/health/problems\n\
- self_repair: fixing own issues\n\
- self_implement: implementing a spec/feature on itself, building capabilities\n\
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
- threat_query: threat level/security status/attack detection\n\
- greeting: hi/hello/hey\n\
- farewell: bye/goodbye/see you\n\
- thanks: thank you/thanks/ty\n\
- conversation: opinions/questions/discussion/jokes\n\n\
Sisters are named components: memory, identity, codebase, vision, comm, contract, time, planning, cognition, reality, forge, aegis, veritas, evolve.\n\
Pronouns like \"her\", \"it\", \"that\" referring to a sister in context = sister target.\n\n";
