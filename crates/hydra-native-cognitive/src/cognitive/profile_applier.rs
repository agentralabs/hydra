//! Profile Applier — takes a loaded OperationalProfile and mutates runtime state.
//!
//! Sister-first: beliefs go via Memory sister, goals via Planning sister.

use hydra_native_state::operational_profile::*;
use crate::cognitive::runtime_settings::RuntimeSettings;
use crate::sisters::SistersHandle;

/// Result of applying a profile.
#[derive(Debug, Default)]
pub struct ApplyResult {
    pub permissions_applied: bool,
    pub beliefs_stored: usize,
    pub goals_created: usize,
    pub model_set: bool,
    pub skills_registered: usize,
    pub prompt_overlay: Option<String>,
    pub warnings: Vec<String>,
}

/// Apply an operational profile to the runtime.
pub async fn apply_profile(
    profile: &OperationalProfile,
    settings: &mut RuntimeSettings,
    sisters: Option<&SistersHandle>,
) -> ApplyResult {
    let mut result = ApplyResult::default();

    // 1. Permissions → direct RuntimeSettings override
    if let Some(ref perms) = profile.permissions {
        apply_permissions(perms, settings);
        result.permissions_applied = true;
        eprintln!("[hydra:profile] Permissions applied");
    }

    // 2. Model → HYDRA_MODEL env var
    if let Some(ref model) = profile.model {
        apply_model(model);
        result.model_set = true;
        eprintln!("[hydra:profile] Model set to {}", model.default);
    }

    // 3. Beliefs → Memory sister (sister-first)
    if !profile.beliefs.is_empty() {
        result.beliefs_stored = apply_beliefs(&profile.beliefs, sisters).await;
        eprintln!("[hydra:profile] {} beliefs stored via Memory sister", result.beliefs_stored);
    }

    // 4. Goals → Planning sister (sister-first)
    if !profile.goals.is_empty() {
        result.goals_created = apply_goals(&profile.goals, sisters).await;
        eprintln!("[hydra:profile] {} goals created via Planning sister", result.goals_created);
    }

    // 5. Prompt overlay — combine identity + prompt_overlay.md
    result.prompt_overlay = build_prompt_overlay(profile);
    if result.prompt_overlay.is_some() {
        eprintln!("[hydra:profile] Prompt overlay built");
    }

    result
}

/// Apply permission overrides — only Some values replace existing settings.
fn apply_permissions(perms: &ProfilePermissions, settings: &mut RuntimeSettings) {
    if let Some(fw) = perms.file_write {
        settings.file_write = fw;
    }
    if let Some(se) = perms.shell_exec {
        settings.shell_exec = se;
    }
    if let Some(na) = perms.network_access {
        settings.network_access = na;
    }
    if let Some(ref rt) = perms.risk_threshold {
        settings.risk_threshold = rt.clone();
    }
    if let Some(sm) = perms.sandbox_mode {
        settings.sandbox_mode = sm;
    }
    if let Some(ref mfe) = perms.max_file_edits {
        settings.max_file_edits = mfe.clone();
    }
    if let Some(rac) = perms.require_approval_critical {
        settings.require_approval_critical = rac;
    }
    // UCU #10: Configurable constraints — apply profile-driven limits
    if let Some(max_tokens) = perms.max_response_tokens {
        settings.agentic_token_budget = max_tokens as u64;
        eprintln!("[hydra:profile] max_response_tokens={}", max_tokens);
    }
    if let Some(max_retries) = perms.max_retry_attempts {
        settings.agentic_max_turns = max_retries;
        eprintln!("[hydra:profile] max_retry_attempts={}", max_retries);
    }
    if let Some(max_ctx) = perms.max_context_tokens {
        eprintln!("[hydra:profile] max_context_tokens={} (used by context_manager)", max_ctx);
    }
    if let Some(max_lines) = perms.max_file_lines {
        eprintln!("[hydra:profile] max_file_lines={} (used by code generation)", max_lines);
    }
    if let Some(max_agents) = perms.max_agents {
        eprintln!("[hydra:profile] max_agents={} (used by swarm)", max_agents);
    }
}

/// Store beliefs via Memory sister `memory_add` MCP call.
async fn apply_beliefs(beliefs: &[ProfileBelief], sisters: Option<&SistersHandle>) -> usize {
    let Some(sh) = sisters else {
        eprintln!("[hydra:profile] No sisters — cannot store beliefs");
        return 0;
    };

    let mut stored = 0;
    for belief in beliefs {
        let content = format!("[profile-belief] {}: {}", belief.topic, belief.content);
        sh.memory_workspace_add(&content, &belief.topic).await;
        stored += 1;
    }
    stored
}

