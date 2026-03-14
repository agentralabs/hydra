//! Dream State — runs Living Knowledge Engine modules during idle/periodic intervals.
//! Called from phase_learn at periodic intervals (every 10/20/50 messages).
//!
//! Consolidates: metabolism, adversarial testing, temporal fabric,
//! inference engine, knowledge fusion, ecosystem monitor.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use hydra_native_state::operational_profile::ProfileBelief;

use super::super::loop_runner::CognitiveUpdate;

/// Lightweight Dream State tasks — run every 10 messages.
/// Metabolism + temporal recording + ecosystem check.
pub(crate) async fn run_light(
    beliefs: &[ProfileBelief],
    sisters: &SistersHandle,
    llm_config: &hydra_model::llm_config::LlmConfig,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    // Temporal fabric: record current belief state
    {
        let store_ref = crate::cognitive::temporal_fabric::temporal_store();
        if let Ok(mut store) = store_ref.lock() {
            for b in beliefs {
                store.record(&b.topic, &b.content, b.confidence, "active-profile");
            }
            if let Some(summary) = store.temporal_summary() {
                let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                    title: "Belief Trends".into(),
                    content: summary,
                });
            }
        }
    }

    // Knowledge metabolism: crystallize observations
    let report = crate::cognitive::knowledge_metabolism::metabolize(
        sisters, llm_config, 50,
    ).await;
    if report.crystallizations_created > 0 {
        let _ = tx.send(CognitiveUpdate::EvidenceMemory {
            title: "Knowledge Crystallized".into(),
            content: report.summary(),
        });
    }

    // Ecosystem health check
    let health = crate::cognitive::ecosystem_monitor::assess_health(beliefs);
    if let Some(section) = crate::cognitive::ecosystem_monitor::format_for_prompt(&health) {
        let _ = tx.send(CognitiveUpdate::EvidenceMemory {
            title: "Belief Ecosystem".into(),
            content: section,
        });
    }

    // Awareness mesh sweep — poll watchers for alerts
    let watchers = crate::knowledge::awareness_mesh::watchers_for_profile("dev");
    if !watchers.is_empty() {
        let alerts = crate::knowledge::awareness_mesh::run_sweep(&watchers, sisters).await;
        for alert in &alerts {
            let _ = tx.send(CognitiveUpdate::ProactiveAlert {
                title: alert.title.clone(),
                message: alert.detail.clone(),
                priority: format!("{:?}", alert.severity),
            });
        }
    }
}

/// Deep Dream State tasks — run every 20 messages.
/// Adversarial testing + inference engine + knowledge fusion.
pub(crate) async fn run_deep(
    beliefs: &[ProfileBelief],
    sisters: &SistersHandle,
    llm_config: &hydra_model::llm_config::LlmConfig,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    // Adversarial self-testing on high-confidence beliefs
    if beliefs.len() >= 3 {
        let report = crate::cognitive::adversarial_tester::test_beliefs(
            beliefs, sisters, llm_config, 3,
        ).await;
        if report.beliefs_tested > 0 {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Adversarial Testing".into(),
                content: report.summary(),
            });
        }
    }

    // Inference engine: generate new beliefs from pairs
    if beliefs.len() >= 5 {
        let (inferred, report) = crate::cognitive::inference_engine::run_inference(
            beliefs, llm_config, 5,
        ).await;
        if !inferred.is_empty() {
            for inf in &inferred {
                let content = format!(
                    "[inferred] {} (confidence: {:.0}%, from: {} × {})",
                    inf.content, inf.confidence * 100.0,
                    &inf.parent_a[..inf.parent_a.len().min(40)],
                    &inf.parent_b[..inf.parent_b.len().min(40)],
                );
                sisters.memory_workspace_add(&content, "inferred-beliefs").await;
            }
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Inference Engine".into(),
                content: report.summary(),
            });
        }
    }

    // Knowledge fusion: cross-domain insight generation
    if beliefs.len() >= 10 {
        let (_insights, report) = crate::cognitive::knowledge_fusion::fuse_domains(
            beliefs, sisters, llm_config, 3,
        ).await;
        if report.insights_generated > 0 {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Knowledge Fusion".into(),
                content: report.summary(),
            });
        }
    }

    // Knowledge hunter: fill identified gaps
    let gaps = crate::knowledge::knowledge_hunter::identify_gaps(beliefs);
    if !gaps.is_empty() {
        let (candidates, report) = crate::knowledge::knowledge_hunter::hunt(
            &gaps, sisters, llm_config, 5,
        ).await;
        if !candidates.is_empty() {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Knowledge Hunter".into(),
                content: report.summary(),
            });
        }
    }

    // Creative engine: generate novel ideas from belief intersections
    if beliefs.len() >= 5 {
        let ideas = crate::knowledge::creative_engine::generate_ideas(beliefs, "", 3);
        if !ideas.is_empty() {
            let _ = tx.send(CognitiveUpdate::DreamInsight {
                category: "creative".into(),
                description: crate::knowledge::creative_engine::format_ideas(&ideas),
                confidence: 0.5,
            });
        }
    }
}
