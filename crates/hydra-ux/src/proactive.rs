use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use hydra_core::error::HydraError;
use hydra_core::types::{IconState, ProactiveUpdate};

use crate::icon::IconStateMachine;

/// Configuration for the proactive update engine
#[derive(Debug, Clone)]
pub struct ProactiveConfig {
    /// Maximum time between updates before "still working" is sent
    pub max_silence: Duration,
    /// Progress update interval
    pub progress_interval: Duration,
    /// Minimum progress delta to report (0.0–1.0)
    pub min_progress_delta: f64,
    /// Broadcast channel capacity
    pub channel_capacity: usize,
    /// Maximum length for task names in formatted output
    pub max_task_name_length: usize,
}

impl Default for ProactiveConfig {
    fn default() -> Self {
        Self {
            max_silence: Duration::from_secs(5),
            progress_interval: Duration::from_secs(3),
            min_progress_delta: 0.05,
            channel_capacity: 256,
            max_task_name_length: 200,
        }
    }
}

/// Core engine for proactive updates — never let the user wait in silence
pub struct ProactiveEngine {
    update_tx: broadcast::Sender<ProactiveUpdate>,
    config: ProactiveConfig,
    last_update: Arc<Mutex<Instant>>,
    last_progress: Arc<Mutex<f64>>,
    icon: Arc<IconStateMachine>,
    running: Arc<AtomicBool>,
    updates_sent: Arc<AtomicU64>,
    /// Whether notification fallback is needed
    notifications_denied: Arc<AtomicBool>,
    /// Screen reader mode
    screen_reader_mode: Arc<AtomicBool>,
}

