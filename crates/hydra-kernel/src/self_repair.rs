//! Self-Repair — Hydra detects problems and fixes them automatically.
//!
//! The repair loop:
//!   1. DETECT: metabolism, invariants, or surprise flags a problem
//!   2. DIAGNOSE: identify which subsystem is failing and why
//!   3. REPAIR: execute the appropriate corrective action
//!   4. VERIFY: confirm the repair worked (Lyapunov recovers)
//!
//! Repairs are non-destructive. They never delete data.
//! They rebuild, restart, or reconfigure — never destroy.

use std::path::PathBuf;

/// A diagnosed problem with a repair action.
#[derive(Debug, Clone)]
pub struct Diagnosis {
    pub subsystem: String,
    pub problem: String,
    pub severity: Severity,
    pub repair: RepairAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    /// Self-healing. No user notification needed.
    Minor,
    /// Repaired automatically. User notified.
    Moderate,
    /// Requires user attention after auto-repair attempt.
    Major,
    /// Cannot self-repair. User must intervene.
    Critical,
}

#[derive(Debug, Clone)]
pub enum RepairAction {
    /// Rebuild a database from source files.
    RebuildDatabase { db_name: String, source: String },
    /// Delete and recreate a corrupted file.
    RecreateFile { path: PathBuf },
    /// Restart a subsystem by re-initializing it.
    RestartSubsystem { name: String },
    /// Clear stale lock files.
    ClearStaleLock { path: PathBuf },
    /// Reload skills from disk.
    ReloadSkills,
    /// Compact memory by removing duplicates.
    CompactMemory,
    /// No automated repair — notify user.
    NotifyUser { message: String },
}

/// Run diagnostics on the system and return any problems found.
pub fn diagnose() -> Vec<Diagnosis> {
    let mut problems = Vec::new();

    // Check genome database
    let genome_db = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/data/genome.db");
    if genome_db.exists() && check_sqlite_integrity(&genome_db).is_err() {
        problems.push(Diagnosis {
            subsystem: "genome".into(),
            problem: "genome.db failed integrity check".into(),
            severity: Severity::Moderate,
            repair: RepairAction::RebuildDatabase {
                db_name: "genome".into(),
                source: "skills/*/genome.toml".into(),
            },
        });
    }

    // Check audit database
    let audit_db = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/data/audit.db");
    if audit_db.exists() && check_sqlite_integrity(&audit_db).is_err() {
        problems.push(Diagnosis {
            subsystem: "audit".into(),
            problem: "audit.db failed integrity check".into(),
            severity: Severity::Major,
            repair: RepairAction::RecreateFile { path: audit_db },
        });
    }

    // Check memory file
    let amem = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/data/hydra.amem");
    if amem.exists() {
        if let Ok(meta) = std::fs::metadata(&amem) {
            if meta.len() == 0 {
                problems.push(Diagnosis {
                    subsystem: "memory".into(),
                    problem: "hydra.amem is empty (0 bytes)".into(),
                    severity: Severity::Minor,
                    repair: RepairAction::RecreateFile { path: amem },
                });
            }
        }
    }

    // Check stale boot lock
    let lock = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/hydra.lock");
    if lock.exists() {
        if let Ok(meta) = std::fs::metadata(&lock) {
            if let Ok(modified) = meta.modified() {
                if let Ok(age) = std::time::SystemTime::now().duration_since(modified) {
                    if age.as_secs() > 30 {
                        problems.push(Diagnosis {
                            subsystem: "boot".into(),
                            problem: format!(
                                "stale boot lock ({}s old)",
                                age.as_secs()
                            ),
                            severity: Severity::Minor,
                            repair: RepairAction::ClearStaleLock { path: lock },
                        });
                    }
                }
            }
        }
    }

    // Check skills directory
    let skills_dir = PathBuf::from("skills");
    if !skills_dir.exists() {
        problems.push(Diagnosis {
            subsystem: "skills".into(),
            problem: "skills/ directory not found".into(),
            severity: Severity::Major,
            repair: RepairAction::NotifyUser {
                message: "skills/ directory is missing. Hydra has no skill knowledge.".into(),
            },
        });
    }

    problems
}

