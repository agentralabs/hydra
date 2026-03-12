// NOTE: This is a LIBRARY crate. The desktop binary is at crates/hydra-desktop/.
// Run desktop with: cargo run --bin hydra-desktop

// === Re-export sub-crates at original module paths ===
// This keeps all downstream consumers (hydra-desktop, hydra-cli, hydra-server)
// working with `use hydra_native::*` unchanged.

// From hydra-native-state (foundation types, state, utils, design)
pub use hydra_native_state::state;
pub use hydra_native_state::design;
pub use hydra_native_state::utils;
pub use hydra_native_state::profile;
pub use hydra_native_state::persistence;
pub use hydra_native_state::proactive;
pub use hydra_native_state::federation;

// From hydra-native-cognitive (cognitive loop, sisters, task persistence, remote)
pub use hydra_native_cognitive::cognitive;
pub use hydra_native_cognitive::remote;
pub use hydra_native_cognitive::sisters;
pub use hydra_native_cognitive::sister_improve;
pub use hydra_native_cognitive::swarm;
pub use hydra_native_cognitive::threat;
pub use hydra_native_cognitive::task_persistence;

// Local modules (remain in this crate)
pub mod app;
pub mod audio;
pub mod codebase;
pub mod commands;
pub mod components;
pub mod modes;
pub mod shortcuts;
pub mod update;
pub mod styles;

// === Public re-exports (preserve existing API surface) ===

// App
pub use app::{AppSection, AppViewModel, WindowConfig};
pub use commands::hydra::{CommandResult, HydraCommands};
pub use modes::{CompanionMode, ImmersiveMode, InvisibleMode, WorkspaceMode};
pub use shortcuts::ShortcutRegistry;
pub use update::UpdateManager;

// State (from hydra-native-state)
pub use hydra_native_state::state::app::{AppMode, AppState};
pub use hydra_native_state::state::hydra::{AppConfig, HydraState};
pub use hydra_native_state::state::runs::RunTracker;
pub use hydra_native_state::state::sessions::{Session, SessionStatus, SessionStore};
pub use hydra_native_state::state::user::UserPreferences;
pub use hydra_native_state::state::settings::SettingsStore;

// Utils (from hydra-native-state)
pub use hydra_native_state::utils::{detect_language, extract_json_plan, format_bytes, generate_deliverable_steps};
pub use hydra_native_state::utils::markdown::markdown_to_html;

// Design (from hydra-native-state)
pub use hydra_native_state::design::{DesignColors, DesignTheme, Radius, Spacing, Typography};

// Profile (from hydra-native-state)
pub use hydra_native_state::profile::{load_profile, save_profile, PersistedProfile, UserAutonomyLevel};

// Proactive (from hydra-native-state)
pub use hydra_native_state::proactive::{ProactiveAlert, AlertPriority, ProactiveNotifier};

// Federation (from hydra-native-state)
pub use hydra_native_state::federation::FederationManager;

// Cognitive (from hydra-native-cognitive)
pub use hydra_native_cognitive::cognitive::{AgentSpawner, CognitiveLoopConfig, CognitiveUpdate, DecideEngine, DecideResult, InventionEngine, run_cognitive_loop};
pub use hydra_native_cognitive::cognitive::streaming::{StreamBuffer, StreamingConfig, StreamState};

// Sisters (from hydra-native-cognitive)
pub use hydra_native_cognitive::sisters::{init_sisters, Sisters, SistersHandle};

// Components (local)
pub use components::command_palette::CommandPalette;
pub use components::diff_viewer::FileDiff;
pub use components::receipts::ReceiptAuditView;
pub use components::federation::FederationPanel;
pub use components::search::SearchOverlay;
pub use components::skills::SkillBrowser;
pub use components::topbar::{TopBarState, TopBarAction, PhaseDot};
pub use components::workspace::{PanelLayout, PanelConfig, PanelType, EvidenceTab};
pub use components::sisters_dashboard::{SistersDashboard, SisterInfo, SisterStatus};
pub use components::tabbed_settings::{TabbedSettings, SettingsTab};
pub use components::behavior::BehaviorSettings;
pub use components::notifications_settings::NotificationSettings;
pub use components::drag_drop::{DropZoneState, DroppedFile, FileType};
pub use components::ghost_cursor::{GhostCursorState, CursorMode, CursorAction, CursorSession, CursorVisualState, OsAutomation, cursor_svg, interpolate_arc};
pub use components::undo_wiring::{UndoToast, UndoWiring};
pub use components::onboarding::OnboardingFlow;
pub use components::globe::{GlobeRenderParams, GlobeSize, derive_globe_state, globe_params, globe_svg};
