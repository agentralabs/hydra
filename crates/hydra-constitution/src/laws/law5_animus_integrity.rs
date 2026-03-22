//! Law 5: Animus Integrity
//! The internal signal semiring (S, ⊕, ⊗, 0, 1) is closed.
//! No element from outside the semiring can be injected without verification.
//! Internal Animus Prime bus cannot be spoofed, intercepted, or modified.

use crate::{
    constants::{ANIMUS_MAGIC, ANIMUS_VERSION},
    errors::ConstitutionError,
    laws::{ConstitutionalLaw, LawCheckContext, LawId},
};

pub struct AnimusIntegrity;

impl ConstitutionalLaw for AnimusIntegrity {
    fn law_id(&self) -> LawId {
        LawId::Law5AnimusIntegrity
    }

    fn description(&self) -> &'static str {
        "Internal Animus Prime communication cannot be intercepted, modified, \
         or spoofed by any layer above Tier 0. The nervous system is inviolable."
    }

    fn check(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        self.check_animus_injection(context)?;
        self.check_animus_interception(context)?;
        self.check_animus_format(context)?;
        Ok(())
    }
}

impl AnimusIntegrity {
    fn check_animus_injection(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        let action = context.action_type.as_str();
        let injection_actions = [
            "animus.inject",
            "animus.spoof",
            "animus.forge",
            "bus.inject",
            "signal.forge",
        ];

        if injection_actions.iter().any(|a| action.starts_with(a)) {
            return Err(ConstitutionError::AnimusBusViolation {
                reason: format!(
                    "action '{}' attempts to inject forged signal into Animus bus",
                    action
                ),
            });
        }

        Ok(())
    }

    fn check_animus_interception(
        &self,
        context: &LawCheckContext,
    ) -> Result<(), ConstitutionError> {
        let action = context.action_type.as_str();
        let interception_actions = [
            "animus.intercept",
            "animus.tap",
            "animus.sniff",
            "bus.intercept",
            "signal.intercept",
        ];

        if interception_actions.iter().any(|a| action.starts_with(a)) {
            return Err(ConstitutionError::AnimusBusViolation {
                reason: format!(
                    "action '{}' attempts to intercept Animus internal bus",
                    action
                ),
            });
        }

        Ok(())
    }

    fn check_animus_format(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        // If a signal claims to be Animus Prime, verify its header
        if let Some(magic_str) = context.metadata.get("animus_magic") {
            let magic_bytes = magic_str.as_bytes();
            if magic_bytes.len() < 4 || &magic_bytes[..4] != ANIMUS_MAGIC {
                return Err(ConstitutionError::AnimusBusViolation {
                    reason: format!(
                        "invalid Animus magic header: expected {:?}, got {:?}",
                        ANIMUS_MAGIC,
                        &magic_bytes[..magic_bytes.len().min(4)]
                    ),
                });
            }
        }

        if let Some(version_str) = context.metadata.get("animus_version") {
            if let Ok(version) = version_str.parse::<u32>() {
                if version != ANIMUS_VERSION {
                    return Err(ConstitutionError::AnimusBusViolation {
                        reason: format!(
                            "Animus version mismatch: expected {:#010x}, got {:#010x}",
                            ANIMUS_VERSION, version
                        ),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law() -> AnimusIntegrity {
        AnimusIntegrity
    }

    #[test]
    fn permits_legitimate_signal() {
        let ctx = LawCheckContext::new("act-001", "signal.emit")
            .with_meta("animus_magic", "ANMA")
            .with_meta("animus_version", ANIMUS_VERSION.to_string());
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn blocks_signal_injection() {
        let ctx = LawCheckContext::new("act-002", "animus.inject");
        assert!(matches!(
            law().check(&ctx),
            Err(ConstitutionError::AnimusBusViolation { .. })
        ));
    }

    #[test]
    fn blocks_bus_interception() {
        let ctx = LawCheckContext::new("act-003", "animus.intercept");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn blocks_signal_forgery() {
        let ctx = LawCheckContext::new("act-004", "signal.forge");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn blocks_invalid_magic_header() {
        let ctx = LawCheckContext::new("act-005", "signal.emit").with_meta("animus_magic", "FAKE");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn permits_action_without_animus_header() {
        // Non-Animus actions don't need to provide the header
        let ctx = LawCheckContext::new("act-006", "agent.spawn");
        assert!(law().check(&ctx).is_ok());
    }
}
