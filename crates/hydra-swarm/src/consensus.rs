//! Consensus detection using Jaccard similarity grouping.

use crate::constants::{CONSENSUS_MIN_AGENTS, CONSENSUS_SIMILARITY_THRESHOLD};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// An answer from a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnswer {
    /// Which agent provided this answer.
    pub agent_id: Uuid,
    /// The answer tokens (words).
    pub tokens: Vec<String>,
}

impl AgentAnswer {
    /// Create a new agent answer from a text string (split on whitespace).
    pub fn from_text(agent_id: Uuid, text: &str) -> Self {
        let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_lowercase()).collect();
        Self { agent_id, tokens }
    }
}

/// The consensus signal produced by detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusSignal {
    /// Whether consensus was reached.
    pub reached: bool,
    /// How many agents agreed.
    pub agreeing_count: usize,
    /// Total agents participating.
    pub total_count: usize,
    /// The average similarity score within the agreeing group.
    pub similarity: f64,
}

impl ConsensusSignal {
    /// Return true if the consensus signal is strong
    /// (majority agrees and similarity is above threshold).
    pub fn is_strong(&self) -> bool {
        self.reached && self.similarity >= CONSENSUS_SIMILARITY_THRESHOLD
    }
}

/// Compute Jaccard similarity between two token sets.
fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
    let set_a: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

/// Detect consensus among a set of agent answers.
///
/// Groups answers by Jaccard similarity. If the largest group has
/// at least CONSENSUS_MIN_AGENTS members, consensus is reached.
pub fn detect_consensus(answers: &[AgentAnswer]) -> ConsensusSignal {
    if answers.len() < CONSENSUS_MIN_AGENTS {
        return ConsensusSignal {
            reached: false,
            agreeing_count: answers.len(),
            total_count: answers.len(),
            similarity: 0.0,
        };
    }

    // Greedy grouping: assign each answer to the first group it's similar to
    let mut groups: Vec<Vec<usize>> = Vec::new();

    for i in 0..answers.len() {
        let mut placed = false;
        for group in &mut groups {
            let representative = group[0];
            let sim = jaccard_similarity(&answers[representative].tokens, &answers[i].tokens);
            if sim >= CONSENSUS_SIMILARITY_THRESHOLD {
                group.push(i);
                placed = true;
                break;
            }
        }
        if !placed {
            groups.push(vec![i]);
        }
    }

    // Find the largest group
    let largest = groups.iter().max_by_key(|g| g.len());

    match largest {
        Some(group) if group.len() >= CONSENSUS_MIN_AGENTS => {
            // Compute average pairwise similarity within the group
            let mut total_sim = 0.0;
            let mut pair_count = 0u64;
            for (idx_i, &i) in group.iter().enumerate() {
                for &j in group.iter().skip(idx_i + 1) {
                    total_sim += jaccard_similarity(&answers[i].tokens, &answers[j].tokens);
                    pair_count += 1;
                }
            }
            let avg_sim = if pair_count > 0 {
                total_sim / pair_count as f64
            } else {
                1.0
            };

            ConsensusSignal {
                reached: true,
                agreeing_count: group.len(),
                total_count: answers.len(),
                similarity: avg_sim,
            }
        }
        _ => ConsensusSignal {
            reached: false,
            agreeing_count: 1,
            total_count: answers.len(),
            similarity: 0.0,
        },
    }
}
