//! CausalFactor — one link in an attribution chain.
//! Each factor explains one layer of WHY something cost what it did.

use crate::cost::{CostClass, CostItem};
use serde::{Deserialize, Serialize};

/// The type of causal factor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CausalFactorType {
    /// A domain knowledge gap that required acquisition.
    KnowledgeGap { topic: String },
    /// A security threat triggered extra analysis.
    SecurityThreat {
        threat_name: String,
        severity: String,
    },
    /// A concurrent lock conflict caused rerouting.
    ConcurrencyConflict { resource: String },
    /// Missing credentials caused approach rerouting.
    MissingCredentials { target: String },
    /// An anti-pattern was detected, requiring mitigation.
    AntiPatternDetected { pattern_name: String },
    /// First-time operation in a new domain/system.
    FirstTimeOperation { domain: String },
    /// A configuration issue caused rerouting.
    ConfigurationIssue { detail: String },
    /// Upstream system failure caused rerouting.
    UpstreamFailure { system: String },
    /// Principal decision that incurred cost.
    PrincipalDecision { description: String },
    /// Unknown — attribution could not be determined.
    Unknown,
}

impl CausalFactorType {
    pub fn label(&self) -> String {
        match self {
            Self::KnowledgeGap { topic } => {
                let end = topic.len().min(30);
                format!("knowledge-gap:{}", &topic[..end])
            }
            Self::SecurityThreat { threat_name, .. } => format!("security:{}", threat_name),
            Self::ConcurrencyConflict { resource } => format!("concurrency:{}", resource),
            Self::MissingCredentials { target } => format!("missing-creds:{}", target),
            Self::AntiPatternDetected { pattern_name } => {
                format!("anti-pattern:{}", pattern_name)
            }
            Self::FirstTimeOperation { domain } => format!("first-time:{}", domain),
            Self::ConfigurationIssue { .. } => "config-issue".into(),
            Self::UpstreamFailure { system } => format!("upstream:{}", system),
            Self::PrincipalDecision { .. } => "principal-decision".into(),
            Self::Unknown => "unknown".into(),
        }
    }

    /// Is this factor potentially avoidable in future?
    pub fn is_avoidable(&self) -> bool {
        matches!(
            self,
            Self::ConcurrencyConflict { .. }
                | Self::MissingCredentials { .. }
                | Self::KnowledgeGap { .. }
                | Self::ConfigurationIssue { .. }
        )
    }

    /// Is this factor a one-time cost (not recurring)?
    pub fn is_one_time(&self) -> bool {
        matches!(self, Self::FirstTimeOperation { .. })
    }
}

/// One causal factor in an attribution chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalFactor {
    pub id: String,
    pub factor_type: CausalFactorType,
    pub cost_fraction: f64,
    pub description: String,
    pub recommendation: Option<String>,
}

impl CausalFactor {
    pub fn new(
        factor_type: CausalFactorType,
        cost_fraction: f64,
        description: impl Into<String>,
        recommendation: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            factor_type,
            cost_fraction: cost_fraction.clamp(0.0, 1.0),
            description: description.into(),
            recommendation,
        }
    }

    pub fn is_avoidable(&self) -> bool {
        self.factor_type.is_avoidable()
    }
    pub fn is_one_time(&self) -> bool {
        self.factor_type.is_one_time()
    }
}

