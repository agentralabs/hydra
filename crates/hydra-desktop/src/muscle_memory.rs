//! Layer 6: Muscle Memory — crystallized action sequences in genome.
//!
//! Every successful action sequence is stored. After repeated success,
//! it becomes CRYSTALLIZED — replayed instantly without vision or LLM.
//! Like how you type your password without looking at the keyboard.

use serde::{Serialize, Deserialize};

/// A single UI action primitive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiPrimitive {
    /// Click at absolute coordinates.
    ClickAt { x: f64, y: f64 },
    /// Click an element by role + label (resolved via a11y/OCR at replay time).
    ClickElement { role: String, label: String },
    /// Press a single key.
    KeyPress { key: String },
    /// Key combination (e.g., cmd+s).
    KeyCombo { modifier: String, key: String },
    /// Type text character by character.
    TypeText { text: String },
    /// Navigate a menu path (e.g., ["File", "Export"]).
    MenuNavigate { path: Vec<String> },
    /// Switch to a named tool (resolved via AMM).
    SwitchTool { tool: String },
    /// Wait for a condition (text appears, timeout).
    WaitFor { condition: String, timeout_ms: u64 },
    /// Drag from point A to point B.
    Drag { x1: f64, y1: f64, x2: f64, y2: f64 },
    /// Scroll wheel at position.
    ScrollWheel { x: f64, y: f64, dy: i32 },
    /// Click with modifier held.
    ModifierClick { x: f64, y: f64, modifier: String },
    /// Drag with modifier held.
    ModifierDrag { x1: f64, y1: f64, x2: f64, y2: f64, modifier: String },
    /// Write to clipboard and paste.
    PasteText { text: String },
    /// Wait for screen to stabilize.
    WaitForStable { timeout_ms: u64 },
}

/// A stored muscle memory sequence for an app + goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuscleMemory {
    pub app: String,
    pub goal: String,
    pub goal_hash: u64,
    pub steps: Vec<UiPrimitive>,
    pub confidence: f64,
    pub replays: u32,
    pub last_used: String,
}

impl MuscleMemory {
    /// Is this sequence reliable enough to replay without vision?
    pub fn is_crystallized(&self) -> bool {
        self.confidence > 0.95 && self.replays >= 5
    }

    /// Record a successful replay — increases confidence toward crystallization.
    pub fn record_success(&mut self) {
        self.replays += 1;
        self.confidence = (self.confidence + 0.05).min(1.0);
        self.last_used = chrono::Utc::now().to_rfc3339();
    }

    /// Record a failed replay — decreases confidence, may un-crystallize.
    pub fn record_failure(&mut self) {
        self.confidence = (self.confidence - 0.2).max(0.0);
    }

    /// Try to recall a muscle memory for this app + goal from genome.
    pub fn recall(app: &str, goal: &str, genome: &hydra_genome::GenomeStore) -> Option<Self> {
        let query = format!("muscle_memory:{app}:{goal}");
        let results = genome.query(&query);
        results.first().and_then(|entry| {
            serde_json::from_str(&entry.approach.steps.join("\n")).ok()
        })
    }

    /// Store this muscle memory into genome.
    pub fn store(&self, genome: &mut hydra_genome::GenomeStore) {
        let tag = format!("muscle_memory:{}:{}", self.app, self.goal);
        let json = serde_json::to_string(self).unwrap_or_default();
        if let Err(e) = genome.add_from_operation(
            &tag,
            hydra_genome::ApproachSignature {
                approach_type: "muscle_memory".into(),
                steps: vec![json],
                tools_used: vec!["amm".into(), "ui_primitive".into()],
            },
            self.confidence,
        ) {
            eprintln!("hydra-muscle: store failed: {e}");
        } else {
            eprintln!("hydra-muscle: stored '{}' for '{}' (conf={:.2}, replays={})",
                self.goal, self.app, self.confidence, self.replays);
        }
    }

    /// Create a new muscle memory from a successful action sequence.
    pub fn from_success(app: &str, goal: &str, steps: Vec<UiPrimitive>) -> Self {
        Self {
            app: app.into(), goal: goal.into(),
            goal_hash: hash_goal(goal),
            steps, confidence: 0.3,
            replays: 1,
            last_used: chrono::Utc::now().to_rfc3339(),
        }
    }
}

fn hash_goal(goal: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    goal.to_lowercase().hash(&mut h);
    h.finish()
}
