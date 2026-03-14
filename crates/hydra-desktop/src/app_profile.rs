//! Desktop profile management — seeding, loading, slash command handling.
//! Bridges the profile system (in hydra-native-cognitive) to the Desktop surface.

use hydra_native::operational_profile::{self, OperationalProfile};
use hydra_native::cognitive::profile_loader;
use hydra_native::cognitive::profile_applier;

/// Seed factory profiles to ~/.hydra/profiles/ if needed.
pub fn seed_profiles_if_needed() {
    if let Some(profiles_dir) = operational_profile::profiles_dir() {
        if !profiles_dir.exists() || operational_profile::list_profiles().len() < 3 {
            if let Some(factory) = hydra_native::cognitive::profile_updater::factory_profiles_dir() {
                match operational_profile::seed_profiles(&factory) {
                    Ok(count) => eprintln!("[hydra-desktop] Seeded {} factory profiles", count),
                    Err(e) => eprintln!("[hydra-desktop] Profile seeding failed: {}", e),
                }
            } else {
                for name in &["dev", "devops", "writer"] {
                    let _ = operational_profile::scaffold_profile(name);
                }
            }
        }
    }
}

/// Load a profile by name, returning (profile, overlay_string).
pub fn load_profile_by_name(name: &str) -> Result<(OperationalProfile, Option<String>), String> {
    let profile = profile_loader::load_profile(name)?;
    let overlay = profile_applier::build_prompt_overlay(&profile);
    Ok((profile, overlay))
}

/// Auto-load the persisted active profile (if any).
pub fn auto_load_profile() -> Option<(OperationalProfile, Option<String>)> {
    let active_name = operational_profile::active_profile_name()?;
    load_profile_by_name(&active_name).ok()
}

/// Handle a slash command from Desktop input. Returns Some(response) if handled,
/// None if the command should be passed to the cognitive loop.
pub fn handle_slash_command(text: &str, active_profile: &Option<OperationalProfile>) -> Option<String> {
    let text = text.trim();
    if !text.starts_with('/') {
        return None;
    }

    let parts: Vec<&str> = text.splitn(3, ' ').collect();
    let cmd = parts[0];
    let sub = parts.get(1).copied().unwrap_or("");
    let arg = parts.get(2).copied().unwrap_or("");

    match cmd {
        "/profile" => Some(handle_profile_command(sub, arg, active_profile)),
        "/roi" => Some(hydra_native::knowledge::economics_tracker::roi_summary()),
        "/knowledge" => {
            let mentor = hydra_native::knowledge::mentor_system::mentor_state();
            let summary = mentor.lock().map(|s| s.progress_summary())
                .unwrap_or_else(|_| "Knowledge tracker unavailable.".into());
            Some(format!("Knowledge Progress\n\n{}", summary))
        }
        _ => None, // Not a profile/knowledge command — pass to cognitive loop
    }
}

fn handle_profile_command(sub: &str, arg: &str, active: &Option<OperationalProfile>) -> String {
    match sub {
        "" | "show" => {
            if let Some(ref p) = active {
                operational_profile::profile_summary(p)
            } else {
                "No profile active. Use /profile load <name>".into()
            }
        }
        "list" => {
            let profiles = operational_profile::list_profiles();
            if profiles.is_empty() {
                return "No profiles found.".into();
            }
            let active_name = active.as_ref().map(|p| p.name.as_str());
            let mut lines = vec!["Available Profiles\n".to_string()];
            for name in &profiles {
                let marker = if active_name == Some(name.as_str()) { " (active)" } else { "" };
                let counts = profile_loader::load_profile(name)
                    .map(|p| format!(" — {} beliefs, {} skills", p.beliefs.len(), p.skills.len()))
                    .unwrap_or_default();
                lines.push(format!("  {}{}{}", name, marker, counts));
            }
            lines.join("\n")
        }
        "beliefs" => {
            if let Some(ref p) = active {
                if p.beliefs.is_empty() {
                    format!("Profile '{}' has no beliefs.", p.name)
                } else {
                    let mut msg = format!("Profile '{}' — {} beliefs:\n", p.name, p.beliefs.len());
                    for b in &p.beliefs {
                        msg.push_str(&format!("\n  [{}] {} ({:.0}%)", b.topic, b.content, b.confidence * 100.0));
                    }
                    msg
                }
            } else {
                "No profile active.".into()
            }
        }
        "skills" => {
            if let Some(ref p) = active {
                if p.skills.is_empty() {
                    format!("Profile '{}' has no skills.", p.name)
                } else {
                    let mut msg = format!("Profile '{}' — {} skills:\n", p.name, p.skills.len());
                    for s in &p.skills { msg.push_str(&format!("\n  {} — {}", s.name, s.description)); }
                    msg
                }
            } else {
                "No profile active.".into()
            }
        }
        "info" => {
            if arg.is_empty() { return "Usage: /profile info <name>".into(); }
            match profile_loader::load_profile(arg) {
                Ok(p) => operational_profile::profile_summary(&p),
                Err(e) => format!("Cannot load '{}': {}", arg, e),
            }
        }
        "unload" => "Profile unloaded. Restart to clear.".into(),
        _ => format!("Unknown subcommand '{}'. Available: show, list, load, unload, beliefs, skills, info", sub),
    }
}
