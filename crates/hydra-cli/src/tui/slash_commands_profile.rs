//! Slash commands — /profile (Operational Profiles).
use super::app::{App, Message, MessageRole};
use hydra_native::operational_profile;

impl App {
    pub(crate) fn slash_cmd_profile(&mut self, args: &str, timestamp: &str) {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts.first().copied().unwrap_or("").trim();

        match sub {
            "" | "show" => self.profile_show(timestamp),
            "list" => self.profile_list(timestamp),
            "load" => {
                let name = parts.get(1).copied().unwrap_or("").trim();
                if name.is_empty() {
                    self.push_profile_msg("Usage: /profile load <name>", timestamp);
                } else {
                    self.profile_load(name, timestamp);
                }
            }
            "create" => {
                let name = parts.get(1).copied().unwrap_or("").trim();
                if name.is_empty() {
                    self.push_profile_msg("Usage: /profile create <name>", timestamp);
                } else {
                    self.profile_create(name, timestamp);
                }
            }
            "unload" => self.profile_unload(timestamp),
            "export" => {
                let name = parts.get(1).copied().unwrap_or("").trim();
                if name.is_empty() {
                    self.push_profile_msg("Usage: /profile export <name>", timestamp);
                } else {
                    self.profile_export(name, timestamp);
                }
            }
            "info" => {
                let name = parts.get(1).copied().unwrap_or("").trim();
                if name.is_empty() {
                    self.push_profile_msg("Usage: /profile info <name>", timestamp);
                } else {
                    self.profile_info(name, timestamp);
                }
            }
            "validate" => {
                let name = parts.get(1).copied().unwrap_or("").trim();
                self.profile_validate(name, timestamp);
            }
            "update" => {
                let name = parts.get(1).copied().unwrap_or("").trim();
                self.profile_update(name, timestamp);
            }
            "beliefs" => {
                let name = parts.get(1).copied().unwrap_or("").trim();
                self.profile_beliefs(name, timestamp);
            }
            "skills" => {
                let name = parts.get(1).copied().unwrap_or("").trim();
                self.profile_skills(name, timestamp);
            }
            _ => {
                self.push_profile_msg(
                    "Unknown subcommand. Available: show, list, load, create, unload, export, info, validate, update, beliefs, skills",
                    timestamp,
                );
            }
        }
    }

    fn profile_show(&mut self, timestamp: &str) {
        if let Some(ref profile) = self.active_profile {
            let summary = operational_profile::profile_summary(profile);
            self.push_profile_msg(&format!("Active Profile\n\n{}", summary), timestamp);
        } else {
            self.push_profile_msg(
                "No operational profile active.\n\nUse /profile list to see available profiles.\nUse /profile load <name> to activate one.",
                timestamp,
            );
        }
    }

    fn profile_list(&mut self, timestamp: &str) {
        let profiles = operational_profile::list_profiles();
        if profiles.is_empty() {
            let dir = operational_profile::profiles_dir()
                .map(|d| d.display().to_string())
                .unwrap_or_else(|| "~/.hydra/profiles/".into());
            self.push_profile_msg(
                &format!(
                    "No profiles found.\n\nCreate one with /profile create <name>\nProfiles directory: {}",
                    dir
                ),
                timestamp,
            );
        } else {
            use hydra_native::cognitive::profile_loader;
            let active = self.active_profile.as_ref().map(|p| p.name.as_str());
            let mut lines = vec!["Available Profiles\n".to_string()];
            for name in &profiles {
                let marker = if active == Some(name.as_str()) { " (active)" } else { "" };
                // Load profile to get belief/skill counts
                let counts = profile_loader::load_profile(name)
                    .map(|p| format!(" — {} beliefs, {} skills", p.beliefs.len(), p.skills.len()))
                    .unwrap_or_default();
                lines.push(format!("  {}{}{}", name, marker, counts));
            }
            self.push_profile_msg(&lines.join("\n"), timestamp);
        }
    }