/// Infer causal factors from attribution cost items.
pub fn infer_factors(costs: &[CostItem], total: f64, context: &str) -> Vec<CausalFactor> {
    let lower = context.to_lowercase();
    let mut factors = Vec::new();

    for item in costs {
        if total < 1e-10 {
            break;
        }
        let fraction = item.amount / total;

        match &item.class {
            CostClass::ReroutingOverhead { attempts } => {
                let (factor_type, recommendation) =
                    if lower.contains("lock") || lower.contains("concurrent") {
                        (
                            CausalFactorType::ConcurrencyConflict {
                                resource: "deployment-target".into(),
                            },
                            Some("Coordinate concurrent operations via hydra-scheduler.".into()),
                        )
                    } else if lower.contains("auth") || lower.contains("cred") {
                        (
                            CausalFactorType::MissingCredentials {
                                target: "deployment-target".into(),
                            },
                            Some("Pre-provision credentials before deployment window.".into()),
                        )
                    } else {
                        (
                            CausalFactorType::ConfigurationIssue {
                                detail: format!("{} reroutes required", attempts),
                            },
                            Some("Review configuration before next deployment.".into()),
                        )
                    };
                factors.push(CausalFactor::new(
                    factor_type,
                    fraction,
                    format!(
                        "{} approach reroute(s): {:.0}% of total cost",
                        attempts,
                        fraction * 100.0
                    ),
                    recommendation,
                ));
            }
            CostClass::KnowledgeAcquisition { topic } => {
                let is_first_time = lower.contains("new") || lower.contains("first");
                let factor_type = if is_first_time {
                    CausalFactorType::FirstTimeOperation {
                        domain: topic.clone(),
                    }
                } else {
                    CausalFactorType::KnowledgeGap {
                        topic: topic.clone(),
                    }
                };
                let recommendation = if !is_first_time {
                    Some(format!(
                        "Load a skill for '{}' domain to avoid future acquisition cost.",
                        topic
                    ))
                } else {
                    None
                };
                factors.push(CausalFactor::new(
                    factor_type,
                    fraction,
                    format!(
                        "Knowledge acquisition for '{}': {:.0}% of cost",
                        topic,
                        fraction * 100.0
                    ),
                    recommendation,
                ));
            }
            CostClass::RedTeamCost => {
                factors.push(CausalFactor::new(
                    CausalFactorType::SecurityThreat {
                        threat_name: "pre-execution-analysis".into(),
                        severity: "proactive".into(),
                    },
                    fraction,
                    format!(
                        "Red team analysis: {:.0}% of cost (constitutional requirement)",
                        fraction * 100.0
                    ),
                    None,
                ));
            }
            CostClass::SisterCallCost { sister_name } => {
                factors.push(CausalFactor::new(
                    CausalFactorType::PrincipalDecision {
                        description: format!("{} call required by task", sister_name),
                    },
                    fraction,
                    format!(
                        "{} call: {:.0}% of cost (task requirement)",
                        sister_name,
                        fraction * 100.0
                    ),
                    None,
                ));
            }
            _ => {
                factors.push(CausalFactor::new(
                    CausalFactorType::PrincipalDecision {
                        description: "core task execution".into(),
                    },
                    fraction,
                    format!("Core execution: {:.0}% of cost", fraction * 100.0),
                    None,
                ));
            }
        }
    }

    // Sort by cost fraction descending
    factors.sort_by(|a, b| {
        b.cost_fraction
            .partial_cmp(&a.cost_fraction)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    factors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rerouting_inferred_as_concurrency_for_lock_context() {
        let costs = vec![CostItem::new(
            CostClass::ReroutingOverhead { attempts: 2 },
            0,
            0.5,
            1000,
        )];
        let total = costs.iter().map(|c| c.amount).sum();
        let factors = infer_factors(&costs, total, "deployment lock concurrent access");
        assert!(factors
            .iter()
            .any(|f| matches!(f.factor_type, CausalFactorType::ConcurrencyConflict { .. })));
    }

    #[test]
    fn avoidable_factors_flagged() {
        let f = CausalFactor::new(
            CausalFactorType::ConcurrencyConflict {
                resource: "r".into(),
            },
            0.3,
            "test",
            None,
        );
        assert!(f.is_avoidable());
    }

    #[test]
    fn first_time_is_not_avoidable() {
        let f = CausalFactor::new(
            CausalFactorType::FirstTimeOperation {
                domain: "gcp".into(),
            },
            0.2,
            "test",
            None,
        );
        assert!(!f.is_avoidable());
        assert!(f.is_one_time());
    }

    #[test]
    fn factors_sorted_by_fraction_descending() {
        let costs = vec![
            CostItem::new(CostClass::DirectExecution, 1000, 2.0, 2000),
            CostItem::new(CostClass::ReroutingOverhead { attempts: 2 }, 0, 0.5, 500),
            CostItem::new(
                CostClass::SisterCallCost {
                    sister_name: "AgenticMemory".into(),
                },
                200,
                0.2,
                200,
            ),
        ];
        let total: f64 = costs.iter().map(|c| c.amount).sum();
        let factors = infer_factors(&costs, total, "deploy service");
        for w in factors.windows(2) {
            assert!(w[0].cost_fraction >= w[1].cost_fraction);
        }
    }
}
