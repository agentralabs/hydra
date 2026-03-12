//! Timeline panel component data — chronological events with timestamps and phase indicators.

use serde::{Deserialize, Serialize};

/// The kind of event shown in the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineEventKind {
    /// A cognitive phase started or ended.
    PhaseChange,
    /// A tool was invoked.
    ToolCall,
    /// User approval was requested or granted.
    Approval,
    /// An error or warning occurred.
    Error,
    /// Informational log entry.
    Info,
    /// A sister delegation event.
    Delegation,
}

/// A single event in the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: usize,
    pub timestamp: String,
    pub kind: TimelineEventKind,
    pub title: String,
    pub detail: Option<String>,
    pub phase_label: Option<String>,
    pub duration_ms: Option<u64>,
}

/// The timeline panel view model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePanel {
    pub events: Vec<TimelineEvent>,
    next_id: usize,
    pub auto_scroll: bool,
    pub filter: Option<TimelineEventKind>,
}

impl TimelinePanel {
    /// Create an empty timeline panel.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_id: 0,
            auto_scroll: true,
            filter: None,
        }
    }

    /// Push a new event onto the timeline.
    pub fn push_event(
        &mut self,
        timestamp: &str,
        kind: TimelineEventKind,
        title: &str,
        detail: Option<&str>,
        phase_label: Option<&str>,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.events.push(TimelineEvent {
            id,
            timestamp: timestamp.to_owned(),
            kind,
            title: title.to_owned(),
            detail: detail.map(|s| s.to_owned()),
            phase_label: phase_label.map(|s| s.to_owned()),
            duration_ms: None,
        });
        id
    }

    /// Attach a duration to an existing event by id.
    pub fn set_event_duration(&mut self, id: usize, duration_ms: u64) {
        if let Some(ev) = self.events.iter_mut().find(|e| e.id == id) {
            ev.duration_ms = Some(duration_ms);
        }
    }

    /// Return events matching the current filter, or all events if no filter is set.
    pub fn visible_events(&self) -> Vec<&TimelineEvent> {
        match self.filter {
            Some(kind) => self.events.iter().filter(|e| e.kind == kind).collect(),
            None => self.events.iter().collect(),
        }
    }

    /// Set the event kind filter. Pass `None` to show all events.
    pub fn set_filter(&mut self, kind: Option<TimelineEventKind>) {
        self.filter = kind;
    }

    /// Toggle auto-scroll behavior.
    pub fn toggle_auto_scroll(&mut self) {
        self.auto_scroll = !self.auto_scroll;
    }

    /// Total number of events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Number of error events.
    pub fn error_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.kind == TimelineEventKind::Error)
            .count()
    }

    /// The most recent event, if any.
    pub fn latest_event(&self) -> Option<&TimelineEvent> {
        self.events.last()
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.events.clear();
        self.next_id = 0;
    }

    /// CSS class for an event kind.
    pub fn event_css_class(kind: TimelineEventKind) -> &'static str {
        match kind {
            TimelineEventKind::PhaseChange => "timeline-phase",
            TimelineEventKind::ToolCall => "timeline-tool",
            TimelineEventKind::Approval => "timeline-approval",
            TimelineEventKind::Error => "timeline-error",
            TimelineEventKind::Info => "timeline-info",
            TimelineEventKind::Delegation => "timeline-delegation",
        }
    }

    /// Icon for an event kind.
    pub fn event_icon(kind: TimelineEventKind) -> &'static str {
        match kind {
            TimelineEventKind::PhaseChange => "\u{25C6}",  // diamond
            TimelineEventKind::ToolCall => "\u{2699}",     // gear
            TimelineEventKind::Approval => "\u{2714}",     // heavy check
            TimelineEventKind::Error => "\u{26A0}",        // warning
            TimelineEventKind::Info => "\u{2139}",         // info
            TimelineEventKind::Delegation => "\u{21C4}",   // left right arrows
        }
    }
}

impl Default for TimelinePanel {
    fn default() -> Self {
        Self::new()
    }
}