    fn profile_load(&mut self, name: &str, timestamp: &str) {
        use hydra_native::cognitive::profile_loader;
        use hydra_native::cognitive::profile_applier;

        match profile_loader::load_profile(name) {
            Ok(profile) => {
                // Validate first
                let warnings = profile_loader::validate_profile(&profile);

                // Build prompt overlay (synchronous)
                let overlay = profile_applier::build_prompt_overlay(&profile);

                // Apply permissions synchronously
                // (beliefs/goals require async sister calls — done on next cognitive loop)
                if let Some(ref perms) = profile.permissions {
                    // Note: we can't mutate RuntimeSettings here directly since they're
                    // built fresh each loop. The overlay + model are the persistent parts.
                    if let Some(fw) = perms.file_write {
                        if !fw {
                            eprintln!("[hydra:profile] file_write disabled by profile");
                        }
                    }
                }

                // Apply model
                if let Some(ref model) = profile.model {
                    std::env::set_var("HYDRA_MODEL", &model.default);
                    let (m, p) = super::app_helpers::resolve_model_and_provider();
                    self.model_name = m;
                    self.provider_name = p;
                }

                // Persist active profile name
                if let Some(mut persisted) = hydra_native::profile::load_profile() {
                    persisted.active_operational_profile = Some(name.to_string());
                    hydra_native::profile::save_profile(&persisted);
                }

                let summary = operational_profile::profile_summary(&profile);
                self.profile_prompt_overlay = overlay;
                self.active_profile = Some(profile);

                let mut msg = format!("Profile '{}' loaded\n\n{}", name, summary);
                if !warnings.is_empty() {
                    msg.push_str("\n\nWarnings:");
                    for w in &warnings {
                        msg.push_str(&format!("\n  {}", w));
                    }
                }
                self.push_profile_msg(&msg, timestamp);
            }
            Err(e) => {
                self.push_profile_msg(&format!("Failed to load profile: {}", e), timestamp);
            }
        }
    }

    fn profile_create(&mut self, name: &str, timestamp: &str) {
        match operational_profile::scaffold_profile(name) {
            Ok(path) => {
                self.push_profile_msg(
                    &format!(
                        "Profile '{}' created at {}\n\nEdit the TOML files to customize, then /profile load {}",
                        name, path.display(), name
                    ),
                    timestamp,
                );
            }
            Err(e) => {
                self.push_profile_msg(&format!("Failed to create profile: {}", e), timestamp);
            }
        }
    }

    fn profile_unload(&mut self, timestamp: &str) {
        if self.active_profile.is_none() {
            self.push_profile_msg("No profile is active.", timestamp);
            return;
        }

        let name = self.active_profile.as_ref().map(|p| p.name.clone()).unwrap_or_default();
        self.active_profile = None;
        self.profile_prompt_overlay = None;

        // Clear from persisted profile
        if let Some(mut persisted) = hydra_native::profile::load_profile() {
            persisted.active_operational_profile = None;
            hydra_native::profile::save_profile(&persisted);
        }

        self.push_profile_msg(&format!("Profile '{}' unloaded. Using defaults.", name), timestamp);
    }

    fn profile_export(&mut self, name: &str, timestamp: &str) {
        let dir = match operational_profile::profiles_dir() {
            Some(d) => d.join(name),
            None => {
                self.push_profile_msg("Cannot determine profiles directory.", timestamp);
                return;
            }
        };

        if dir.exists() {
            self.push_profile_msg(&format!("Profile '{}' already exists. Choose a different name.", name), timestamp);
            return;
        }

        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.push_profile_msg(&format!("Failed to create directory: {}", e), timestamp);
            return;
        }

        // Export current model
        let model = std::env::var("HYDRA_MODEL").unwrap_or_else(|_| "claude-sonnet-4-6".into());
        let model_toml = format!("[model]\ndefault = \"{}\"\n", model);
        let _ = std::fs::write(dir.join("model.toml"), model_toml);

        // Export identity stub
        let identity = format!(
            "[identity]\npersona = \"Custom profile exported from Hydra.\"\ntone = \"balanced\"\nconstraints = []\n"
        );
        let _ = std::fs::write(dir.join("identity.toml"), identity);

        // Export permissions from current env
        let perms = format!(
            "[permissions]\nfile_write = {}\nshell_exec = {}\nnetwork_access = true\nrisk_threshold = \"medium\"\n",
            std::env::var("HYDRA_FILE_WRITE").map(|v| v != "0" && v != "false").unwrap_or(true),
            std::env::var("HYDRA_SHELL_EXEC").map(|v| v != "0" && v != "false").unwrap_or(true),
        );
        let _ = std::fs::write(dir.join("permissions.toml"), perms);

