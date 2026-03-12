//! BehavioralContract — promise types for Hydra actions.
//!
//! Contracts define what Hydra promises to do and not do,
//! with runtime enforcement and violation detection.

use serde::{Deserialize, Serialize};

/// Status of a contract
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractStatus {
    Active,
    Fulfilled,
    Violated,
    Expired,
    Suspended,
}

/// A clause within a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractClause {
    pub id: String,
    pub description: String,
    pub clause_type: ClauseType,
    pub binding: bool,
}

/// Type of clause
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClauseType {
    /// Must do this
    Obligation,
    /// Must not do this
    Prohibition,
    /// Allowed to do this
    Permission,
    /// Will produce this result
    Guarantee,
}

/// A promise attached to a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Promise {
    pub id: String,
    pub description: String,
    pub fulfilled: bool,
    pub deadline: Option<String>,
}

impl Promise {
    pub fn new(description: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.into(),
            fulfilled: false,
            deadline: None,
        }
    }

    pub fn with_deadline(mut self, deadline: &str) -> Self {
        self.deadline = Some(deadline.into());
        self
    }

    pub fn fulfill(&mut self) {
        self.fulfilled = true;
    }
}

/// A behavioral contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralContract {
    pub id: String,
    pub name: String,
    pub description: String,
    pub clauses: Vec<ContractClause>,
    pub promises: Vec<Promise>,
    pub status: ContractStatus,
    pub created_at: String,
    pub expires_at: Option<String>,
}

impl BehavioralContract {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            clauses: Vec::new(),
            promises: Vec::new(),
            status: ContractStatus::Active,
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at: None,
        }
    }

    /// Add a clause to the contract
    pub fn add_clause(&mut self, description: &str, clause_type: ClauseType, binding: bool) {
        self.clauses.push(ContractClause {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.into(),
            clause_type,
            binding,
        });
    }

    /// Add a promise
    pub fn add_promise(&mut self, promise: Promise) {
        self.promises.push(promise);
    }

    /// Check if all promises are fulfilled
    pub fn all_promises_fulfilled(&self) -> bool {
        !self.promises.is_empty() && self.promises.iter().all(|p| p.fulfilled)
    }

    /// Fulfill a promise by ID
    pub fn fulfill_promise(&mut self, promise_id: &str) -> bool {
        if let Some(p) = self.promises.iter_mut().find(|p| p.id == promise_id) {
            p.fulfill();

            // Check if contract is now fully fulfilled
            if self.all_promises_fulfilled() {
                self.status = ContractStatus::Fulfilled;
            }
            true
        } else {
            false
        }
    }

    /// Mark the contract as violated
    pub fn violate(&mut self) {
        self.status = ContractStatus::Violated;
    }

    /// Get binding clauses
    pub fn binding_clauses(&self) -> Vec<&ContractClause> {
        self.clauses.iter().filter(|c| c.binding).collect()
    }

    /// Get obligations
    pub fn obligations(&self) -> Vec<&ContractClause> {
        self.clauses
            .iter()
            .filter(|c| c.clause_type == ClauseType::Obligation)
            .collect()
    }

    /// Get prohibitions
    pub fn prohibitions(&self) -> Vec<&ContractClause> {
        self.clauses
            .iter()
            .filter(|c| c.clause_type == ClauseType::Prohibition)
            .collect()
    }
}

/// Store for contracts
pub struct ContractStore {
    contracts: parking_lot::RwLock<Vec<BehavioralContract>>,
}

impl ContractStore {
    pub fn new() -> Self {
        Self {
            contracts: parking_lot::RwLock::new(Vec::new()),
        }
    }

    pub fn add(&self, contract: BehavioralContract) -> String {
        let id = contract.id.clone();
        self.contracts.write().push(contract);
        id
    }

    pub fn get(&self, id: &str) -> Option<BehavioralContract> {
        self.contracts.read().iter().find(|c| c.id == id).cloned()
    }

    pub fn active_contracts(&self) -> Vec<BehavioralContract> {
        self.contracts
            .read()
            .iter()
            .filter(|c| c.status == ContractStatus::Active)
            .cloned()
            .collect()
    }

    pub fn count(&self) -> usize {
        self.contracts.read().len()
    }
}

impl Default for ContractStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_creation() {
        let mut contract = BehavioralContract::new("safety", "Safety guarantees");
        contract.add_clause(
            "Never delete files without confirmation",
            ClauseType::Prohibition,
            true,
        );
        contract.add_clause(
            "Always create backups before modification",
            ClauseType::Obligation,
            true,
        );

        assert_eq!(contract.clauses.len(), 2);
        assert_eq!(contract.binding_clauses().len(), 2);
        assert_eq!(contract.prohibitions().len(), 1);
        assert_eq!(contract.obligations().len(), 1);
    }

    #[test]
    fn test_promise_fulfillment() {
        let mut contract = BehavioralContract::new("delivery", "Delivery contract");
        let promise = Promise::new("Deliver results within 5 seconds");
        let promise_id = promise.id.clone();
        contract.add_promise(promise);

        assert!(!contract.all_promises_fulfilled());
        assert!(contract.fulfill_promise(&promise_id));
        assert!(contract.all_promises_fulfilled());
        assert_eq!(contract.status, ContractStatus::Fulfilled);
    }

    #[test]
    fn test_contract_violation() {
        let mut contract = BehavioralContract::new("test", "Test contract");
        contract.add_clause("Must not fail", ClauseType::Prohibition, true);

        assert_eq!(contract.status, ContractStatus::Active);
        contract.violate();
        assert_eq!(contract.status, ContractStatus::Violated);
    }

    #[test]
    fn test_contract_store() {
        let store = ContractStore::new();
        let contract = BehavioralContract::new("test", "Test");
        let id = store.add(contract);

        assert_eq!(store.count(), 1);
        assert!(store.get(&id).is_some());
        assert_eq!(store.active_contracts().len(), 1);
    }
}
