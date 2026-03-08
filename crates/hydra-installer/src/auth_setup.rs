use std::path::Path;

use crate::error::InstallerError;
use crate::profile::InstallProfile;

/// Generate a random 32-byte hex token (64 hex characters).
pub fn generate_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    // Use the stdlib's random state to produce entropy without pulling in
    // the `rand` crate. We hash multiple seeds to fill 32 bytes.
    let mut bytes = [0u8; 32];
    for chunk in bytes.chunks_mut(8) {
        let s = RandomState::new();
        let mut h = s.build_hasher();
        h.write_u64(chunk.as_ptr() as u64);
        let val = h.finish();
        for (i, b) in val.to_le_bytes().iter().enumerate() {
            if i < chunk.len() {
                chunk[i] = *b;
            }
        }
    }

    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Generate an auth token and persist it to `data_dir/hydra-auth-token`.
pub fn setup_auth(data_dir: &Path) -> Result<String, InstallerError> {
    std::fs::create_dir_all(data_dir).map_err(|e| InstallerError::Io {
        context: format!("creating auth data dir {}", data_dir.display()),
        source: e,
    })?;

    let token = generate_token();
    let token_path = data_dir.join("hydra-auth-token");

    std::fs::write(&token_path, &token).map_err(|e| InstallerError::Io {
        context: format!("writing auth token to {}", token_path.display()),
        source: e,
    })?;

    Ok(token)
}

/// Returns `true` if the given profile requires authentication setup.
pub fn auth_required(profile: &InstallProfile) -> bool {
    matches!(profile, InstallProfile::Server)
}
