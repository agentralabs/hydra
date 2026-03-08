#[cfg(feature = "desktop")]
pub mod desktop;
pub mod app;
pub mod audio;
pub mod cognitive;
pub mod commands;
pub mod components;
/// Design system: colors, typography, themes.
pub mod design {
    pub mod colors;
    pub mod theme;
    pub mod typography;

    pub use colors::DesignColors;
    pub use theme::DesignTheme;
    pub use typography::{Radius, Spacing, Typography};
}
pub mod modes;
pub mod profile;
pub mod shortcuts;
pub mod sisters;
pub mod update;
pub mod state;
pub mod styles;
pub mod utils;

pub use app::{AppSection, AppViewModel, WindowConfig};
pub use commands::hydra::{CommandResult, HydraCommands};
pub use design::{DesignColors, DesignTheme, Radius, Spacing, Typography};
pub use modes::{CompanionMode, ImmersiveMode, InvisibleMode, WorkspaceMode};
pub use profile::{load_profile, save_profile, PersistedProfile};
pub use sisters::{init_sisters, Sisters, SistersHandle};
pub use state::app::{AppMode, AppState};
pub use state::hydra::{AppConfig, HydraState};
pub use state::runs::RunTracker;
pub use state::sessions::{Session, SessionStatus, SessionStore};
pub use state::user::UserPreferences;
pub use utils::{detect_language, extract_json_plan, format_bytes, generate_deliverable_steps};
pub use cognitive::{CognitiveLoopConfig, CognitiveUpdate, run_cognitive_loop};
pub use components::command_palette::CommandPalette;
pub use components::diff_viewer::FileDiff;
pub use components::receipts::ReceiptAuditView;
pub use shortcuts::ShortcutRegistry;
pub use components::federation::FederationPanel;
pub use components::search::SearchOverlay;
pub use components::skills::SkillBrowser;
pub use update::UpdateManager;
pub use components::topbar::{TopBarState, TopBarAction, PhaseDot};
pub use components::workspace::{PanelLayout, PanelConfig, PanelType, EvidenceTab};
pub use components::sisters_dashboard::{SistersDashboard, SisterInfo, SisterStatus};
pub use components::tabbed_settings::{TabbedSettings, SettingsTab};
pub use components::behavior::BehaviorSettings;
pub use components::notifications_settings::NotificationSettings;
pub use cognitive::streaming::{StreamBuffer, StreamingConfig, StreamState};
pub use state::settings::SettingsStore;
pub use utils::markdown::markdown_to_html;
pub use components::drag_drop::{DropZoneState, DroppedFile, FileType};
pub use components::undo_wiring::{UndoToast, UndoWiring};
pub use components::onboarding::OnboardingFlow;
pub use components::globe::{GlobeRenderParams, GlobeSize, derive_globe_state, globe_params, globe_svg};
