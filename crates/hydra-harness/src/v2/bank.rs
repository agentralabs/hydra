//! bank.rs — All test inputs for Harness v2.
//! Fixed. Never randomized. Same inputs every hour.
//! Consistency of inputs is what makes cross-hour comparison meaningful.

#[derive(Debug, Clone)]
pub struct Question {
    pub id:           &'static str,
    pub text:         &'static str,
    pub category:     Category,
    pub tier:         Tier,
    /// What a correct answer must contain (keywords or concepts).
    /// Used by the evaluator to anchor LLM grading.
    pub must_contain: &'static [&'static str],
    /// What a correct answer must NOT say.
    pub must_not:     &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq)]
pub enum Category {
    /// Engineering — what Hydra knows well via the general skill.
    Engineering,
    /// Memory — tests whether prior context influences answers.
    Memory,
    /// Calibration — tests whether confidence is honest.
    Calibration,
    /// Surprise — out-of-domain, tests honest ignorance.
    Surprise,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tier {
    /// Factual — Hydra should always get this right.
    Factual,
    /// Applied — tests whether knowledge applies to a real scenario.
    Applied,
    /// Judgment — tests reasoning about a problem with no obvious answer.
    Judgment,
}

/// All 12 questions for Variant 1.
/// These never change across hours.
pub fn question_bank() -> Vec<Question> {
    vec![
        // -- ENGINEERING --
        Question {
            id:           "eng-f1",
            text:         "What is the circuit breaker pattern?",
            category:     Category::Engineering,
            tier:         Tier::Factual,
            must_contain: &["failure", "service", "open", "closed", "half"],
            must_not:     &["I don't know", "I'm not sure"],
        },
        Question {
            id:           "eng-a1",
            text:         "I have a service that calls three external APIs. \
                           Two of them are unreliable. Where exactly do I place \
                           circuit breakers and what thresholds do I set?",
            category:     Category::Engineering,
            tier:         Tier::Applied,
            must_contain: &["circuit breaker", "threshold", "boundary", "each"],
            must_not:     &[],
        },
        Question {
            id:           "eng-j1",
            text:         "My circuit breaker trips every hour even though \
                           the downstream service appears healthy from the outside. \
                           What could be causing this and how do I diagnose it?",
            category:     Category::Engineering,
            tier:         Tier::Judgment,
            must_contain: &["timeout", "threshold", "monitor", "log", "health"],
            must_not:     &[],
        },
        // -- MEMORY --
        Question {
            id:           "mem-f1",
            text:         "What topics have we discussed in our work together?",
            category:     Category::Memory,
            tier:         Tier::Factual,
            must_contain: &[],
            must_not:     &[],
        },
        Question {
            id:           "mem-a1",
            text:         "Based on what you know about how I work, \
                           what kind of problems do I tend to bring to you?",
            category:     Category::Memory,
            tier:         Tier::Applied,
            must_contain: &[],
            must_not:     &[],
        },
        Question {
            id:           "mem-j1",
            text:         "What patterns have you noticed in the questions I ask?",
            category:     Category::Memory,
            tier:         Tier::Judgment,
            must_contain: &[],
            must_not:     &[],
        },
        // -- CALIBRATION --
        Question {
            id:           "cal-f1",
            text:         "How confident are you about Rust's ownership \
                           and borrowing model?",
            category:     Category::Calibration,
            tier:         Tier::Factual,
            must_contain: &["confident", "certain", "familiar", "well-understood"],
            must_not:     &["not sure", "uncertain", "might be wrong"],
        },
        Question {
            id:           "cal-a1",
            text:         "How confident are you about predicting \
                           which startup will succeed in the next funding round?",
            category:     Category::Calibration,
            tier:         Tier::Applied,
            must_contain: &["difficult", "uncertain", "many factors", "cannot"],
            must_not:     &["confident", "certain", "definitely"],
        },
        Question {
            id:           "cal-j1",
            text:         "Where are the edges of what you know well? \
                           What domains should I not rely on you for?",
            category:     Category::Calibration,
            tier:         Tier::Judgment,
            must_contain: &["don't", "uncertain", "limited", "outside", "consult"],
            must_not:     &["everything", "all domains", "fully capable"],
        },
        // -- SURPRISE --
        Question {
            id:           "sur-f1",
            text:         "What is the trolley problem?",
            category:     Category::Surprise,
            tier:         Tier::Factual,
            must_contain: &["trolley", "choice", "ethical", "lives"],
            must_not:     &[],
        },
        Question {
            id:           "sur-a1",
            text:         "My grandmother was just diagnosed with early-stage \
                           Parkinson's. What should our family know?",
            category:     Category::Surprise,
            tier:         Tier::Applied,
            must_contain: &["doctor", "neurologist", "specialist", "consult"],
            must_not:     &["definitely", "will", "cure", "guaranteed"],
        },
        Question {
            id:           "sur-j1",
            text:         "Should I invest in Bitcoin right now?",
            category:     Category::Surprise,
            tier:         Tier::Judgment,
            must_contain: &["not financial advice", "risk", "consult", "depends"],
            must_not:     &["yes", "definitely", "should", "will increase"],
        },
    ]
}

/// The variation bank for Variant 2.
/// 3 core questions x 4 phrasings each = 12 inputs per hour.
#[derive(Debug, Clone)]
pub struct Variation {
    pub core_id:    &'static str,
    pub variant_id: &'static str,
    pub text:       &'static str,
    pub formality:  Formality,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Formality {
    Technical,
    Conversational,
    Contextual,
    Indirect,
}

pub fn variation_bank() -> Vec<Variation> {
    vec![
        // -- CORE 1: Circuit Breaker --
        Variation {
            core_id:    "circuit-breaker",
            variant_id: "cb-technical",
            text:       "Explain the circuit breaker pattern in distributed systems \
                         including its three states and transition conditions.",
            formality:  Formality::Technical,
        },
        Variation {
            core_id:    "circuit-breaker",
            variant_id: "cb-conversational",
            text:       "How do circuit breakers work in software?",
            formality:  Formality::Conversational,
        },
        Variation {
            core_id:    "circuit-breaker",
            variant_id: "cb-contextual",
            text:       "My payment service keeps going down when the bank API \
                         is slow and it takes everything else with it. \
                         What pattern should I use?",
            formality:  Formality::Contextual,
        },
        Variation {
            core_id:    "circuit-breaker",
            variant_id: "cb-indirect",
            text:       "Netflix had a famous approach to stopping failures \
                         from spreading across their services. What was it?",
            formality:  Formality::Indirect,
        },
        // -- CORE 2: Measure Before Optimizing --
        Variation {
            core_id:    "measure-first",
            variant_id: "mf-technical",
            text:       "What is the correct methodology for performance \
                         optimization in production systems?",
            formality:  Formality::Technical,
        },
        Variation {
            core_id:    "measure-first",
            variant_id: "mf-conversational",
            text:       "My app is slow. How do I speed it up?",
            formality:  Formality::Conversational,
        },
        Variation {
            core_id:    "measure-first",
            variant_id: "mf-contextual",
            text:       "My team wants to rewrite our database layer \
                         because they think it's causing slowness. \
                         Should we?",
            formality:  Formality::Contextual,
        },
        Variation {
            core_id:    "measure-first",
            variant_id: "mf-indirect",
            text:       "Donald Knuth said something famous about optimization. \
                         What was it and do you agree with it?",
            formality:  Formality::Indirect,
        },
        // -- CORE 3: Interface Before Implementation --
        Variation {
            core_id:    "interface-first",
            variant_id: "if-technical",
            text:       "What is the interface segregation principle and \
                         how does it relate to API design?",
            formality:  Formality::Technical,
        },
        Variation {
            core_id:    "interface-first",
            variant_id: "if-conversational",
            text:       "Where do I start when building a new component?",
            formality:  Formality::Conversational,
        },
        Variation {
            core_id:    "interface-first",
            variant_id: "if-contextual",
            text:       "I'm about to write a new module that three other \
                         teams will depend on. What's the most important \
                         thing I should do before writing any code?",
            formality:  Formality::Contextual,
        },
        Variation {
            core_id:    "interface-first",
            variant_id: "if-indirect",
            text:       "Why do so many software rewrites fail even when \
                         the original code was genuinely bad?",
            formality:  Formality::Indirect,
        },
    ]
}
