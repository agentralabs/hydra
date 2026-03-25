//! Evolution approval gate — queues proposals for owner review.
//! Hydra proposes, owner approves/rejects via /evolution TUI command.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Status of an evolution proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected { reason: String },
}

/// A queued evolution proposal awaiting owner approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionProposal {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub entries: usize,
    pub blast_radius: String,
    pub skill_path: String,
    pub proposed_at: DateTime<Utc>,
    pub status: ProposalStatus,
}

/// Check if a given blast radius level requires approval.
pub fn needs_approval(blast: &str, config: &super::config::GuardrailConfig) -> bool {
    let level = match blast.to_lowercase().as_str() {
        "contained" => 0u8,
        "visible" => 1,
        "irreversible" => 2,
        "catastrophic" => 3,
        _ => 1,
    };
    level >= config.approval_threshold()
}

/// Queue a proposal for owner review.
pub fn queue_proposal(proposal: &EvolutionProposal) {
    let dir = queue_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.json", proposal.id));
    match serde_json::to_string_pretty(proposal) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("hydra-guardrail: queue write failed: {e}");
            } else {
                eprintln!("hydra-guardrail: evolution queued: {} ({})", proposal.name, proposal.id);
            }
        }
        Err(e) => eprintln!("hydra-guardrail: serialize proposal failed: {e}"),
    }
}

/// Load all pending proposals from queue directory.
pub fn load_pending() -> Vec<EvolutionProposal> {
    let dir = queue_dir();
    let mut proposals = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(p) = serde_json::from_str::<EvolutionProposal>(&content) {
                        if matches!(p.status, ProposalStatus::Pending) {
                            proposals.push(p);
                        }
                    }
                }
            }
        }
    }
    proposals.sort_by(|a, b| a.proposed_at.cmp(&b.proposed_at));
    proposals
}

/// Approve a proposal by ID — moves to processed directory.
pub fn approve(id: &str) -> bool {
    process_proposal(id, ProposalStatus::Approved)
}

/// Reject a proposal by ID with reason.
pub fn reject(id: &str, reason: &str) -> bool {
    process_proposal(id, ProposalStatus::Rejected { reason: reason.into() })
}

fn process_proposal(id: &str, status: ProposalStatus) -> bool {
    let queue_path = queue_dir().join(format!("{id}.json"));
    if let Ok(content) = std::fs::read_to_string(&queue_path) {
        if let Ok(mut proposal) = serde_json::from_str::<EvolutionProposal>(&content) {
            proposal.status = status;
            let processed_dir = processed_dir();
            let _ = std::fs::create_dir_all(&processed_dir);
            let dest = processed_dir.join(format!("{id}.json"));
            if let Ok(json) = serde_json::to_string_pretty(&proposal) {
                let _ = std::fs::write(&dest, json);
            }
            let _ = std::fs::remove_file(&queue_path);
            return true;
        }
    }
    false
}

fn queue_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/guardrails/evolution-queue")
}
fn processed_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/guardrails/evolution-processed")
}
