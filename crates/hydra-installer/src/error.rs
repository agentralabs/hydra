use std::path::PathBuf;

/// Errors that can occur during installation.
#[derive(Debug)]
pub enum InstallerError {
    /// Filesystem I/O error with context.
    Io {
        context: String,
        source: std::io::Error,
    },
    /// Failed to parse a configuration file.
    ConfigParse {
        path: PathBuf,
        source: serde_json::Error,
    },
    /// Unsupported shell for completions.
    UnsupportedShell(String),
}

impl std::fmt::Display for InstallerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::ConfigParse { path, source } => {
                write!(f, "failed to parse {}: {source}", path.display())
            }
            Self::UnsupportedShell(shell) => write!(f, "unsupported shell: {shell}"),
        }
    }
}

impl std::error::Error for InstallerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::ConfigParse { source, .. } => Some(source),
            Self::UnsupportedShell(_) => None,
        }
    }
}
