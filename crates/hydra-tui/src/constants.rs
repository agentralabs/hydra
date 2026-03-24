//! All constants for hydra-tui.
//! No magic numbers anywhere else in this crate.
//! Every UI color, timing, and layout value lives here.

// ---------------------------------------------------------------------------
// Brand palette — from Go reference template (palette.go)
// ---------------------------------------------------------------------------

/// Primary accent, borders, headers, user label, assistant text.
pub const HYDRA_BLUE: (u8, u8, u8) = (100, 149, 237);

/// Username, keywords, active tool.
pub const HYDRA_CYAN: (u8, u8, u8) = (0, 210, 210);

/// Success, connected, git branch.
pub const HYDRA_GREEN: (u8, u8, u8) = (80, 200, 120);

/// Error, offline, critical.
pub const HYDRA_RED: (u8, u8, u8) = (220, 80, 80);

/// Warning, uncertain.
pub const HYDRA_YELLOW: (u8, u8, u8) = (240, 200, 80);

/// Approval, action needed.
pub const HYDRA_ORANGE: (u8, u8, u8) = (240, 160, 60);

/// Model name, thinking phase.
pub const HYDRA_PURPLE: (u8, u8, u8) = (160, 120, 220);

/// Labels, paths, hints.
pub const HYDRA_DIM: (u8, u8, u8) = (128, 128, 128);

// ---------------------------------------------------------------------------
// Output pacer timings (milliseconds) — from HYDRA-TUI-ARCHITECTURE spec
// ---------------------------------------------------------------------------

/// Characters rendered per frame (at 30fps = 600 chars/sec).
pub const PACER_CHARS_PER_FRAME: usize = 20;

/// Pause at sentence boundary "." (ms).
pub const PACER_SENTENCE_PAUSE_MS: u64 = 80;

/// Pause at paragraph boundary (double newline) (ms).
pub const PACER_PARAGRAPH_PAUSE_MS: u64 = 120;

/// Pause before code block (```) (ms).
pub const PACER_CODE_BLOCK_PAUSE_MS: u64 = 200;

/// Pause before dot content appears (ms).
pub const PACER_DOT_PAUSE_MS: u64 = 50;

/// Pause before connector content appears (ms).
pub const PACER_CONNECTOR_PAUSE_MS: u64 = 100;

/// Delay between tool result lines (ms).
pub const PACER_TOOL_LINE_DELAY_MS: u64 = 200;

/// Pause between sections (ms).
pub const PACER_SECTION_PAUSE_MS: u64 = 300;

/// Delay between table rows (ms).
pub const PACER_TABLE_ROW_DELAY_MS: u64 = 200;

/// Delay between bullet items (ms).
pub const PACER_BULLET_DELAY_MS: u64 = 300;

/// Pause between error sections (ms).
pub const PACER_ERROR_SECTION_PAUSE_MS: u64 = 300;

/// Hold time for urgent briefing items (ms).
pub const PACER_URGENT_HOLD_MS: u64 = 500;

/// Hold time for informational briefing items (ms).
pub const PACER_INFO_HOLD_MS: u64 = 300;

/// Acceleration multiplier when user is scrolling.
pub const PACER_SCROLL_ACCEL: f64 = 2.0;

/// Acceleration multiplier when user is typing.
pub const PACER_TYPING_ACCEL: f64 = 5.0;

/// Deceleration multiplier for critical/error content.
pub const PACER_CRITICAL_DECEL: f64 = 0.5;

/// Maximum characters before truncation is offered.
pub const PACER_TRUNCATION_THRESHOLD: usize = 4096;

/// Maximum tool output lines before truncation.
pub const PACER_TOOL_TRUNCATION_LINES: usize = 50;

// ---------------------------------------------------------------------------
// Thinking verb colors — 12 permanent (R, G, B) tuples
// From HYDRA-THINKING-VERBS.md — these NEVER change.
// ---------------------------------------------------------------------------

/// Amber — General context (Cogitating, Ruminating, Deliberating, Musing).
pub const VERB_COLOR_GENERAL: (u8, u8, u8) = (200, 169, 110);

/// Coral — Forge context (Forging, Smithing, Blueprinting, Crafting).
pub const VERB_COLOR_FORGE: (u8, u8, u8) = (200, 112, 74);

/// Cyan — Codebase context (Scanning, Parsing, Traversing, Indexing).
pub const VERB_COLOR_CODEBASE: (u8, u8, u8) = (106, 184, 212);

/// Green — Memory context (Remembering, Recollecting, Excavating, Surfacing).
pub const VERB_COLOR_MEMORY: (u8, u8, u8) = (74, 170, 106);

/// Purple — Synthesis/Cognition context (Synthesizing, Ideating, Contemplating, Composing).
pub const VERB_COLOR_SYNTHESIS: (u8, u8, u8) = (138, 106, 191);

/// Blue — Workflow context (Orchestrating, Sequencing, Pipelining, Routing).
pub const VERB_COLOR_WORKFLOW: (u8, u8, u8) = (106, 138, 191);

/// Teal — Veritas context (Verifying, Truthing, Validating, Cross-checking).
pub const VERB_COLOR_VERITAS: (u8, u8, u8) = (74, 200, 160);

/// Red — Aegis context (Shielding, Fortifying, Guarding, Sentineling).
pub const VERB_COLOR_AEGIS: (u8, u8, u8) = (200, 74, 74);

/// Indigo — Dream context (Dreaming, Drifting, Night-thinking, Star-gazing).
pub const VERB_COLOR_DREAM: (u8, u8, u8) = (122, 106, 200);

/// Pink — Persona context (Channeling, Voicing, Shifting, Embodying).
pub const VERB_COLOR_PERSONA: (u8, u8, u8) = (200, 106, 154);