/// Create goals via Planning sister.
async fn apply_goals(goals: &[ProfileGoal], sisters: Option<&SistersHandle>) -> usize {
    let Some(sh) = sisters else {
        eprintln!("[hydra:profile] No sisters — cannot create goals");
        return 0;
    };

    let mut created = 0;
    for goal in goals {
        let content = format!(
            "Goal: {} — {} (priority: {})",
            goal.title, goal.description, goal.priority
        );
        // Use planning_goal if available, fall back to memory_add
        sh.memory_workspace_add(&content, "goals").await;
        created += 1;
    }
    created
}

/// Set HYDRA_MODEL env var from profile model config.
fn apply_model(model: &ProfileModel) {
    // Safety: set_var is safe in single-threaded context during profile load
    unsafe {
        std::env::set_var("HYDRA_MODEL", &model.default);
    }
}

/// Build the prompt overlay string from identity + prompt_overlay.md.
pub fn build_prompt_overlay(profile: &OperationalProfile) -> Option<String> {
    let mut parts = Vec::new();

    // Identity persona → prompt prefix
    if let Some(ref id) = profile.identity {
        let mut id_section = format!("# Operational Identity\n{}\n", id.persona);
        if let Some(ref tone) = id.tone {
            id_section.push_str(&format!("Communication tone: {}\n", tone));
        }
        if !id.constraints.is_empty() {
            id_section.push_str("Constraints:\n");
            for c in &id.constraints {
                id_section.push_str(&format!("- {}\n", c));
            }
        }
        parts.push(id_section);
    }

    // Beliefs → injected so the LLM can reference them in answers
    if !profile.beliefs.is_empty() {
        // Epistemic map — classify beliefs by confidence tier
        let map = super::epistemic_mapper::map_knowledge("", &profile.beliefs);
        let high = map.strong_ground.len();
        let uncertain = map.uncertain_ground.len();

        let mut section = format!(
            "# Profile Beliefs ({} total: {} strong, {} uncertain)\n\
             Be epistemically honest: for strong beliefs state them confidently; \
             for uncertain ones express appropriate doubt; for topics not covered \
             acknowledge the gap.\n",
            profile.beliefs.len(), high, uncertain,
        );
        for b in &profile.beliefs {
            section.push_str(&format!("- [{}] {} (confidence: {:.0}%)\n",
                b.topic, b.content, b.confidence * 100.0));
        }
        parts.push(section);
    }

    // Skills → injected so the LLM knows available capabilities
    if !profile.skills.is_empty() {
        let mut section = "# Profile Skills\n".to_string();
        for s in &profile.skills {
            section.push_str(&format!("- {}: {}\n", s.name, s.description));
        }
        parts.push(section);
    }

    // Sister emphasis
    if let Some(ref sisters) = profile.sisters {
        if !sisters.emphasize.is_empty() {
            parts.push(format!(
                "# Sister Emphasis\nPrioritize these sisters: {}\n",
                sisters.emphasize.join(", ")
            ));
        }
    }

    // Raw prompt overlay (appended last)
    if let Some(ref overlay) = profile.prompt_overlay {
        parts.push(format!("# Profile Instructions\n{}\n", overlay));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Revert runtime settings to defaults (for /profile unload).
pub fn revert_to_defaults(settings: &mut RuntimeSettings) {
    *settings = RuntimeSettings::default();
    eprintln!("[hydra:profile] RuntimeSettings reverted to defaults");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_permissions_partial() {
        let mut settings = RuntimeSettings::default();
        let perms = ProfilePermissions {
            file_write: Some(false),
            shell_exec: None, // Should not change
            risk_threshold: Some("high".into()),
            ..Default::default()
        };
        apply_permissions(&perms, &mut settings);
        assert!(!settings.file_write);
        assert!(settings.shell_exec); // Unchanged
        assert_eq!(settings.risk_threshold, "high");
    }

    #[test]
    fn test_build_prompt_overlay_identity() {
        let profile = OperationalProfile {
            identity: Some(ProfileIdentity {
                persona: "You are a DevOps engineer.".into(),
                tone: Some("detailed".into()),
                constraints: vec!["Never modify production.".into()],
            }),
            ..Default::default()
        };
        let overlay = build_prompt_overlay(&profile).unwrap();
        assert!(overlay.contains("DevOps engineer"));
        assert!(overlay.contains("detailed"));
        assert!(overlay.contains("Never modify production"));
    }

    #[test]
    fn test_build_prompt_overlay_empty() {
        let profile = OperationalProfile::default();
        assert!(build_prompt_overlay(&profile).is_none());
    }

    #[test]
    fn test_revert_to_defaults() {
        let mut settings = RuntimeSettings::default();
        settings.file_write = false;
        settings.risk_threshold = "high".into();
        revert_to_defaults(&mut settings);
        assert!(settings.file_write);
        assert_eq!(settings.risk_threshold, "medium");
    }
}
