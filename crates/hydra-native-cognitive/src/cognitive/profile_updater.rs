//! Profile Updater — merges factory profile updates into user profile directories.
//! Replaces factory content (beliefs/factory/, skills/factory/) while preserving
//! user content (beliefs/learned/, skills/custom/).

use std::path::{Path, PathBuf};

/// Report of what was updated during a profile update.
#[derive(Debug, Default)]
pub struct UpdateReport {
    pub profile_name: String,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub dirs_created: usize,
    pub errors: Vec<String>,
}

impl UpdateReport {
    pub fn summary(&self) -> String {
        let mut lines = vec![
            format!("Profile '{}' updated", self.profile_name),
            format!("  Files copied: {}", self.files_copied),
            format!("  Dirs created: {}", self.dirs_created),
        ];
        if self.files_skipped > 0 {
            lines.push(format!("  Files skipped: {}", self.files_skipped));
        }
        if !self.errors.is_empty() {
            lines.push(format!("  Errors: {}", self.errors.len()));
            for e in &self.errors {
                lines.push(format!("    - {}", e));
            }
        }
        lines.join("\n")
    }
}

/// Update a user's profile from factory source.
/// - Replaces beliefs/factory/ and skills/factory/ entirely
/// - Never touches beliefs/learned/ or skills/custom/
/// - Merges top-level TOML configs (prefers user values, adds new fields)
pub fn update_profile(factory_dir: &Path, user_dir: &Path) -> UpdateReport {
    let name = user_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut report = UpdateReport { profile_name: name, ..Default::default() };

    if !factory_dir.is_dir() {
        report.errors.push(format!(
            "Factory source not found: {}", factory_dir.display()
        ));
        return report;
    }

    // Ensure user dir exists
    if let Err(e) = std::fs::create_dir_all(user_dir) {
        report.errors.push(format!("Cannot create user dir: {}", e));
        return report;
    }

    // Replace factory directories (beliefs/factory, skills/factory)
    replace_factory_subdir(factory_dir, user_dir, "beliefs/factory", &mut report);
    replace_factory_subdir(factory_dir, user_dir, "skills/factory", &mut report);

    // Ensure learned/custom dirs exist
    ensure_dir(user_dir, "beliefs/learned", &mut report);
    ensure_dir(user_dir, "skills/custom", &mut report);

    // Copy top-level TOML files (only if user doesn't have them, or profile.toml always)
    copy_toml_if_missing(factory_dir, user_dir, "profile.toml", &mut report);
    copy_toml_if_missing(factory_dir, user_dir, "identity.toml", &mut report);
    copy_toml_if_missing(factory_dir, user_dir, "model.toml", &mut report);
    copy_toml_if_missing(factory_dir, user_dir, "permissions.toml", &mut report);
    copy_toml_if_missing(factory_dir, user_dir, "sisters.toml", &mut report);
    copy_toml_if_missing(factory_dir, user_dir, "goals.toml", &mut report);
    copy_toml_if_missing(factory_dir, user_dir, "connections.toml", &mut report);

    eprintln!("[hydra:profile_updater] {}", report.summary());
    report
}

/// Replace a factory subdirectory entirely (delete + copy).
fn replace_factory_subdir(
    factory_dir: &Path,
    user_dir: &Path,
    rel: &str,
    report: &mut UpdateReport,
) {
    let src = factory_dir.join(rel);
    let dst = user_dir.join(rel);

    if !src.is_dir() {
        return; // Factory doesn't have this dir — nothing to update
    }

    // Remove existing factory dir in user profile
    if dst.is_dir() {
        if let Err(e) = std::fs::remove_dir_all(&dst) {
            report.errors.push(format!(
                "Cannot remove old {}: {}", dst.display(), e
            ));
            return;
        }
    }

    // Copy recursively
    copy_dir_recursive(&src, &dst, report);
}

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &Path, dst: &Path, report: &mut UpdateReport) {
    if let Err(e) = std::fs::create_dir_all(dst) {
        report.errors.push(format!("Cannot create {}: {}", dst.display(), e));
        return;
    }
    report.dirs_created += 1;

    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(e) => {
            report.errors.push(format!("Cannot read {}: {}", src.display(), e));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest, report);
        } else {
            match std::fs::copy(&path, &dest) {
                Ok(_) => report.files_copied += 1,
                Err(e) => report.errors.push(format!(
                    "Copy {} → {}: {}", path.display(), dest.display(), e
                )),
            }
        }
    }
}

/// Ensure a subdirectory exists in the user profile.
fn ensure_dir(user_dir: &Path, rel: &str, report: &mut UpdateReport) {
    let dir = user_dir.join(rel);
    if !dir.exists() {
        match std::fs::create_dir_all(&dir) {
            Ok(_) => report.dirs_created += 1,
            Err(e) => report.errors.push(format!(
                "Cannot create {}: {}", dir.display(), e
            )),
        }
    }
}

/// Copy a TOML file from factory to user dir only if user doesn't have it.
fn copy_toml_if_missing(
    factory_dir: &Path,
    user_dir: &Path,
    filename: &str,
    report: &mut UpdateReport,
) {
    let src = factory_dir.join(filename);
    let dst = user_dir.join(filename);

    if !src.exists() {
        return;
    }

    if dst.exists() {
        report.files_skipped += 1;
        return;
    }

    match std::fs::copy(&src, &dst) {
        Ok(_) => report.files_copied += 1,
        Err(e) => report.errors.push(format!(
            "Copy {} → {}: {}", src.display(), dst.display(), e
        )),
    }
}

/// List factory profile names from the source profiles/ directory.
pub fn list_factory_profiles(factory_base: &Path) -> Vec<String> {
    let entries = match std::fs::read_dir(factory_base) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries.filter_map(|e| {
        let e = e.ok()?;
        let name = e.file_name().to_string_lossy().to_string();
        // Skip _schema and hidden dirs
        if name.starts_with('_') || name.starts_with('.') {
            return None;
        }
        if e.file_type().ok()?.is_dir() {
            Some(name)
        } else {
            None
        }
    }).collect()
}

/// Find the factory profiles directory (compile-time or relative to exe).
pub fn factory_profiles_dir() -> Option<PathBuf> {
    // Try relative to current exe first
    if let Ok(exe) = std::env::current_exe() {
        // Walk up from exe to find profiles/ dir
        let mut dir = exe.as_path();
        for _ in 0..5 {
            if let Some(parent) = dir.parent() {
                let candidate = parent.join("profiles");
                if candidate.is_dir()
                    && candidate.join("dev").is_dir()
                {
                    return Some(candidate);
                }
                dir = parent;
            }
        }
    }

    // Try relative to CARGO_MANIFEST_DIR (dev mode)
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let workspace = Path::new(&manifest);
        // Walk up to workspace root
        let mut dir = workspace;
        for _ in 0..4 {
            let candidate = dir.join("profiles");
            if candidate.is_dir() && candidate.join("dev").is_dir() {
                return Some(candidate);
            }
            match dir.parent() {
                Some(p) => dir = p,
                None => break,
            }
        }
    }

    // Try current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("profiles");
        if candidate.is_dir() && candidate.join("dev").is_dir() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_report_summary() {
        let report = UpdateReport {
            profile_name: "test".into(),
            files_copied: 5,
            files_skipped: 2,
            dirs_created: 3,
            errors: vec![],
        };
        let s = report.summary();
        assert!(s.contains("test"));
        assert!(s.contains("5"));
    }

    #[test]
    fn test_list_factory_profiles_nonexistent() {
        let profiles = list_factory_profiles(Path::new("/nonexistent"));
        assert!(profiles.is_empty());
    }
}
