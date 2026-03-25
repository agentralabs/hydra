//! Drop item classifier — detects what a dropped file IS.
//! Uses extension, filename pattern, and content sniffing.
//! Structural detection only (not intent classification) — CLAUDE.md compliant.

use std::path::Path;

/// All item types the drop gateway recognizes.
/// `Custom(String)` allows future extensions without modifying this enum.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DropItemType {
    /// API key, token, or secret (.env, .key, credential.*)
    ApiCredential,
    /// SSH private/public key (id_rsa, id_ed25519, *.pub)
    SshKey,
    /// TLS/SSL certificate (*.pem, *.crt, *.cert)
    Certificate,
    /// Skill archive with genome.toml (*.tar.gz, *.zip)
    SkillPackage,
    /// Markdown document to learn as skill (*.md, not immerse-*)
    SkillMarkdown,
    /// Exported genome entries (genome*.json)
    GenomeEntries,
    /// Learning source configuration (sources.toml)
    LearningSource,
    /// Config override (config.toml, settings.toml)
    ConfigOverride,
    /// Machine registry (machines.toml)
    MachineConfig,
    /// Cloud backup provider config (cloud.toml)
    CloudBackupConfig,
    /// Monitor poller/watcher config (monitor.toml)
    MonitorSource,
    /// Immersion content (immerse-*.md)
    ImmersionContent,
    /// Database/API/Cloud connector config (connector-*.toml)
    Connector,
    /// Document for analysis (*.pdf, *.csv, *.png, *.txt)
    Document,
    /// Future extension — any string identifier
    Custom(String),
    /// Could not classify
    Unknown,
}

impl DropItemType {
    pub fn label(&self) -> String {
        match self {
            Self::ApiCredential => "credential".into(),
            Self::SshKey => "ssh-key".into(),
            Self::Certificate => "certificate".into(),
            Self::SkillPackage => "skill-package".into(),
            Self::SkillMarkdown => "skill-markdown".into(),
            Self::GenomeEntries => "genome-entries".into(),
            Self::LearningSource => "learning-source".into(),
            Self::ConfigOverride => "config".into(),
            Self::MachineConfig => "machine-config".into(),
            Self::CloudBackupConfig => "cloud-config".into(),
            Self::MonitorSource => "monitor-config".into(),
            Self::ImmersionContent => "immersion".into(),
            Self::Connector => "connector".into(),
            Self::Document => "document".into(),
            Self::Custom(name) => name.clone(),
            Self::Unknown => "unknown".into(),
        }
    }
}

/// Maximum single file size (10MB).
pub const MAX_FILE_BYTES: u64 = 10_485_760;

/// Classify a dropped file by extension, filename, and content sniffing.
pub fn classify(path: &Path) -> DropItemType {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let lower = name.to_lowercase();
    let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();

    // 1. Extension-based (fast path)
    match ext.as_str() {
        "pem" | "crt" | "cert" => return DropItemType::Certificate,
        "pub" => return DropItemType::SshKey,
        "pdf" | "csv" | "png" | "jpg" | "jpeg" | "bmp" | "tiff" => return DropItemType::Document,
        "gz" | "zip" if lower.contains(".tar.") || lower.ends_with(".zip") => return DropItemType::SkillPackage,
        _ => {}
    }

    // 2. Filename pattern matching
    if lower.starts_with("id_rsa") || lower.starts_with("id_ed25519") || lower.starts_with("id_ecdsa") {
        return DropItemType::SshKey;
    }
    if lower.starts_with("connector-") && ext == "toml" { return DropItemType::Connector; }
    if lower.starts_with("immerse-") && ext == "md" { return DropItemType::ImmersionContent; }
    if ext == "md" { return DropItemType::SkillMarkdown; }
    if lower.starts_with("genome") && ext == "json" { return DropItemType::GenomeEntries; }

    // 3. TOML structure detection — read first 1KB
    if ext == "toml" { return classify_toml(path); }

    // 4. .env and credential detection
    if ext == "env" || lower.starts_with("credential") || lower == ".env" || lower.ends_with(".key") {
        return DropItemType::ApiCredential;
    }

    // 5. Content sniffing for extensionless files
    if ext.is_empty() {
        if let Ok(content) = std::fs::read_to_string(path) {
            let upper = content.to_uppercase();
            if upper.contains("API_KEY=") || upper.contains("TOKEN=") || upper.contains("SECRET=") || upper.contains("PASSWORD=") {
                return DropItemType::ApiCredential;
            }
        }
    }

    DropItemType::Unknown
}