/// Sage — Data context (Crunching, Munging, Tabulating, Correlating).
pub const VERB_COLOR_DATA: (u8, u8, u8) = (138, 200, 122);

/// Gold — HydraBranded context (Hydrating, Multi-minding, Ring-resonating).
pub const VERB_COLOR_HYDRA_BRANDED: (u8, u8, u8) = (232, 200, 122);

/// All 12 verb colors in order, for iteration.
pub const ALL_VERB_COLORS: [(u8, u8, u8); 12] = [
    VERB_COLOR_GENERAL,
    VERB_COLOR_FORGE,
    VERB_COLOR_CODEBASE,
    VERB_COLOR_MEMORY,
    VERB_COLOR_SYNTHESIS,
    VERB_COLOR_WORKFLOW,
    VERB_COLOR_VERITAS,
    VERB_COLOR_AEGIS,
    VERB_COLOR_DREAM,
    VERB_COLOR_PERSONA,
    VERB_COLOR_DATA,
    VERB_COLOR_HYDRA_BRANDED,
];

// ---------------------------------------------------------------------------
// Dot colors — 7 kinds (R, G, B) — from TUI architecture spec
// ---------------------------------------------------------------------------

/// Dim — Active / Working.
pub const DOT_COLOR_ACTIVE: (u8, u8, u8) = HYDRA_DIM;

/// Green — Success / Complete.
pub const DOT_COLOR_SUCCESS: (u8, u8, u8) = HYDRA_GREEN;

/// Red — Error / Failure / Alert.
pub const DOT_COLOR_ERROR: (u8, u8, u8) = HYDRA_RED;

/// Yellow — Narration / Thinking.
pub const DOT_COLOR_NARRATION: (u8, u8, u8) = HYDRA_YELLOW;

/// Cyan — Read / Search / Query.
pub const DOT_COLOR_READ: (u8, u8, u8) = HYDRA_CYAN;

/// Purple — Memory / Belief / Cognitive.
pub const DOT_COLOR_COGNITIVE: (u8, u8, u8) = HYDRA_PURPLE;

/// Orange — Companion / Background.
pub const DOT_COLOR_COMPANION: (u8, u8, u8) = HYDRA_ORANGE;

/// All 7 dot colors in order, for iteration.
pub const ALL_DOT_COLORS: [(u8, u8, u8); 7] = [
    DOT_COLOR_ACTIVE,
    DOT_COLOR_SUCCESS,
    DOT_COLOR_ERROR,
    DOT_COLOR_NARRATION,
    DOT_COLOR_READ,
    DOT_COLOR_COGNITIVE,
    DOT_COLOR_COMPANION,
];

// ---------------------------------------------------------------------------
// UI chrome colors
// ---------------------------------------------------------------------------

/// Status bar background color.
pub const STATUS_BAR_BG: (u8, u8, u8) = (30, 30, 46);

/// Status bar foreground color.
pub const STATUS_BAR_FG: (u8, u8, u8) = (205, 214, 244);

/// Input box border color when focused.
pub const INPUT_BORDER_FOCUSED: (u8, u8, u8) = HYDRA_BLUE;

/// Input box border color when unfocused.
pub const INPUT_BORDER_UNFOCUSED: (u8, u8, u8) = HYDRA_DIM;

/// Welcome screen accent color.
pub const WELCOME_ACCENT: (u8, u8, u8) = HYDRA_CYAN;

/// Stream area background color.
pub const STREAM_BG: (u8, u8, u8) = (24, 24, 37);

/// User message color.
pub const USER_MESSAGE_COLOR: (u8, u8, u8) = HYDRA_BLUE;

/// Assistant text color.
pub const ASSISTANT_TEXT_COLOR: (u8, u8, u8) = HYDRA_BLUE;

/// System notification color.
pub const SYSTEM_NOTIFICATION_COLOR: (u8, u8, u8) = HYDRA_YELLOW;

/// 6 thinking cycling colors for Go-style spinner.
pub const THINK_CYCLE_COLORS: [(u8, u8, u8); 6] = [
    HYDRA_BLUE,
    HYDRA_GREEN,
    HYDRA_PURPLE,
    HYDRA_ORANGE,
    HYDRA_CYAN,
    HYDRA_YELLOW,
];

// ---------------------------------------------------------------------------
// Spinner frames — Hydra-themed (from HYDRA-THINKING-VERBS.md)
// ◌ → ◐ → ◑ → ◒ → ◓ → ● → ◌
// ---------------------------------------------------------------------------

/// Spinner animation frames for the thinking indicator.
pub const SPINNER_FRAMES: &[&str] = &["◌", "◐", "◑", "◒", "◓", "●"];

/// Interval between spinner frame advances (ms).
pub const SPINNER_INTERVAL_MS: u64 = 180;

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Minimum terminal width in columns.
pub const MIN_TERMINAL_WIDTH: u16 = 60;

/// Minimum terminal height in rows.
pub const MIN_TERMINAL_HEIGHT: u16 = 15;

/// Height of the input box in rows.
pub const INPUT_BOX_HEIGHT: u16 = 3;

/// Height of the status bar in rows.
pub const STATUS_BAR_HEIGHT: u16 = 1;

/// Maximum visible stream items before old ones scroll off.
pub const MAX_VISIBLE_STREAM_ITEMS: usize = 1000;

/// Verb rotation interval (ms) — how often the thinking verb changes.
pub const VERB_ROTATION_INTERVAL_MS: u64 = 2200;

/// Maximum stream buffer size before oldest items are evicted.
pub const MAX_STREAM_BUFFER: usize = 5000;
