//! Private feature availability checks and stubs.
//! Public builds get no-op stubs. Private builds get real implementations.

/// Check which private features are available at compile time.
pub struct PrivateFeatures;

impl PrivateFeatures {
    pub fn advanced_gate() -> bool {
        cfg!(feature = "advanced-gate")
    }

    pub fn consolidation() -> bool {
        cfg!(feature = "consolidation")
    }

    pub fn omniscience() -> bool {
        cfg!(feature = "omniscience")
    }

    pub fn collective() -> bool {
        cfg!(feature = "collective")
    }

    pub fn replay() -> bool {
        cfg!(feature = "replay")
    }

    pub fn enterprise() -> bool {
        cfg!(feature = "enterprise")
    }

    /// Returns a list of all active private features.
    pub fn active() -> Vec<&'static str> {
        let mut features = Vec::new();
        if Self::advanced_gate() {
            features.push("advanced-gate");
        }
        if Self::consolidation() {
            features.push("consolidation");
        }
        if Self::omniscience() {
            features.push("omniscience");
        }
        if Self::collective() {
            features.push("collective");
        }
        if Self::replay() {
            features.push("replay");
        }
        if Self::enterprise() {
            features.push("enterprise");
        }
        features
    }

    /// Returns true if any private features are active.
    pub fn any_active() -> bool {
        !Self::active().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_features_default_off() {
        // In default build, no private features should be active
        #[cfg(not(feature = "private"))]
        {
            assert!(!PrivateFeatures::advanced_gate());
            assert!(!PrivateFeatures::consolidation());
            assert!(!PrivateFeatures::omniscience());
            assert!(!PrivateFeatures::collective());
            assert!(!PrivateFeatures::replay());
            assert!(!PrivateFeatures::enterprise());
            assert!(!PrivateFeatures::any_active());
            assert!(PrivateFeatures::active().is_empty());
        }
    }

    #[test]
    fn test_active_returns_list() {
        let active = PrivateFeatures::active();
        // Just verify it's a valid list (contents depend on build features)
        assert!(active.len() <= 6);
    }
}