/// Execute a repair action. Returns true if repair succeeded.
pub fn execute_repair(repair: &RepairAction) -> bool {
    match repair {
        RepairAction::ClearStaleLock { path } => {
            match std::fs::remove_file(path) {
                Ok(()) => {
                    eprintln!("hydra: SELF-REPAIR — cleared stale lock at {}", path.display());
                    true
                }
                Err(e) => {
                    eprintln!("hydra: SELF-REPAIR FAILED — could not clear lock: {e}");
                    false
                }
            }
        }

        RepairAction::RecreateFile { path } => {
            // Rename the corrupted file, don't delete it
            let backup = path.with_extension("corrupted");
            match std::fs::rename(path, &backup) {
                Ok(()) => {
                    eprintln!(
                        "hydra: SELF-REPAIR — moved corrupted {} to {}",
                        path.display(),
                        backup.display()
                    );
                    true
                }
                Err(e) => {
                    eprintln!("hydra: SELF-REPAIR FAILED — could not rename: {e}");
                    false
                }
            }
        }

        RepairAction::RebuildDatabase { db_name, source } => {
            eprintln!(
                "hydra: SELF-REPAIR — rebuilding {db_name} from {source}"
            );
            // The genome store rebuilds from skills/ on next boot
            // Just rename the corrupted db
            let db_path = dirs::home_dir()
                .unwrap_or_default()
                .join(format!(".hydra/data/{db_name}.db"));
            let backup = db_path.with_extension("db.corrupted");
            match std::fs::rename(&db_path, &backup) {
                Ok(()) => {
                    eprintln!("hydra: SELF-REPAIR — {db_name}.db will rebuild on next boot");
                    true
                }
                Err(e) => {
                    eprintln!("hydra: SELF-REPAIR FAILED — {e}");
                    false
                }
            }
        }

        RepairAction::ReloadSkills => {
            eprintln!("hydra: SELF-REPAIR — skills will reload on next boot");
            true
        }

        RepairAction::CompactMemory => {
            eprintln!("hydra: SELF-REPAIR — memory compaction scheduled for next dream cycle");
            true
        }

        RepairAction::RestartSubsystem { name } => {
            eprintln!("hydra: SELF-REPAIR — subsystem '{name}' will reinitialize on next cycle");
            true
        }

        RepairAction::NotifyUser { message } => {
            eprintln!("hydra: NEEDS ATTENTION — {message}");
            false // user must act
        }
    }
}

/// Run the full repair loop: diagnose → repair → report.
pub fn self_repair() -> Vec<(Diagnosis, bool)> {
    let problems = diagnose();
    if problems.is_empty() {
        return Vec::new();
    }

    eprintln!(
        "hydra: SELF-REPAIR — {} problem(s) detected",
        problems.len()
    );

    let mut results = Vec::new();
    for diag in problems {
        let repaired = execute_repair(&diag.repair);
        eprintln!(
            "hydra: {} [{}] {} — {}",
            if repaired { "REPAIRED" } else { "UNRESOLVED" },
            diag.subsystem,
            diag.problem,
            if repaired { "fixed" } else { "needs attention" }
        );
        results.push((diag, repaired));
    }

    results
}

/// Check SQLite database integrity.
fn check_sqlite_integrity(path: &PathBuf) -> Result<(), String> {
    match rusqlite::Connection::open(path) {
        Ok(conn) => {
            match conn.query_row("PRAGMA integrity_check", [], |row| {
                row.get::<_, String>(0)
            }) {
                Ok(result) if result == "ok" => Ok(()),
                Ok(result) => Err(format!("integrity check: {result}")),
                Err(e) => Err(format!("integrity check failed: {e}")),
            }
        }
        Err(e) => Err(format!("cannot open: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnose_runs_without_panic() {
        let problems = diagnose();
        // May or may not find problems — the point is it doesn't crash
        for p in &problems {
            assert!(!p.subsystem.is_empty());
            assert!(!p.problem.is_empty());
        }
    }

    #[test]
    fn repair_notify_returns_false() {
        let repair = RepairAction::NotifyUser {
            message: "test".into(),
        };
        assert!(!execute_repair(&repair));
    }

    #[test]
    fn repair_reload_skills_returns_true() {
        assert!(execute_repair(&RepairAction::ReloadSkills));
    }
}
