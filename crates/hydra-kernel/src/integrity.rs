//! Self-Preservation: Integrity Monitor — verifies data health every 30 minutes.
//! Auto-recovers from backup on corruption. Reports in briefing.

use std::path::{Path, PathBuf};
use std::time::Instant;

/// Health status of a single data file.
#[derive(Debug, Clone, PartialEq)]
pub enum Health {
    Ok(usize),
    Corrupted(String),
    Missing,
}

/// Result of a full integrity check.
#[derive(Debug, Clone, Default)]
pub struct IntegrityReport {
    pub genome: Option<Health>,
    pub memory: Option<Health>,
    pub calibration: Option<Health>,
    pub config: Option<Health>,
    pub genome_recovered: bool,
    pub memory_recovered: bool,
    pub issues: Vec<String>,
}

impl IntegrityReport {
    pub fn is_healthy(&self) -> bool { self.issues.is_empty() }
    pub fn summary(&self) -> String {
        if self.is_healthy() {
            "All systems healthy".into()
        } else {
            format!("{} issues: {}", self.issues.len(), self.issues.join("; "))
        }
    }
}

/// Integrity monitor — periodic data verification.
pub struct IntegrityMonitor {
    pub check_interval_secs: u64,
    pub last_check: Option<Instant>,
}

impl IntegrityMonitor {
    pub fn new() -> Self {
        Self { check_interval_secs: 1800, last_check: None } // 30 min
    }

    /// Whether it's time to run a check.
    pub fn should_check(&self) -> bool {
        self.last_check.map(|t| t.elapsed().as_secs() >= self.check_interval_secs).unwrap_or(true)
    }

    /// Run a full integrity check and auto-recover if needed.
    pub fn check(&mut self) -> IntegrityReport {
        self.last_check = Some(Instant::now());
        let mut report = IntegrityReport::default();
        let data_dir = data_dir();

        // Genome DB
        let genome_path = data_dir.join("genome.db");
        report.genome = Some(check_file_health(&genome_path, "genome"));
        if let Some(Health::Corrupted(ref e)) = report.genome {
            report.issues.push(format!("genome: {e}"));
            if auto_recover(&genome_path, "genome.db") {
                report.genome_recovered = true;
                report.issues.push("genome: auto-recovered from backup".into());
            }
        }

        // Memory file
        let mem_path = data_dir.join("hydra.amem");
        report.memory = Some(check_file_health(&mem_path, "memory"));
        if let Some(Health::Corrupted(ref e)) = report.memory {
            report.issues.push(format!("memory: {e}"));
            if auto_recover(&mem_path, "hydra.amem") {
                report.memory_recovered = true;
                report.issues.push("memory: auto-recovered from backup".into());
            }
        }

        // Config file
        let config_path = dirs::home_dir().unwrap_or_default().join(".hydra/config.toml");
        report.config = Some(if config_path.exists() { Health::Ok(0) } else { Health::Missing });

        if report.is_healthy() {
            eprintln!("hydra-integrity: all healthy");
        } else {
            eprintln!("hydra-integrity: {}", report.summary());
        }
        report
    }
}

impl Default for IntegrityMonitor {
    fn default() -> Self { Self::new() }
}

/// Check a single file's health.
fn check_file_health(path: &Path, label: &str) -> Health {
    if !path.exists() { return Health::Missing; }
    match std::fs::metadata(path) {
        Ok(meta) => {
            let size = meta.len();
            if size == 0 {
                Health::Corrupted(format!("{label}: empty file"))
            } else {
                Health::Ok(size as usize)
            }
        }
        Err(e) => Health::Corrupted(format!("{label}: {e}")),
    }
}

/// Attempt to auto-recover a file from the latest backup.
fn auto_recover(target: &Path, filename: &str) -> bool {
    let backup_dir = dirs::home_dir().unwrap_or_default().join(".hydra/backups");
    if !backup_dir.exists() { return false; }
    // Find most recent backup containing this file
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&backup_dir)
        .ok().into_iter().flatten().flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    dirs.sort_by(|a, b| b.cmp(a)); // Most recent first
    for dir in dirs {
        let backup_file = dir.join(filename);
        if backup_file.exists() {
            match std::fs::copy(&backup_file, target) {
                Ok(_) => {
                    eprintln!("hydra-integrity: recovered {filename} from {}", dir.display());
                    return true;
                }
                Err(e) => eprintln!("hydra-integrity: recovery failed: {e}"),
            }
        }
    }
    false
}

fn data_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_creates() {
        let m = IntegrityMonitor::new();
        assert!(m.should_check()); // First check always due
    }

    #[test]
    fn healthy_report_is_healthy() {
        let r = IntegrityReport::default();
        assert!(r.is_healthy());
    }

    #[test]
    fn corrupted_report_not_healthy() {
        let mut r = IntegrityReport::default();
        r.issues.push("genome corrupted".into());
        assert!(!r.is_healthy());
    }

    #[test]
    fn missing_file_detected() {
        let h = check_file_health(Path::new("/nonexistent/file.db"), "test");
        assert_eq!(h, Health::Missing);
    }

    #[test]
    fn summary_format() {
        let mut r = IntegrityReport::default();
        assert_eq!(r.summary(), "All systems healthy");
        r.issues.push("genome: empty".into());
        assert!(r.summary().contains("1 issues"));
    }
}
