//! O28: Consequence Prediction Engine — application state machine.
//!
//! Before executing any action, PREDICTS what the screen will look like.
//! After executing, compares prediction vs reality and LEARNS.
//! Over time, builds a complete state machine for each application.
//! After 100 uses, Hydra navigates the app like a blind pianist plays piano.

use std::collections::HashMap;
use std::path::PathBuf;

/// A state the application can be in (e.g., "idle", "file_dialog", "drawing_line").
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppState {
    pub name: String,
    pub indicators: Vec<String>,
    pub available_actions: Vec<String>,
}

/// A transition between states triggered by an action.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateTransition {
    pub from_state: String,
    pub action: String,
    pub to_state: String,
    pub confidence: f64,
    pub observations: u32,
}

/// Prediction of what will happen after an action.
#[derive(Debug, Clone)]
pub struct StatePrediction {
    pub predicted_state: String,
    pub expected_changes: Vec<String>,
    pub confidence: f64,
}

/// Complete state machine for an application. Grows from observation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppStateGraph {
    pub app: String,
    pub states: HashMap<String, AppState>,
    pub transitions: Vec<StateTransition>,
    pub current_state: String,
}

impl AppStateGraph {
    /// Create an empty graph for a new app.
    pub fn new(app: &str) -> Self {
        let mut states = HashMap::new();
        states.insert("idle".into(), AppState {
            name: "idle".into(), indicators: vec![], available_actions: vec![],
        });
        Self { app: app.into(), states, transitions: Vec::new(), current_state: "idle".into() }
    }

    /// Predict the next state after performing an action.
    pub fn predict(&self, action: &str) -> Option<StatePrediction> {
        let action_lower = action.to_lowercase();
        let matching: Vec<&StateTransition> = self.transitions.iter()
            .filter(|t| t.from_state == self.current_state && t.action.to_lowercase() == action_lower)
            .collect();
        if matching.is_empty() { return None; }
        // Pick highest-confidence transition
        let best = matching.iter().max_by(|a, b|
            a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))?;
        let expected = self.states.get(&best.to_state)
            .map(|s| s.indicators.clone()).unwrap_or_default();
        Some(StatePrediction {
            predicted_state: best.to_state.clone(),
            expected_changes: expected,
            confidence: best.confidence,
        })
    }

    /// Record an observed transition. Updates confidence or creates new transition.
    pub fn observe_transition(&mut self, action: &str, actual_state: &str) {
        let action_lower = action.to_lowercase();
        // Find existing transition
        if let Some(t) = self.transitions.iter_mut().find(|t|
            t.from_state == self.current_state && t.action.to_lowercase() == action_lower
            && t.to_state == actual_state) {
            t.observations += 1;
            t.confidence = (t.confidence + 0.05).min(1.0);
        } else {
            // Check if there's a WRONG prediction (same from+action, different to)
            if let Some(wrong) = self.transitions.iter_mut().find(|t|
                t.from_state == self.current_state && t.action.to_lowercase() == action_lower
                && t.to_state != actual_state) {
                wrong.confidence = (wrong.confidence - 0.1).max(0.0);
            }
            // Add new transition
            self.transitions.push(StateTransition {
                from_state: self.current_state.clone(),
                action: action_lower, to_state: actual_state.into(),
                confidence: 0.5, observations: 1,
            });
        }
        // Ensure state exists
        if !self.states.contains_key(actual_state) {
            self.states.insert(actual_state.into(), AppState {
                name: actual_state.into(), indicators: vec![], available_actions: vec![],
            });
        }
        self.current_state = actual_state.into();
    }

    /// Check if a prediction was correct.
    pub fn verify_prediction(&self, prediction: &StatePrediction, actual_state: &str) -> bool {
        prediction.predicted_state == actual_state
    }

    /// Get the number of known transitions (measure of app understanding).
    pub fn knowledge_level(&self) -> usize { self.transitions.len() }

    /// Load state graph from disk.
    pub fn load(app: &str) -> Option<Self> {
        let path = state_graph_dir().join(format!("{}.toml", sanitize(app)));
        let content = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Save state graph to disk.
    pub fn save(&self) {
        let dir = state_graph_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.toml", sanitize(&self.app)));
        match toml::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    eprintln!("hydra-state-graph: save failed: {e}");
                }
            }
            Err(e) => eprintln!("hydra-state-graph: serialize failed: {e}"),
        }
    }
}

fn state_graph_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/app_states")
}

fn sanitize(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predict_after_observation() {
        let mut graph = AppStateGraph::new("test");
        graph.observe_transition("cmd+o", "open_dialog");
        graph.current_state = "idle".into();
        let pred = graph.predict("cmd+o").unwrap();
        assert_eq!(pred.predicted_state, "open_dialog");
        assert!(pred.confidence >= 0.5);
    }

    #[test]
    fn confidence_grows_with_repetition() {
        let mut graph = AppStateGraph::new("test");
        for _ in 0..5 {
            graph.current_state = "idle".into();
            graph.observe_transition("cmd+s", "saved");
            graph.current_state = "idle".into();
        }
        let t = graph.transitions.iter().find(|t| t.action == "cmd+s").unwrap();
        assert!(t.confidence > 0.7);
        assert_eq!(t.observations, 5);
    }
}