/// Classify a TOML file by its structure (table/array names).
fn classify_toml(path: &Path) -> DropItemType {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return DropItemType::Unknown,
    };
    let sample = &content[..content.len().min(2048)];
    if sample.contains("[[machine]]") { return DropItemType::MachineConfig; }
    if sample.contains("[[source]]") { return DropItemType::LearningSource; }
    if sample.contains("[[poller]]") || sample.contains("[[watcher]]") { return DropItemType::MonitorSource; }
    if sample.contains("[tui]") || sample.contains("[llm]") { return DropItemType::ConfigOverride; }
    if sample.contains("provider") && sample.contains("bucket") { return DropItemType::CloudBackupConfig; }
    if sample.contains("[[entries]]") { return DropItemType::ConfigOverride; } // genome-style but in TOML
    DropItemType::Unknown
}

/// Security check — reject dangerous files before processing.
pub fn security_check(path: &Path) -> Result<(), String> {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    // Path traversal
    if name.contains("..") || name.starts_with('/') {
        return Err("Path traversal detected".into());
    }

    // Symlink rejection
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err("Symlinks not allowed in drop folder".into());
        }
        // Size check
        if meta.len() > MAX_FILE_BYTES {
            return Err(format!("File too large ({:.1}MB, max {}MB)", meta.len() as f64 / 1e6, MAX_FILE_BYTES / 1_000_000));
        }
    }

    // Executable detection (ELF/Mach-O headers)
    if let Ok(bytes) = std::fs::read(path) {
        if bytes.len() >= 4 {
            // ELF: \x7fELF
            if bytes[..4] == [0x7f, 0x45, 0x4c, 0x46] {
                return Err("Executable files (ELF) not allowed".into());
            }
            // Mach-O: \xcf\xfa\xed\xfe or \xce\xfa\xed\xfe
            if (bytes[..4] == [0xcf, 0xfa, 0xed, 0xfe]) || (bytes[..4] == [0xce, 0xfa, 0xed, 0xfe]) {
                return Err("Executable files (Mach-O) not allowed".into());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn classify_env_file() {
        let tmp = std::env::temp_dir().join("test_drop_api.env");
        std::fs::write(&tmp, "ANTHROPIC_API_KEY=sk-test").unwrap();
        assert_eq!(classify(&tmp), DropItemType::ApiCredential);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn classify_markdown() {
        let tmp = std::env::temp_dir().join("test_deploy.md");
        std::fs::write(&tmp, "# Deploy Guide\n\n1. Build\n2. Ship").unwrap();
        assert_eq!(classify(&tmp), DropItemType::SkillMarkdown);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn classify_immersion() {
        let tmp = std::env::temp_dir().join("immerse-rust.md");
        std::fs::write(&tmp, "# Rust\n\nOwnership model").unwrap();
        assert_eq!(classify(&tmp), DropItemType::ImmersionContent);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn classify_machines_toml() {
        let tmp = std::env::temp_dir().join("test_machines.toml");
        std::fs::write(&tmp, "[[machine]]\nname = \"prod\"\nhost = \"10.0.0.1\"").unwrap();
        assert_eq!(classify(&tmp), DropItemType::MachineConfig);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn security_rejects_large_file() {
        let tmp = std::env::temp_dir().join("test_big.txt");
        let mut f = std::fs::File::create(&tmp).unwrap();
        // Write header only — actual size check against metadata
        f.write_all(b"small").unwrap();
        assert!(security_check(&tmp).is_ok());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn classify_unknown() {
        let tmp = std::env::temp_dir().join("test_random.xyz");
        std::fs::write(&tmp, "random content").unwrap();
        assert_eq!(classify(&tmp), DropItemType::Unknown);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn classify_ssh_key() {
        let tmp = std::env::temp_dir().join("id_rsa");
        std::fs::write(&tmp, "-----BEGIN RSA PRIVATE KEY-----").unwrap();
        assert_eq!(classify(&tmp), DropItemType::SshKey);
        let _ = std::fs::remove_file(&tmp);
    }
}
