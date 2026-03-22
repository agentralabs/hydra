//! fixer.rs — Attempts automated fixes for known failure patterns.

use crate::TestResult;

/// Attempt to fix a failed test result.
/// Returns updated TestResult with fix_attempted and fix_succeeded set.
pub fn attempt_fix(result: &mut TestResult) {
    result.fix_attempted = true;

    let fix_applied = match (result.crate_name.as_str(), result.capability.as_str()) {
        // If skill not present -- check for misplaced files
        ("skill-loading", "skills_present") => {
            fix_skill_loading(result)
        }

        // Default: log for manual review
        _ => {
            result.fix_notes = Some(format!(
                "No automated fix available for {}::{}. Needs manual review.",
                result.crate_name, result.capability,
            ));
            false
        }
    };

    result.fix_succeeded = Some(fix_applied);
}

fn fix_skill_loading(result: &mut TestResult) -> bool {
    let skills_dir = std::path::PathBuf::from("skills");
    let general_dir = skills_dir.join("general");
    if !general_dir.exists() {
        result.fix_notes = Some(
            "skills/general/ not found. \
             Unzip skills-general.zip into the hydra directory and restart."
                .into(),
        );
        return false;
    }
    if !general_dir.join("genome.toml").exists() {
        result.fix_notes = Some(
            "skills/general/genome.toml missing. \
             The skill folder is incomplete."
                .into(),
        );
        return false;
    }
    result.fix_notes = Some(
        "skills/general/ present. May need restart to load.".into(),
    );
    false
}
