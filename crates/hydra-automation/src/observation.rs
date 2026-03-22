//! ExecutionObservation — one recorded execution.
//! Every execution through hydra-executor is observed here.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One observed execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionObservation {
    pub id: String,
    pub action_id: String,
    pub intent: String,
    /// Normalized params (keys only — values abstracted for pattern matching).
    pub param_keys: Vec<String>,
    /// Actual params for genome seed generation.
    pub params: HashMap<String, String>,
    pub domain: String,
    pub duration_ms: u64,
    pub succeeded: bool,
    pub observed_at: chrono::DateTime<chrono::Utc>,
}

impl ExecutionObservation {
    pub fn new(
        action_id: impl Into<String>,
        intent: impl Into<String>,
        params: HashMap<String, String>,
        domain: impl Into<String>,
        duration_ms: u64,
        succeeded: bool,
    ) -> Self {
        let param_keys: Vec<String> = {
            let mut keys: Vec<String> = params.keys().cloned().collect();
            keys.sort();
            keys
        };
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action_id: action_id.into(),
            intent: intent.into(),
            param_keys,
            params,
            domain: domain.into(),
            duration_ms,
            succeeded,
            observed_at: chrono::Utc::now(),
        }
    }

    /// Signature for pattern grouping — action + sorted param keys.
    pub fn signature(&self) -> String {
        format!("{}::{}", self.action_id, self.param_keys.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_includes_action_and_params() {
        let mut p = HashMap::new();
        p.insert("env".into(), "staging".into());
        p.insert("tag".into(), "v1.0".into());
        let obs = ExecutionObservation::new(
            "deploy.run",
            "deploy to staging",
            p,
            "engineering",
            1000,
            true,
        );
        let sig = obs.signature();
        assert!(sig.contains("deploy.run"));
        assert!(sig.contains("env"));
        assert!(sig.contains("tag"));
    }

    #[test]
    fn param_keys_sorted() {
        let mut p = HashMap::new();
        p.insert("z_param".into(), "z".into());
        p.insert("a_param".into(), "a".into());
        let obs = ExecutionObservation::new("test.action", "test", p, "test", 100, true);
        assert_eq!(obs.param_keys[0], "a_param");
        assert_eq!(obs.param_keys[1], "z_param");
    }
}
