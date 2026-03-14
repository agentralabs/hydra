pub mod state;
pub mod design {
    pub mod colors;
    pub mod theme;
    pub mod typography;

    pub use colors::DesignColors;
    pub use theme::DesignTheme;
    pub use typography::{Radius, Spacing, Typography};
}
pub mod utils;
pub mod profile;
pub mod operational_profile;
pub mod persistence;
pub mod proactive;
pub mod federation;
// Re-exports for convenience
pub use state::app::{AppMode, AppState};
pub use state::hydra::{AppConfig, HydraState};
pub use state::runs::RunTracker;
pub use state::sessions::{Session, SessionStatus, SessionStore};
pub use state::user::UserPreferences;
pub use state::settings::SettingsStore;
pub use utils::{detect_language, extract_json_plan, format_bytes, generate_deliverable_steps};
pub use utils::markdown::markdown_to_html;
pub use profile::{load_profile, save_profile, PersistedProfile, UserAutonomyLevel};
pub use operational_profile::OperationalProfile;
pub use proactive::{ProactiveAlert, AlertPriority, ProactiveNotifier};
pub use federation::FederationManager;
pub use design::{DesignColors, DesignTheme, Radius, Spacing, Typography};
