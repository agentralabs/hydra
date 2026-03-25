//! V3 Test Bank — shared types + combined bank.
//! Part A: 32 operational tests (calibrate orchestrations as Hydra grows).
//! Part B: 28 real-user-day tests (simulate a full day of use).
//! Part C: 28 orchestration tests (100% O0-O25 coverage).
//! Total: 88 tests across 17 categories.

/// Test category — original 6 + 7 new real-user-day categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum V3Category {
    // ── Part A: Operational calibration ──
    DropGateway,
    Conductor,
    Security,
    OpLearning,
    Monitor,
    Background,
    // ── Part B: Real user day ──
    MorningRoutine,
    CodingSession,
    Monitoring,
    Learning,
    Communication,
    Safety,
    Persistence,
    // ── Part C: Orchestration coverage ──
    OrchFoundation,
    OrchExecution,
    OrchIntelligence,
    OrchPresence,
}

impl V3Category {
    pub fn label(&self) -> &'static str {
        match self {
            Self::DropGateway => "Drop Gateway",
            Self::Conductor => "Conductor",
            Self::Security => "Security",
            Self::OpLearning => "Op:Learning",
            Self::Monitor => "Op:Monitor",
            Self::Background => "Background",
            Self::MorningRoutine => "Morning Routine",
            Self::CodingSession => "Coding Session",
            Self::Monitoring => "Monitoring",
            Self::Learning => "Learning",
            Self::Communication => "Communication",
            Self::Safety => "Safety",
            Self::Persistence => "Persistence",
            Self::OrchFoundation => "Orch:Foundation",
            Self::OrchExecution => "Orch:Execution",
            Self::OrchIntelligence => "Orch:Intelligence",
            Self::OrchPresence => "Orch:Presence",
        }
    }

    /// Equal weight per category present.
    pub fn weight(&self, total_categories: usize) -> f64 {
        1.0 / total_categories.max(1) as f64
    }

    /// Safety category blocks deployment on ANY failure.
    pub fn is_blocking(&self) -> bool { matches!(self, Self::Safety | Self::Security) }

    /// Is this a Part A (ops) category?
    pub fn is_ops(&self) -> bool {
        matches!(self, Self::DropGateway | Self::Conductor | Self::Security
            | Self::OpLearning | Self::Monitor | Self::Background)
    }

    /// All unique categories found in a test bank.
    pub fn categories_in(tests: &[V3Test]) -> Vec<V3Category> {
        let mut cats: Vec<V3Category> = Vec::new();
        for t in tests {
            if !cats.contains(&t.category) { cats.push(t.category); }
        }
        cats
    }
}

/// How a test is evaluated.
#[derive(Debug, Clone, Copy)]
pub enum EvalMethod {
    FileCheck,
    SubprocessCheck,
    DirectCheck,
    OutputCheck,
    LlmGrade,
}

/// A single V3 test definition.
#[derive(Debug, Clone)]
pub struct V3Test {
    pub id: &'static str,
    pub name: &'static str,
    pub category: V3Category,
    pub eval_method: EvalMethod,
    pub input: &'static str,
    pub pass_contains: &'static [&'static str],
    pub fail_contains: &'static [&'static str],
    pub min_hour: u32,
    pub timeout_secs: u64,
}

/// Combined test bank: 32 ops + 28 day + 28 orch = 88 tests.
pub fn test_bank() -> Vec<V3Test> {
    let mut all = super::bank_ops::ops_tests();
    all.extend(super::bank_day::day_tests());
    all.extend(super::bank_orch::orch_tests());
    all
}

/// Only operational calibration tests (original 32).
pub fn ops_bank() -> Vec<V3Test> { super::bank_ops::ops_tests() }

/// Only real-user-day tests (new 28).
pub fn day_bank() -> Vec<V3Test> { super::bank_day::day_tests() }

/// Only orchestration coverage tests (28).
pub fn orch_bank() -> Vec<V3Test> { super::bank_orch::orch_tests() }