        self.push_profile_msg(
            &format!("Profile '{}' exported to {}\nEdit the files and /profile load {}", name, dir.display(), name),
            timestamp,
        );
    }

    fn profile_info(&mut self, name: &str, timestamp: &str) {
        use hydra_native::cognitive::profile_loader;
        match profile_loader::load_profile(name) {
            Ok(profile) => {
                let summary = operational_profile::profile_summary(&profile);
                let warnings = profile_loader::validate_profile(&profile);
                let mut msg = format!("Profile Info: {}\n\n{}", name, summary);
                if !warnings.is_empty() {
                    msg.push_str("\n\nWarnings:");
                    for w in &warnings {
                        msg.push_str(&format!("\n  {}", w));
                    }
                }
                self.push_profile_msg(&msg, timestamp);
            }
            Err(e) => self.push_profile_msg(&format!("Cannot load '{}': {}", name, e), timestamp),
        }
    }

    fn profile_validate(&mut self, name: &str, timestamp: &str) {
        use hydra_native::cognitive::profile_loader;
        let target = if name.is_empty() {
            match &self.active_profile {
                Some(p) => p.clone(),
                None => {
                    self.push_profile_msg("No active profile. Usage: /profile validate <name>", timestamp);
                    return;
                }
            }
        } else {
            match profile_loader::load_profile(name) {
                Ok(p) => p,
                Err(e) => {
                    self.push_profile_msg(&format!("Cannot load '{}': {}", name, e), timestamp);
                    return;
                }
            }
        };
        let warnings = profile_loader::validate_profile(&target);
        if warnings.is_empty() {
            self.push_profile_msg(&format!("Profile '{}' is valid.", target.name), timestamp);
        } else {
            let mut msg = format!("Profile '{}' — {} warnings:", target.name, warnings.len());
            for w in &warnings { msg.push_str(&format!("\n  {}", w)); }
            self.push_profile_msg(&msg, timestamp);
        }
    }

    fn profile_update(&mut self, name: &str, timestamp: &str) {
        use hydra_native::cognitive::profile_updater;
        let factory = match profile_updater::factory_profiles_dir() {
            Some(f) => f,
            None => {
                self.push_profile_msg("Cannot find factory profiles source directory.", timestamp);
                return;
            }
        };
        let user_base = match operational_profile::profiles_dir() {
            Some(d) => d,
            None => {
                self.push_profile_msg("Cannot determine profiles directory.", timestamp);
                return;
            }
        };
        let names: Vec<String> = if name.is_empty() {
            profile_updater::list_factory_profiles(&factory)
        } else {
            vec![name.to_string()]
        };
        if names.is_empty() {
            self.push_profile_msg("No factory profiles found to update.", timestamp);
            return;
        }
        let mut results = Vec::new();
        for n in &names {
            let src = factory.join(n);
            let dst = user_base.join(n);
            let report = profile_updater::update_profile(&src, &dst);
            results.push(report.summary());
        }
        self.push_profile_msg(&results.join("\n\n"), timestamp);
    }

    fn profile_beliefs(&mut self, name: &str, timestamp: &str) {
        use hydra_native::cognitive::profile_loader;
        let target = if name.is_empty() {
            match &self.active_profile {
                Some(p) => p.clone(),
                None => {
                    self.push_profile_msg("No active profile. Usage: /profile beliefs <name>", timestamp);
                    return;
                }
            }
        } else {
            match profile_loader::load_profile(name) {
                Ok(p) => p,
                Err(e) => {
                    self.push_profile_msg(&format!("Cannot load '{}': {}", name, e), timestamp);
                    return;
                }
            }
        };
        if target.beliefs.is_empty() {
            self.push_profile_msg(&format!("Profile '{}' has no beliefs.", target.name), timestamp);
        } else {
            let mut msg = format!("Profile '{}' — {} beliefs:\n", target.name, target.beliefs.len());
            for b in &target.beliefs {
                msg.push_str(&format!("\n  [{}] {} (confidence: {:.0}%)", b.topic, b.content, b.confidence * 100.0));
            }
            self.push_profile_msg(&msg, timestamp);
        }
    }

    fn profile_skills(&mut self, name: &str, timestamp: &str) {
        use hydra_native::cognitive::profile_loader;
        let target = if name.is_empty() {
            match &self.active_profile {
                Some(p) => p.clone(),
                None => {
                    self.push_profile_msg("No active profile. Usage: /profile skills <name>", timestamp);
                    return;
                }
            }
        } else {
            match profile_loader::load_profile(name) {
                Ok(p) => p,
                Err(e) => {
                    self.push_profile_msg(&format!("Cannot load '{}': {}", name, e), timestamp);
                    return;
                }
            }
        };
        if target.skills.is_empty() {
            self.push_profile_msg(&format!("Profile '{}' has no skills.", target.name), timestamp);
        } else {
            let mut msg = format!("Profile '{}' — {} skills:\n", target.name, target.skills.len());
            for s in &target.skills {
                msg.push_str(&format!("\n  {} — {}", s.name, s.description));
            }
            self.push_profile_msg(&msg, timestamp);
        }
    }

    fn push_profile_msg(&mut self, content: &str, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: content.to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }
}
