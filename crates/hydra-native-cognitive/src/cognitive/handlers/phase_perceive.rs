//! PERCEIVE phase — extracted from loop_runner.rs for compilation performance.
//!
//! Queries sisters for context, retrieves memories, loads beliefs,
//! discovers MCP tools, and loads federation state.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cognitive::inventions::InventionEngine;
use crate::sisters::SistersHandle;
use hydra_native_state::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use hydra_native_state::utils::generate_deliverable_steps;
use hydra_db::{HydraDb, McpDiscoveredSkillRow, FederationStateRow};

use super::super::loop_runner::CognitiveUpdate;
use super::memory::extract_memory_facts;

/// Output of the PERCEIVE phase, consumed by THINK.
pub(crate) struct PerceiveResult {
    pub perceived: serde_json::Value,
    pub always_on_memory: Option<String>,
    pub beliefs_context: Option<String>,
    pub federation_context: Option<String>,
    pub perceive_ms: u64,
}

/// Run the PERCEIVE phase: gather context from sisters, memory, beliefs, MCP, federation.
pub(crate) async fn run_perceive(
    text: &str,
    is_simple: bool,
    is_complex: bool,
    sisters_handle: &Option<SistersHandle>,
    inventions: &Option<Arc<InventionEngine>>,
    proactive_notifier: &Option<Arc<parking_lot::Mutex<hydra_native_state::proactive::ProactiveNotifier>>>,
    federation: &Option<Arc<hydra_native_state::federation::FederationManager>>,
    db: &Option<Arc<HydraDb>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> PerceiveResult {
    use std::time::Instant;

    let _ = tx.send(CognitiveUpdate::Phase("Perceive".into()));
    let _ = tx.send(CognitiveUpdate::IconState("listening".into()));
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));
    let perceive_start = Instant::now();

    // Surface dream insights from idle processing
    if let Some(ref inv) = inventions {
        inv.tick_idle(0);
        inv.reset_idle();
        if let Some(insights) = inv.surface_insights(0.6) {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Dream Insights".to_string(),
                content: insights,
            });
        }
        if let Some(dream_text) = inv.maybe_dream() {
            let _ = tx.send(CognitiveUpdate::DreamInsight {
                category: "idle_processing".to_string(),
                description: dream_text.clone(),
                confidence: 0.7,
            });
        }
    }

    // Setup workspace panels based on complexity
    if is_simple {
        let _ = tx.send(CognitiveUpdate::PlanClear);
        let _ = tx.send(CognitiveUpdate::TimelineClear);
        let _ = tx.send(CognitiveUpdate::EvidenceClear);
    } else {
        let steps = generate_deliverable_steps(text);
        let _ = tx.send(CognitiveUpdate::PlanInit {
            goal: text.to_string(),
            steps: steps.clone(),
        });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(0));
        let _ = tx.send(CognitiveUpdate::TimelineClear);
        let _ = tx.send(CognitiveUpdate::EvidenceClear);
    }

    // Query sisters for perceived context
    let perceived = if let Some(ref sh) = sisters_handle {
        if is_simple {
            eprintln!("[hydra:perceive] SIMPLE mode — memory + cognition only");
            sh.perceive_simple(text).await
        } else {
            eprintln!("[hydra:perceive] COMPLEX mode — all sisters");
            sh.perceive(text).await
        }
    } else {
        serde_json::json!({
            "input": text,
            "involves_code": false,
            "involves_vision": false,
        })
    };

    // Always-on memory retrieval
    let always_on_memory = if let Some(ref sh) = sisters_handle {
        if let Some(ref mem) = sh.memory {
            let mem_limit = if is_simple { 3 } else { 8 };
            let mem_result = mem.call_tool("memory_query", serde_json::json!({
                "query": text,
                "max_results": mem_limit,
                "sort_by": "highest_confidence"
            })).await;
            match mem_result {
                Ok(v) => {
                    let raw = crate::sisters::extract_text(&v);
                    if !raw.is_empty() && !raw.contains("No memories found") {
                        let facts = extract_memory_facts(&raw);
                        if !facts.is_empty() {
                            let input_lower = text.to_lowercase();
                            let input_words: Vec<&str> = input_lower.split_whitespace()
                                .filter(|w| w.len() >= 3)
                                .collect();
                            let mut scored_facts: Vec<(usize, &String)> = facts.iter()
                                .map(|f| {
                                    let f_lower = f.to_lowercase();
                                    let score = input_words.iter()
                                        .filter(|w| f_lower.contains(*w))
                                        .count();
                                    (score, f)
                                })
                                .collect();
                            scored_facts.sort_by(|a, b| b.0.cmp(&a.0));
                            let sorted_facts: Vec<String> = scored_facts.iter()
                                .map(|(_, f)| (*f).clone())
                                .collect();
                            eprintln!("[hydra:perceive] Always-on memory: {} facts retrieved (sorted by relevance)", sorted_facts.len());
                            Some(sorted_facts.join("\n"))
                        } else { None }
                    } else { None }
                }
                Err(_) => None,
            }
        } else { None }
    } else { None };

    // Belief loading from DB
    let belief_limit = if is_simple { 5 } else { 20 };
    let beliefs_context = if let Some(ref db) = db {
        match db.get_active_beliefs(belief_limit) {
            Ok(beliefs) if !beliefs.is_empty() => {
                let summary: String = beliefs.iter()
                    .map(|b| format!("- {} [{}]: {} (confidence: {:.0}%)", b.subject, b.category, b.content, b.confidence * 100.0))
                    .collect::<Vec<_>>()
                    .join("\n");
                let _ = tx.send(CognitiveUpdate::BeliefsLoaded {
                    count: beliefs.len(),
                    summary: summary.clone(),
                });
                Some(summary)
            }
            _ => None,
        }
    } else { None };

    // MCP skill discovery (complex only)
    let _mcp_context = if !is_complex {
        None
    } else if let Some(ref sh) = sisters_handle {
        let tools = sh.discover_mcp_tools();
        if !tools.is_empty() {
            if let Some(ref db) = db {
                let now = chrono::Utc::now().to_rfc3339();
                let mut servers_seen: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
                for (server, tool_name) in &tools {
                    servers_seen.entry(server.clone()).or_default().push(tool_name.clone());
                    let skill_id = format!("mcp-{}-{}", server.to_lowercase(), tool_name);
                    let _ = db.upsert_mcp_skill(&McpDiscoveredSkillRow {
                        id: skill_id,
                        server_name: server.clone(),
                        tool_name: tool_name.clone(),
                        description: None,
                        input_schema: None,
                        discovered_at: now.clone(),
                        last_used_at: None,
                        use_count: 0,
                        active: true,
                    });
                }
                for (server, tool_names) in &servers_seen {
                    let _ = tx.send(CognitiveUpdate::McpSkillsDiscovered {
                        server: server.clone(),
                        tools: tool_names.clone(),
                        count: tool_names.len(),
                    });
                }
            }
            let mut by_server: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
            for (server, tool_name) in &tools {
                by_server.entry(server).or_default().push(tool_name);
            }
            let summary: String = by_server.iter()
                .map(|(server, tls)| format!("- {} ({} tools): {}", server, tls.len(), tls.join(", ")))
                .collect::<Vec<_>>()
                .join("\n");
            Some(summary)
        } else { None }
    } else { None };

    // Federation context (complex only)
    let federation_context = if !is_complex {
        None
    } else if let Some(ref fed) = federation {
        if fed.is_enabled() {
            let peer_count = fed.peer_count();
            let available = fed.registry.available_peers().len();
            let federation_state = fed.sync.version();
            if let Some(ref db) = db {
                for peer in fed.registry.list() {
                    let _ = db.upsert_federation_peer(&FederationStateRow {
                        peer_id: peer.id.clone(),
                        peer_name: Some(peer.name.clone()),
                        endpoint: peer.endpoint.clone(),
                        trust_level: format!("{:?}", peer.trust_level),
                        capabilities: Some(serde_json::to_string(&peer.capabilities.sisters).unwrap_or_default()),
                        federation_type: format!("{:?}", peer.federation_type),
                        last_sync_version: 0,
                        last_seen: peer.last_seen.clone(),
                        active_tasks: peer.active_tasks as i64,
                        active: true,
                    });
                }
            }
            let _ = tx.send(CognitiveUpdate::FederationSync {
                peers_online: peer_count,
                last_sync_version: federation_state as i64,
            });
            if peer_count > 0 {
                Some(format!("Federation: {} peers registered, {} available for delegation (sync v{})", peer_count, available, federation_state))
            } else { None }
        } else { None }
    } else { None };

    let perceive_ms = perceive_start.elapsed().as_millis() as u64;

    // Proactive: anticipate needs
    if let Some(ref notifier) = proactive_notifier {
        let mut n = notifier.lock();
        n.anticipate(text);
        for alert in n.drain() {
            let _ = tx.send(CognitiveUpdate::ProactiveAlert {
                title: alert.title,
                message: alert.message,
                priority: format!("{:?}", alert.priority),
            });
        }
    }

    // Add perceived context as evidence (complex tasks only)
    if !is_simple {
        if let Some(mem) = perceived["memory_context"].as_str() {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Relevant memories".into(),
                content: mem.into(),
            });
        }
        if let Some(code) = perceived["codebase_context"].as_str() {
            let _ = tx.send(CognitiveUpdate::EvidenceCode {
                title: "Codebase analysis".into(),
                content: code.into(),
                language: None,
                file_path: None,
            });
        }
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: 0, duration_ms: Some(perceive_ms) });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(1));
    }

    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));

    PerceiveResult {
        perceived,
        always_on_memory,
        beliefs_context,
        federation_context,
        perceive_ms,
    }
}
