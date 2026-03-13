//! CognitiveUpdate — all update variants emitted by the cognitive loop for the UI.
//!
//! Extracted from loop_runner.rs for file size compliance (was 401 lines).

use hydra_native_state::state::hydra::PhaseStatus;

/// Updates emitted by the cognitive loop for the UI to consume.
#[derive(Debug, Clone)]
pub enum CognitiveUpdate {
    Phase(String),
    IconState(String),
    PhaseStatuses(Vec<PhaseStatus>),
    Typing(bool),

    // -- Plan panel --
    PlanInit { goal: String, steps: Vec<String> },
    PlanClear,
    PlanStepStart(usize),
    PlanStepComplete { index: usize, duration_ms: Option<u64> },

    // -- Evidence panel --
    EvidenceClear,
    EvidenceMemory { title: String, content: String },
    EvidenceCode {
        title: String,
        content: String,
        language: Option<String>,
        file_path: Option<String>,
    },

    // -- Timeline panel --
    TimelineClear,

    // -- Messages --
    Message { role: String, content: String, css_class: String },

    // -- Sidebar --
    SidebarCompleteTask(String),

    // -- Celebration --
    Celebrate(String),

    // -- Final state --
    ResetIdle,
    SuggestMode(String),

    // -- Approval flow --
    AwaitApproval {
        approval_id: Option<String>,
        risk_level: String,
        action: String,
        description: String,
        challenge_phrase: Option<String>,
    },

    // -- Settings --
    SettingsApplied { confirmation: String },

    // -- Sister visibility --
    SistersCalled { sisters: Vec<String> },

    // -- Token budget --
    TokenUsage { input_tokens: u64, output_tokens: u64 },

    // -- Streaming --
    StreamChunk { content: String },
    StreamComplete,

    // -- Undo/Redo --
    UndoStatus { can_undo: bool, can_redo: bool, last_action: Option<String> },

    // -- Proactive notifications --
    ProactiveAlert { title: String, message: String, priority: String },

    // -- Proactive File Watcher (P2) --
    ProactiveFileSuggestion { title: String, message: String, priority: String, action: Option<String> },

    // -- Inventions --
    SkillCrystallized { name: String, actions_count: usize },
    ReflectionInsight { insight: String },
    CompressionApplied { original_tokens: usize, compressed_tokens: usize, ratio: f64 },
    DreamInsight { category: String, description: String, confidence: f64 },
    ShadowValidation { safe: bool, recommendation: String },
    PredictionResult { action: String, confidence: f64, recommendation: String },
    PatternEvolved { summary: String },
    TemporalStored { category: String, content: String },

    // -- Ghost Cursor --
    CursorMove { x: f64, y: f64, label: Option<String> },
    CursorClick,
    CursorTyping { active: bool },
    CursorVisibility { visible: bool },
    CursorModeChange { mode: String },
    CursorPaused { paused: bool },

    // -- Belief system --
    BeliefsLoaded { count: usize, summary: String },
    BeliefUpdated { subject: String, content: String, confidence: f64, is_new: bool },

    // -- MCP Skill Discovery --
    McpSkillsDiscovered { server: String, tools: Vec<String>, count: usize },

    // -- Federation --
    FederationSync { peers_online: usize, last_sync_version: i64 },
    FederationDelegated { peer_name: String, task_summary: String },

    // -- Self-Repair --
    RepairStarted { spec: String, task: String },
    RepairCheckResult { name: String, passed: bool },
    RepairIteration { iteration: u32, passed: usize, total: usize },
    RepairCompleted { task: String, status: String, iterations: u32 },

    // -- Omniscience Loop --
    OmniscienceAnalyzing { phase: String },
    OmniscienceGapFound { description: String, severity: String, category: String },
    OmniscienceSpecGenerated { spec_name: String, task: String },
    OmniscienceValidation { spec_name: String, safe: bool, recommendation: String },
    OmniscienceScanComplete { gaps_found: usize, specs_generated: usize, health_score: f64 },

    // -- Phase loading --
    PhaseLoading { phase: String, elapsed_ms: u64 },

    // -- Consolidation daemon --
    ConsolidationCycleComplete { cycle: u64, strengthened: usize, decayed: usize, gc_cleaned: usize },

    // -- Obstacle resolution --
    ObstacleDetected { pattern: String, error_summary: String },
    ObstacleResolved { pattern: String, resolution: String, attempts: usize },

    // -- Autonomous project execution --
    ProjectExecPhase { repo: String, phase: String, detail: String },

    // -- Agent Swarm --
    SwarmSpawned { count: usize, agent_ids: Vec<String> },
    SwarmTaskAssigned { agent_id: String, task_desc: String },
    SwarmResults { total: usize, succeeded: usize, failed: usize, summary: String },

    // -- Agentic Loop (Phase 1: Multi-Turn) --
    /// Progress update for each agentic loop turn.
    AgenticTurn { turn: u8, tool_count: usize, exec_count: usize },
    /// Agentic loop completed.
    AgenticComplete { turns: u8, total_tokens: u64, stop_reason: String },

    // -- Response Verification (Phase 2) --
    VerificationApplied { checked: usize, corrected: usize },

    // -- Model Escalation (Phase 4) --
    ModelEscalated { from: String, to: String, reason: String },

    // -- Background Tasks (Phase 5) --
    BackgroundTaskComplete { task_name: String, summary: String },

    // -- Metacognition (Phase 7) --
    MetacognitiveInsight { assessment: String },

    // -- Build System (full system builder) --
    BuildPhaseStarted { phase: String, detail: String },
    BuildProgress { phase: String, completed: usize, total: usize },
    BuildPhaseComplete { phase: String, duration_ms: u64, summary: String },
    BuildComplete { report: String },
    BuildFailed { phase: String, error: String },

    // -- Tool Actions (Claude Code-style display) --
    ToolAction { tool: String, args: String, result: String, success: bool },
}
