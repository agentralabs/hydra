use hydra_core::types::CompiledIntent;

/// Complexity of the compiled intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Complexity {
    Simple,
    Moderate,
    Complex,
    Critical,
}

impl Complexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Moderate => "moderate",
            Self::Complex => "complex",
            Self::Critical => "critical",
        }
    }
}

/// Status of a compilation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileStatus {
    /// Successfully compiled
    Compiled,
    /// Served from cache (0 tokens)
    Cached,
    /// Classified locally (0 tokens)
    LocallyClassified,
    /// Matched via fuzzy (0 tokens)
    FuzzyMatched,
    /// Required LLM (used tokens)
    LlmCompiled,
    /// Input was empty/whitespace
    Empty,
    /// Budget exhausted — couldn't compile
    BudgetExhausted,
    /// Input needs clarification (ambiguous)
    NeedsClarification,
    /// Input contains contradictions
    Contradiction,
    /// Input too long (truncated and compiled)
    Truncated,
}

/// Result of intent compilation
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub intent: Option<CompiledIntent>,
    pub status: CompileStatus,
    pub tokens_used: u64,
    pub layer: u8, // Which layer resolved it (1-4)
    pub warnings: Vec<String>,
    pub complexity: Complexity,
    pub entities_extracted: usize,
}

impl CompileResult {
    pub fn is_ok(&self) -> bool {
        self.intent.is_some()
    }

    pub fn is_cached(&self) -> bool {
        self.status == CompileStatus::Cached
    }

    pub fn has_warning(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn asks_clarification(&self) -> bool {
        self.status == CompileStatus::NeedsClarification
    }

    pub fn is_safe(&self) -> bool {
        !self.warnings.iter().any(|w| w.contains("injection"))
    }

    pub fn contains_dangerous_patterns(&self) -> bool {
        self.warnings.iter().any(|w| w.contains("injection"))
    }

    pub fn has_uncertainty(&self) -> bool {
        self.intent
            .as_ref()
            .map(|i| i.confidence < 0.7)
            .unwrap_or(true)
    }
}
