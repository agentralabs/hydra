//! Companion — unified coordinator for signals and tasks.
//!
//! DUAL capability: signal stream (prioritized reality) + task executor
//! (background work). All actions are ALWAYS visible in the TUI stream.

use uuid::Uuid;

use crate::errors::CompanionError;
use crate::signal::{SignalBuffer, SignalClass, SignalClassifier, SignalItem, SignalRouting};
use crate::task::{AutonomyLevel, CompanionTask, TaskExecutor};

/// Parsed companion command from user input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionCommand {
    /// Review batched routine signals.
    Digest,
    /// Full list of all signals (surfaced + batched + archived).
    Inbox,
    /// Companion status: tasks running, signals today.
    Status,
    /// Pause all companion tasks (signals still collected).
    Pause,
    /// Resume paused companion tasks.
    Resume,
    /// Defer a notable signal to end-of-day batch.
    Later,
    /// Add a new signal source.
    SignalAdd {
        /// The source to add.
        source: String,
    },
    /// Mute a signal source temporarily.
    SignalMute {
        /// The source to mute.
        source: String,
    },
    /// Not a companion command.
    Unknown,
}

impl CompanionCommand {
    /// Parse a command string into a CompanionCommand.
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("/digest") {
            Self::Digest
        } else if trimmed.eq_ignore_ascii_case("/inbox") {
            Self::Inbox
        } else if trimmed.eq_ignore_ascii_case("/companion") {
            Self::Status
        } else if trimmed.eq_ignore_ascii_case("/pause") {
            Self::Pause
        } else if trimmed.eq_ignore_ascii_case("/resume") {
            Self::Resume
        } else if trimmed.eq_ignore_ascii_case("/later") {
            Self::Later
        } else if let Some(rest) = trimmed.strip_prefix("/signal add ") {
            Self::SignalAdd {
                source: rest.trim().to_string(),
            }
        } else if let Some(rest) = trimmed.strip_prefix("/signal mute ") {
            Self::SignalMute {
                source: rest.trim().to_string(),
            }
        } else {
            Self::Unknown
        }
    }
}

/// Result from routing a signal — tells the TUI what to do.
#[derive(Debug, Clone)]
pub struct RoutedSignal {
    /// The signal that was routed.
    pub signal_id: Uuid,
    /// How it should be presented.
    pub routing: SignalRouting,
    /// The signal class.
    pub class: SignalClass,
    /// Summary content for display.
    pub content: String,
    /// Source of the signal.
    pub source: String,
}

/// The companion coordinator — manages signal classification and task execution.
#[derive(Debug, Clone)]
pub struct Companion {
    /// Signal buffer.
    signals: SignalBuffer,
    /// Signal classifier.
    classifier: SignalClassifier,
    /// Task executor.
    executor: TaskExecutor,
    /// Whether companion tasks are paused.
    paused: bool,
}

impl Companion {
    /// Create a new companion.
    pub fn new() -> Self {
        Self {
            signals: SignalBuffer::new(),
            classifier: SignalClassifier::new(),
            executor: TaskExecutor::new(),
            paused: false,
        }
    }

    /// Receive, classify, and route a signal. Returns routing information.
    pub fn receive_signal(&mut self, mut signal: SignalItem) -> RoutedSignal {
        self.classifier.classify(&mut signal);
        let routing = signal.class.routing();
        let routed = RoutedSignal {
            signal_id: signal.id,
            routing,
            class: signal.class,
            content: signal.content.clone(),
            source: signal.source.clone(),
        };
        self.signals.push(signal);
        routed
    }

    /// Submit a new task with default autonomy. Returns the task ID.
    pub fn submit_task(&mut self, description: String) -> Result<Uuid, CompanionError> {
        self.executor.submit(description)
    }

    /// Submit a new task with specific autonomy level.
    pub fn submit_task_with_autonomy(
        &mut self,
        description: String,
        autonomy: AutonomyLevel,
    ) -> Result<Uuid, CompanionError> {
        self.executor.submit_with_autonomy(description, autonomy)
    }

