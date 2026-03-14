use std::path::{Path, PathBuf};

use crate::error::InstallerError;

/// Return the conventional completions directory for a given shell.
pub fn completion_path(shell: &str) -> Option<PathBuf> {
    match shell {
        "bash" => {
            // XDG_DATA_HOME/bash-completion/completions or ~/.local/share/bash-completion/completions
            let base = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    dirs_fallback_data_home()
                });
            Some(base.join("bash-completion").join("completions"))
        }
        "zsh" => {
            // ~/.zsh/completions is a common user-local path
            Some(dirs_fallback_home().join(".zsh").join("completions"))
        }
        "fish" => {
            let base = std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| dirs_fallback_home().join(".config"));
            Some(base.join("fish").join("completions"))
        }
        _ => None,
    }
}

/// Install a placeholder completion script for Hydra into `install_dir`.
pub fn install_completions(shell: &str, install_dir: &Path) -> Result<(), InstallerError> {
    let filename = match shell {
        "bash" => "hydra.bash",
        "zsh" => "_hydra",
        "fish" => "hydra.fish",
        other => {
            return Err(InstallerError::UnsupportedShell(other.to_string()));
        }
    };

    std::fs::create_dir_all(install_dir).map_err(|e| InstallerError::Io {
        context: format!("creating completions dir {}", install_dir.display()),
        source: e,
    })?;

    let content = match shell {
        "bash" => "# Hydra bash completions (placeholder)\ncomplete -F _hydra hydra\n",
        "zsh" => "#compdef hydra\n# Hydra zsh completions (placeholder)\n",
        "fish" => "# Hydra fish completions (placeholder)\ncomplete -c hydra -f\n",
        _ => unreachable!(),
    };

    let dest = install_dir.join(filename);
    std::fs::write(&dest, content).map_err(|e| InstallerError::Io {
        context: format!("writing completions to {}", dest.display()),
        source: e,
    })?;

    Ok(())
}

// --- helpers (avoid pulling in the `dirs` crate) ---

fn dirs_fallback_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/hydra-fallback-home"))
}

fn dirs_fallback_data_home() -> PathBuf {
    dirs_fallback_home().join(".local").join("share")
}
