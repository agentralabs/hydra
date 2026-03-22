//! OutputPacer — governs ALL output rendering speed.
//!
//! Nothing bypasses the pacer. Every piece of content that appears
//! in the stream goes through `delay_ms()` and `chars_per_frame()`.
//! Pacing carries information: fast=routine, slow=important, hold=urgent.

use crate::constants;

/// The kind of content being rendered, which affects pacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    /// Normal assistant text (character-by-character streaming).
    AssistantText,
    /// Sentence boundary — pause after ".".
    SentenceBoundary,
    /// Paragraph boundary — pause after double newline.
    ParagraphBoundary,
    /// Code block start — pause before ```.
    CodeBlockStart,
    /// Tool status dots and connectors.
    ToolFeedback,
    /// Tool result lines (between result lines).
    ToolResultLine,
    /// Section separator (between sections).
    SectionBreak,
    /// Table row rendering.
    TableRow,
    /// Bullet item rendering.
    BulletItem,
    /// Critical system notifications (errors — SLOWER).
    Critical,
    /// Error section pause.
    ErrorSection,
    /// Companion task status updates.
    CompanionUpdate,
    /// Dream/briefing notifications.
    BackgroundNotification,
    /// Urgent briefing item (holds longer).
    UrgentBriefing,
    /// Informational briefing item.
    InfoBriefing,
    /// User echo (instant — never paced).
    UserEcho,
}

/// Signals from the UI that affect pacer speed.
#[derive(Debug, Clone, Copy, Default)]
pub struct PacerSignals {
    /// User is currently scrolling.
    pub scrolling: bool,
    /// User is currently typing.
    pub typing: bool,
    /// Content is an error.
    pub is_error: bool,
    /// Content is urgent.
    pub is_urgent: bool,
    /// Content needs approval.
    pub needs_approval: bool,
}

/// The output pacer — controls rendering speed for all content.
#[derive(Debug, Clone)]
pub struct OutputPacer {
    /// Current signals from UI.
    signals: PacerSignals,
    /// Characters rendered so far for the current item.
    chars_rendered: usize,
    /// Current speed multiplier (1.0 = normal).
    speed: f64,
}

impl OutputPacer {
    /// Create a new output pacer.
    pub fn new() -> Self {
        Self {
            signals: PacerSignals::default(),
            chars_rendered: 0,
            speed: 1.0,
        }
    }

    /// Update the pacer with new UI signals.
    pub fn update_signals(&mut self, signals: PacerSignals) {
        self.signals = signals;
        self.recalculate_speed();
    }

    /// Recalculate the speed multiplier from current signals.
    fn recalculate_speed(&mut self) {
        let mut s = 1.0;
        if self.signals.scrolling {
            s *= constants::PACER_SCROLL_ACCEL;
        }
        if self.signals.typing {
            s *= constants::PACER_TYPING_ACCEL;
        }
        if self.signals.is_error || self.signals.is_urgent || self.signals.needs_approval {
            s *= constants::PACER_CRITICAL_DECEL;
        }
        self.speed = s;
    }

    /// Reset the character counter for a new content item.
    pub fn reset_for_new_item(&mut self) {
        self.chars_rendered = 0;
    }

    /// Record that characters were rendered.
    pub fn record_rendered(&mut self, count: usize) {
        self.chars_rendered = self.chars_rendered.saturating_add(count);
    }

    /// Compute the delay in milliseconds for the given content kind.
    pub fn delay_ms(&self, kind: ContentKind) -> u64 {
        let raw = match kind {
            ContentKind::UserEcho => return 0,
            ContentKind::AssistantText => 33, // ~30fps for char-by-char
            ContentKind::SentenceBoundary => constants::PACER_SENTENCE_PAUSE_MS,
            ContentKind::ParagraphBoundary => constants::PACER_PARAGRAPH_PAUSE_MS,
            ContentKind::CodeBlockStart => constants::PACER_CODE_BLOCK_PAUSE_MS,
            ContentKind::ToolFeedback => constants::PACER_DOT_PAUSE_MS,
            ContentKind::ToolResultLine => constants::PACER_TOOL_LINE_DELAY_MS,
            ContentKind::SectionBreak => constants::PACER_SECTION_PAUSE_MS,
            ContentKind::TableRow => constants::PACER_TABLE_ROW_DELAY_MS,
            ContentKind::BulletItem => constants::PACER_BULLET_DELAY_MS,
            ContentKind::Critical => constants::PACER_ERROR_SECTION_PAUSE_MS,
            ContentKind::ErrorSection => constants::PACER_ERROR_SECTION_PAUSE_MS,
            ContentKind::CompanionUpdate => constants::PACER_DOT_PAUSE_MS,
            ContentKind::BackgroundNotification => constants::PACER_INFO_HOLD_MS,
            ContentKind::UrgentBriefing => constants::PACER_URGENT_HOLD_MS,
            ContentKind::InfoBriefing => constants::PACER_INFO_HOLD_MS,
        };

        // Apply speed multiplier (higher speed = lower delay).
        let delay = raw as f64 / self.speed;
        delay.max(1.0) as u64
    }

    /// Compute how many characters to render this frame.
    pub fn chars_per_frame(&self, kind: ContentKind) -> usize {
        match kind {
            ContentKind::UserEcho => usize::MAX,
            _ => {
                let base = constants::PACER_CHARS_PER_FRAME as f64;
                (base * self.speed).max(1.0) as usize
            }
        }
    }

    /// Whether the current content should offer truncation.
    pub fn should_truncate(&self) -> bool {
        self.chars_rendered >= constants::PACER_TRUNCATION_THRESHOLD
    }

    /// Return the number of characters rendered so far.
    pub fn chars_rendered(&self) -> usize {
        self.chars_rendered
    }

    /// Return the current speed multiplier.
    pub fn speed(&self) -> f64 {
        self.speed
    }
}

impl Default for OutputPacer {
    fn default() -> Self {
        Self::new()
    }
}
