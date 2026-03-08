//! ContractEnforcer — runtime enforcement of behavioral contracts.

use serde::{Deserialize, Serialize};

use super::contract::{BehavioralContract, ClauseType, ContractStatus};

/// A contract violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub contract_id: String,
    pub clause_id: String,
    pub description: String,
    pub severity: ViolationSeverity,
    pub timestamp: String,
}

/// Severity of a violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Warning,
    Minor,
    Major,
    Critical,
}

/// Result of enforcement check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementResult {
    pub contract_id: String,
    pub compliant: bool,
    pub violations: Vec<Violation>,
    pub checked_clauses: usize,
}

/// Enforces behavioral contracts at runtime
pub struct ContractEnforcer {
    violations: parking_lot::RwLock<Vec<Violation>>,
    checks: parking_lot::RwLock<Vec<EnforcementResult>>,
}

impl ContractEnforcer {
    pub fn new() -> Self {
        Self {
            violations: parking_lot::RwLock::new(Vec::new()),
            checks: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Check if an action is allowed under a contract
    pub fn check_action(
        &self,
        contract: &BehavioralContract,
        action_description: &str,
    ) -> EnforcementResult {
        if contract.status != ContractStatus::Active {
            return EnforcementResult {
                contract_id: contract.id.clone(),
                compliant: true,
                violations: Vec::new(),
                checked_clauses: 0,
            };
        }

        let mut violations = Vec::new();
        let action_lower = action_description.to_lowercase();

        for clause in &contract.clauses {
            match clause.clause_type {
                ClauseType::Prohibition => {
                    // Check if action matches a prohibited pattern
                    let clause_lower = clause.description.to_lowercase();
                    let clause_keywords: Vec<&str> =
                        clause_lower.split_whitespace().collect::<Vec<_>>();
                    let significant_keywords: Vec<&&str> = clause_keywords
                        .iter()
                        .filter(|w| w.len() > 3)
                        .collect();

                    let matches = significant_keywords
                        .iter()
                        .filter(|kw| action_lower.contains(**kw))
                        .count();

                    if matches >= 2 {
                        violations.push(Violation {
                            contract_id: contract.id.clone(),
                            clause_id: clause.id.clone(),
                            description: format!(
                                "Action '{}' may violate prohibition: {}",
                                action_description, clause.description
                            ),
                            severity: if clause.binding {
                                ViolationSeverity::Critical
                            } else {
                                ViolationSeverity::Warning
                            },
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        });
                    }
                }
                _ => {} // Obligations/Permissions/Guarantees checked differently
            }
        }

        let compliant = violations.is_empty();
        self.violations.write().extend(violations.clone());

        let result = EnforcementResult {
            contract_id: contract.id.clone(),
            compliant,
            violations,
            checked_clauses: contract.clauses.len(),
        };

        self.checks.write().push(result.clone());
        result
    }

    /// Get all violations
    pub fn all_violations(&self) -> Vec<Violation> {
        self.violations.read().clone()
    }

    /// Get violation count
    pub fn violation_count(&self) -> usize {
        self.violations.read().len()
    }

    /// Get compliance rate
    pub fn compliance_rate(&self) -> f64 {
        let checks = self.checks.read();
        if checks.is_empty() {
            return 1.0;
        }
        let compliant = checks.iter().filter(|c| c.compliant).count();
        compliant as f64 / checks.len() as f64
    }
}

impl Default for ContractEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safety_contract() -> BehavioralContract {
        let mut contract = BehavioralContract::new("safety", "Safety contract");
        contract.add_clause(
            "Never delete files without user confirmation",
            ClauseType::Prohibition,
            true,
        );
        contract.add_clause(
            "Always create backup before modifying files",
            ClauseType::Obligation,
            true,
        );
        contract
    }

    #[test]
    fn test_compliant_action() {
        let enforcer = ContractEnforcer::new();
        let contract = safety_contract();

        let result = enforcer.check_action(&contract, "read file contents");
        assert!(result.compliant);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_violation_detection() {
        let enforcer = ContractEnforcer::new();
        let contract = safety_contract();

        let result = enforcer.check_action(
            &contract,
            "delete files without confirmation from user",
        );
        assert!(!result.compliant);
        assert!(!result.violations.is_empty());
    }

    #[test]
    fn test_compliance_rate() {
        let enforcer = ContractEnforcer::new();
        let contract = safety_contract();

        enforcer.check_action(&contract, "read file");
        enforcer.check_action(&contract, "write file");
        enforcer.check_action(&contract, "delete files without confirmation");

        let rate = enforcer.compliance_rate();
        assert!(rate > 0.0 && rate < 1.0);
    }

    #[test]
    fn test_inactive_contract_skipped() {
        let enforcer = ContractEnforcer::new();
        let mut contract = safety_contract();
        contract.status = ContractStatus::Expired;

        let result = enforcer.check_action(&contract, "delete everything");
        assert!(result.compliant);
        assert_eq!(result.checked_clauses, 0);
    }
}