    /// Start a task by ID.
    pub fn start_task(&mut self, task_id: Uuid) -> Result<(), CompanionError> {
        self.executor.start_task(task_id)
    }

    /// Complete a task by ID.
    pub fn complete_task(&mut self, task_id: Uuid) -> Result<(), CompanionError> {
        self.executor.complete_task(task_id)
    }

    /// Fail a task by ID with a reason.
    pub fn fail_task(&mut self, task_id: Uuid, reason: String) -> Result<(), CompanionError> {
        self.executor.fail_task(task_id, reason)
    }

    /// Cancel a task by ID.
    pub fn cancel_task(&mut self, task_id: Uuid) -> Result<(), CompanionError> {
        self.executor.cancel_task(task_id)
    }

    /// Block a task — needs user input.
    pub fn block_task(&mut self, task_id: Uuid, reason: String) -> Result<(), CompanionError> {
        self.executor.block_task(task_id, reason)
    }

    /// Unblock a blocked task.
    pub fn unblock_task(&mut self, task_id: Uuid) -> Result<(), CompanionError> {
        self.executor.unblock_task(task_id)
    }

    /// Process /digest — return routine signals and mark them surfaced.
    pub fn digest(&mut self) -> Vec<RoutedSignal> {
        let items: Vec<RoutedSignal> = self
            .signals
            .digest_items()
            .iter()
            .map(|s| RoutedSignal {
                signal_id: s.id,
                routing: SignalRouting::BatchForDigest,
                class: s.class,
                content: s.content.clone(),
                source: s.source.clone(),
            })
            .collect();
        self.signals.mark_digest_surfaced();
        items
    }

    /// Process /inbox — return all signals.
    pub fn inbox(&self) -> Vec<RoutedSignal> {
        self.signals
            .inbox()
            .iter()
            .map(|s| RoutedSignal {
                signal_id: s.id,
                routing: s.class.routing(),
                class: s.class,
                content: s.content.clone(),
                source: s.source.clone(),
            })
            .collect()
    }

    /// Return pending urgent signals that need immediate interruption.
    pub fn pending_urgent(&self) -> Vec<RoutedSignal> {
        self.signals
            .pending_urgent()
            .iter()
            .map(|s| RoutedSignal {
                signal_id: s.id,
                routing: SignalRouting::InterruptNow,
                class: s.class,
                content: s.content.clone(),
                source: s.source.clone(),
            })
            .collect()
    }

    /// Return pending notable signals for next-pause surfacing.
    pub fn pending_notable(&self) -> Vec<RoutedSignal> {
        self.signals
            .pending_notable()
            .iter()
            .map(|s| RoutedSignal {
                signal_id: s.id,
                routing: SignalRouting::NextPause,
                class: s.class,
                content: s.content.clone(),
                source: s.source.clone(),
            })
            .collect()
    }

    /// Mark a signal as surfaced.
    pub fn mark_surfaced(&mut self, signal_id: Uuid) {
        self.signals.mark_surfaced(signal_id);
    }

    /// Pause all companion tasks.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume companion tasks.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Return whether the companion is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Return a reference to the signal buffer.
    pub fn signals(&self) -> &SignalBuffer {
        &self.signals
    }

    /// Return a reference to the task executor.
    pub fn executor(&self) -> &TaskExecutor {
        &self.executor
    }

    /// Return the number of active tasks.
    pub fn active_task_count(&self) -> usize {
        self.executor.active_count()
    }

    /// Return all tasks.
    pub fn tasks(&self) -> &[CompanionTask] {
        self.executor.tasks()
    }

    /// Return a task by ID.
    pub fn get_task(&self, task_id: Uuid) -> Option<&CompanionTask> {
        self.executor.get_task(task_id)
    }

    /// Return a mutable reference to the classifier for customization.
    pub fn classifier_mut(&mut self) -> &mut SignalClassifier {
        &mut self.classifier
    }
}

impl Default for Companion {
    fn default() -> Self {
        Self::new()
    }
}
