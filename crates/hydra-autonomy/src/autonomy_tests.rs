#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_initial_autonomy_is_observer() {
        let autonomy = GraduatedAutonomy::default();
        let level = autonomy.autonomy_level(&TrustDomain::global());
        assert_eq!(level, AutonomyLevel::Observer);
    }

    #[test]
    fn test_trust_earning() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        // Earn trust through many successful actions
        for _ in 0..30 {
            autonomy.record_success(&domain, ActionRisk::Medium);
        }

        let score = autonomy.trust_score(&domain).unwrap();
        assert!(score.value > 0.5);
        assert!(score.autonomy_level() >= AutonomyLevel::Assistant);
    }

    #[test]
    fn test_trust_penalty() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        // Build up trust
        for _ in 0..20 {
            autonomy.record_success(&domain, ActionRisk::Medium);
        }

        let before = autonomy.trust_score(&domain).unwrap().value;

        // Fail a high-risk action
        autonomy.record_failure(&domain, ActionRisk::High);
        let after = autonomy.trust_score(&domain).unwrap().value;

        assert!(after < before);
    }

    #[test]
    fn test_action_check_low_trust() {
        let autonomy = GraduatedAutonomy::default();
        let domain = TrustDomain::global();

        // Low trust should block high-risk actions
        let decision = autonomy.check_action(&domain, ActionRisk::High);
        assert!(!decision.allowed);
        assert!(decision.requires_approval);
    }

    #[test]
    fn test_autonomy_ceiling() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Assistant);
        let domain = TrustDomain::global();

        // Even with max trust, can't exceed ceiling
        for _ in 0..50 {
            autonomy.record_success(&domain, ActionRisk::High);
        }

        let level = autonomy.autonomy_level(&domain);
        assert!(level <= AutonomyLevel::Assistant);
    }

    #[test]
    fn test_trust_decay() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous)
            .with_decay_factor(0.9);
        let domain = TrustDomain::global();

        // Build trust
        for _ in 0..20 {
            autonomy.record_success(&domain, ActionRisk::Medium);
        }

        let before = autonomy.trust_score(&domain).unwrap().value;
        autonomy.apply_decay();
        let after = autonomy.trust_score(&domain).unwrap().value;

        assert!(after < before);
    }

    #[test]
    fn test_domain_specific_trust() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let file_domain = TrustDomain::new("file_operations");
        let net_domain = TrustDomain::new("network");

        // Build trust only in file domain
        for _ in 0..20 {
            autonomy.record_success(&file_domain, ActionRisk::Medium);
        }

        let file_level = autonomy.autonomy_level(&file_domain);
        let net_level = autonomy.autonomy_level(&net_domain);

        assert!(file_level > net_level);
    }

    #[test]
    fn test_critical_always_requires_approval() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        // Max out trust
        for _ in 0..100 {
            autonomy.record_success(&domain, ActionRisk::Critical);
        }

        let decision = autonomy.check_action(&domain, ActionRisk::Critical);
        assert!(decision.requires_approval);
    }

    #[test]
    fn test_trust_score_history() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        autonomy.record_success(&domain, ActionRisk::Low);
        autonomy.record_failure(&domain, ActionRisk::Medium);

        let score = autonomy.trust_score(&domain).unwrap();
        assert_eq!(score.history.len(), 2);
        assert!(score.history[0].delta > 0.0); // earn
        assert!(score.history[1].delta < 0.0); // penalize
    }

    #[test]
    fn test_autonomy_level_thresholds() {
        assert_eq!(AutonomyLevel::Observer.required_trust(), 0.0);
        assert_eq!(AutonomyLevel::Apprentice.required_trust(), 0.2);
        assert_eq!(AutonomyLevel::Assistant.required_trust(), 0.4);
        assert_eq!(AutonomyLevel::Partner.required_trust(), 0.7);
        assert_eq!(AutonomyLevel::Autonomous.required_trust(), 0.9);
    }

    #[test]
    fn test_success_rate_tracking() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        autonomy.record_success(&domain, ActionRisk::Low);
        autonomy.record_success(&domain, ActionRisk::Low);
        autonomy.record_failure(&domain, ActionRisk::Low);

        let score = autonomy.trust_score(&domain).unwrap();
        assert!((score.success_rate() - 2.0 / 3.0).abs() < 0.01);
    }
}