impl ProactiveEngine {
    pub fn new(config: ProactiveConfig) -> Self {
        let (tx, _) = broadcast::channel(config.channel_capacity);
        Self {
            update_tx: tx,
            config,
            last_update: Arc::new(Mutex::new(Instant::now())),
            last_progress: Arc::new(Mutex::new(0.0)),
            icon: Arc::new(IconStateMachine::new()),
            running: Arc::new(AtomicBool::new(false)),
            updates_sent: Arc::new(AtomicU64::new(0)),
            notifications_denied: Arc::new(AtomicBool::new(false)),
            screen_reader_mode: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Subscribe to receive proactive updates
    pub fn subscribe(&self) -> broadcast::Receiver<ProactiveUpdate> {
        self.update_tx.subscribe()
    }

    /// Send an acknowledgment — MUST be called within 100ms of user input
    pub fn send_acknowledgment(&self, message: impl Into<String>) {
        self.icon.transition(IconState::Listening);
        self.send(ProactiveUpdate::Acknowledgment {
            message: message.into(),
        });
    }

    /// Send a progress update — throttled by min_progress_delta
    pub fn send_progress(&self, percent: f64, message: impl Into<String>) {
        let mut last = self.last_progress.lock();
        let delta = (percent - *last).abs();
        if delta >= self.config.min_progress_delta * 100.0 || percent >= 100.0 {
            *last = percent;
            self.icon.transition(IconState::Working);
            self.send(ProactiveUpdate::Progress {
                percent,
                message: message.into(),
                deployment_id: None,
            });
        }
    }

    /// Send a completion update
    pub fn send_completion(&self, summary: hydra_core::types::CompletionSummary) {
        self.icon.transition(IconState::Success);
        self.send(ProactiveUpdate::Completion { summary });
        // Success is transient — schedule return to Idle
        let icon = self.icon.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(2000)).await;
            icon.transition(IconState::Idle);
        });
    }

    /// Send an alert
    pub fn send_alert(
        &self,
        level: hydra_core::types::AlertLevel,
        message: impl Into<String>,
        suggestion: Option<String>,
    ) {
        if matches!(level, hydra_core::types::AlertLevel::Error) {
            self.icon.transition(IconState::Error);
        } else {
            self.icon.transition(IconState::NeedsAttention);
        }
        self.send(ProactiveUpdate::Alert {
            level,
            message: message.into(),
            suggestion,
        });
    }

    /// Send an event
    pub fn send_event(&self, title: impl Into<String>, detail: impl Into<String>) {
        self.send(ProactiveUpdate::Event {
            title: title.into(),
            detail: detail.into(),
        });
    }

    /// Low-level send — updates timestamp, handles disconnected receivers gracefully
    pub fn send(&self, update: ProactiveUpdate) {
        *self.last_update.lock() = Instant::now();
        self.updates_sent.fetch_add(1, Ordering::Relaxed);
        // broadcast::send returns Err if no active receivers — that's fine (EC-UX-001)
        let _ = self.update_tx.send(update);
    }

    /// Check if silence has exceeded the max threshold
    pub fn check_silence(&self) -> bool {
        self.last_update.lock().elapsed() > self.config.max_silence
    }

    /// Start the silence watcher — sends "still working" if silent > max_silence
    pub fn start_silence_watcher(&self) -> JoinHandle<()> {
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let last_update = self.last_update.clone();
        let max_silence = self.config.max_silence;
        let tx = self.update_tx.clone();
        let icon = self.icon.clone();

        tokio::spawn(async move {
            let check_interval = Duration::from_secs(1);
            while running.load(Ordering::SeqCst) {
                tokio::time::sleep(check_interval).await;
                let elapsed = last_update.lock().elapsed();
                if elapsed > max_silence {
                    icon.transition(IconState::Working);
                    let _ = tx.send(ProactiveUpdate::Event {
                        title: "Still working".into(),
                        detail: format!("Working on it... ({} seconds elapsed)", elapsed.as_secs()),
                    });
                    // Update timestamp so we don't spam
                    *last_update.lock() = Instant::now();
                }
            }
        })
    }

    /// Stop the silence watcher
    pub fn stop_silence_watcher(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get a reference to the icon state machine
    pub fn icon(&self) -> &IconStateMachine {
        &self.icon
    }

    /// Total updates sent
    pub fn updates_sent(&self) -> u64 {
        self.updates_sent.load(Ordering::Relaxed)
    }

    /// Check if the engine is healthy (EC-UX-001)
    pub fn is_healthy(&self) -> bool {
        // Engine is healthy as long as the sender exists
        // Dropped receivers don't affect health
        true
    }

    /// Format a progress message, truncating long task names (EC-UX-007)
    pub fn format_progress(&self, task_name: &str, percent: f64) -> String {
        let truncated = if task_name.len() > self.config.max_task_name_length {
            format!("{}...", &task_name[..self.config.max_task_name_length])
        } else {
            task_name.to_string()
        };
        format!("[{:.0}%] {}", percent, truncated)
    }

    /// Format an update for screen reader mode (EC-UX-005)
    pub fn format_accessible(&self, update: &ProactiveUpdate) -> AccessibleUpdate {
        let description = match update {
            ProactiveUpdate::Acknowledgment { message } => {
                format!("Acknowledgment: {message}")
            }
            ProactiveUpdate::Progress {
                percent, message, ..
            } => {
                format!("Progress: {percent:.0} percent. {message}")
            }
            ProactiveUpdate::Event { title, detail } => {
                format!("Event: {title}. {detail}")
            }
            ProactiveUpdate::Decision { request } => {
                let options: Vec<_> = request
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, o)| format!("Option {}: {}", i + 1, o.label))
                    .collect();
                format!(
                    "Decision required: {}. {}",
                    request.question,
                    options.join(". ")
                )
            }
            ProactiveUpdate::Completion { summary } => {
                format!(
                    "Completed: {}. {} actions performed. {} next steps.",
                    summary.headline,
                    summary.actions.len(),
                    summary.next_steps.len()
                )
            }
            ProactiveUpdate::Alert {
                level,
                message,
                suggestion,
            } => {
                let sug = suggestion
                    .as_deref()
                    .map(|s| format!(" Suggestion: {s}"))
                    .unwrap_or_default();
                format!("{level:?} alert: {message}.{sug}")
            }
        };
        AccessibleUpdate {
            role: "status".to_string(),
            aria_live: match update {
                ProactiveUpdate::Alert { .. } | ProactiveUpdate::Decision { .. } => {
                    "assertive".to_string()
                }
                _ => "polite".to_string(),
            },
            description,
        }
    }

    /// Set notification permission denied (EC-UX-009)
    pub fn deny_notifications(&self) {
        self.notifications_denied.store(true, Ordering::SeqCst);
    }

    pub fn notifications_denied(&self) -> bool {
        self.notifications_denied.load(Ordering::SeqCst)
    }

    /// Enable screen reader mode (EC-UX-005)
    pub fn enable_screen_reader_mode(&self) {
        self.screen_reader_mode.store(true, Ordering::SeqCst);
    }

    pub fn is_screen_reader_mode(&self) -> bool {
        self.screen_reader_mode.load(Ordering::SeqCst)
    }

    /// Present a HydraError as a user-friendly alert
    pub fn present_error(&self, error: &HydraError) {
        self.send_alert(
            hydra_core::types::AlertLevel::Error,
            error.user_message(),
            error.suggested_action(),
        );
    }
}

/// Accessible update format for screen readers (EC-UX-005)
#[derive(Debug, Clone)]
pub struct AccessibleUpdate {
    pub role: String,
    pub aria_live: String,
    pub description: String,
}

impl AccessibleUpdate {
    pub fn is_accessible(&self) -> bool {
        !self.role.is_empty() && !self.aria_live.is_empty() && !self.description.is_empty()
    }
}

/// Update throttle for preventing floods (EC-UX-002)
pub struct UpdateThrottle {
    updates: Arc<Mutex<Vec<ProactiveUpdate>>>,
    max_pending: usize,
}

impl UpdateThrottle {
    pub fn new(max_pending: usize) -> Self {
        Self {
            updates: Arc::new(Mutex::new(Vec::new())),
            max_pending,
        }
    }

    /// Add an update, dropping old ones if over capacity
    pub fn push(&self, update: ProactiveUpdate) {
        let mut updates = self.updates.lock();
        if updates.len() >= self.max_pending {
            // Keep only the latest — drop older updates
            let drain_count = updates.len() - self.max_pending / 2;
            updates.drain(..drain_count);
        }
        updates.push(update);
    }

    /// Get all pending updates and clear the buffer
    pub fn drain(&self) -> Vec<ProactiveUpdate> {
        let mut updates = self.updates.lock();
        std::mem::take(&mut *updates)
    }

    pub fn pending_count(&self) -> usize {
        self.updates.lock().len()
    }
}
