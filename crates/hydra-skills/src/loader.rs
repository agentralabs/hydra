//! Skill loader — validates manifests before loading.

use crate::constants::{SKILL_ID_MAX_LEN, SKILL_VERSION_MAX_LEN};
use crate::errors::SkillError;
use crate::skill::SkillManifest;

/// Validates skill manifests before they can be loaded into the registry.
pub struct SkillLoader;

impl SkillLoader {
    /// Validate a skill manifest.
    ///
    /// Checks:
    /// - Skill ID is non-empty and within `SKILL_ID_MAX_LEN`
    /// - Version is non-empty and within `SKILL_VERSION_MAX_LEN`
    /// - At least one capability is declared
    ///
    /// Returns `Ok(())` if the manifest is valid.
    pub fn validate(manifest: &SkillManifest) -> Result<(), SkillError> {
        if manifest.id.is_empty() || manifest.id.len() > SKILL_ID_MAX_LEN {
            return Err(SkillError::NotFound {
                name: format!(
                    "invalid skill ID (empty or exceeds {} chars)",
                    SKILL_ID_MAX_LEN
                ),
            });
        }

        if manifest.version.is_empty() || manifest.version.len() > SKILL_VERSION_MAX_LEN {
            return Err(SkillError::NotFound {
                name: format!(
                    "invalid version for '{}' (empty or exceeds {} chars)",
                    manifest.id, SKILL_VERSION_MAX_LEN
                ),
            });
        }

        if manifest.capabilities.is_empty() {
            return Err(SkillError::NotFound {
                name: format!(
                    "skill '{}' must declare at least one capability",
                    manifest.id
                ),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::SkillDomain;

    fn valid_manifest() -> SkillManifest {
        SkillManifest::new("test-skill", "Test Skill", "1.0.0", SkillDomain::Operations)
            .with_capabilities(vec!["testing".to_string()])
    }

    #[test]
    fn valid_manifest_passes() {
        assert!(SkillLoader::validate(&valid_manifest()).is_ok());
    }

    #[test]
    fn empty_id_fails() {
        let mut m = valid_manifest();
        m.id = String::new();
        assert!(SkillLoader::validate(&m).is_err());
    }

    #[test]
    fn long_id_fails() {
        let mut m = valid_manifest();
        m.id = "x".repeat(SKILL_ID_MAX_LEN + 1);
        assert!(SkillLoader::validate(&m).is_err());
    }

    #[test]
    fn empty_version_fails() {
        let mut m = valid_manifest();
        m.version = String::new();
        assert!(SkillLoader::validate(&m).is_err());
    }

    #[test]
    fn empty_capabilities_fails() {
        let mut m = valid_manifest();
        m.capabilities.clear();
        assert!(SkillLoader::validate(&m).is_err());
    }
}
