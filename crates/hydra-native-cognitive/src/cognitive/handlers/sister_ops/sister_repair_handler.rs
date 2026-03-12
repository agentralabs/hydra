//! Sister repair handler — "fix broken sisters" / "fix contract sister".

use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use crate::sisters::connection::SisterConnection;
use hydra_native_state::utils::safe_truncate;

use super::super::super::loop_runner::CognitiveUpdate;
use super::super::super::intent_router::{IntentCategory, ClassifiedIntent};
use super::super::sisters::get_sister_bin_info;
use super::sister_repair_diagnosis::{diagnose_protocol_mismatch, emit_failure_report};

/// Handle sister repair — "fix broken sisters" / "fix contract sister".
pub(crate) async fn handle_sister_repair(
    text: &str,
    intent: &ClassifiedIntent,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.category != IntentCategory::SisterRepair {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Self-Repair".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    if let Some(ref sh) = sisters_handle {
        let target = intent.target.clone();
        let mut report = String::from("## Sister Repair Report\n\n");

        // Get offline sisters (or just the targeted one)
        let offline: Vec<(&str, &str, &[&str])> = get_sister_bin_info()
            .into_iter()
            .filter(|(name, _, _)| {
                let is_offline = sh.all_sisters().iter()
                    .any(|(n, opt)| n.to_lowercase() == name.to_lowercase() && opt.is_none());
                if let Some(ref t) = target {
                    is_offline && t.to_lowercase() == name.to_lowercase()
                } else {
                    is_offline
                }
            })
            .collect();

        if offline.is_empty() {
            if let Some(ref t) = target {
                report.push_str(&format!("**{}** sister is already online! No fix needed.\n", t));
            } else {
                report.push_str("All sisters are online! Nothing to fix.\n");
            }
        } else {
            report.push_str(&format!("Found **{}** offline sister(s). Repairing...\n\n", offline.len()));

            for (name, bin_name, args) in &offline {
                report.push_str(&format!("### {} Sister\n\n", name));

                let home = std::env::var("HOME").unwrap_or_default();
                let bin_path = format!("{}/.local/bin/{}", home, bin_name);
                let name_lower = name.to_lowercase();
                let workspace_root = format!("{}/Documents/agentralabs-tech", home);
                let sister_repo = format!("{}/agentic-{}", workspace_root, name_lower);
                let mcp_crate = format!("{}/crates/agentic-{}-mcp", sister_repo, name_lower);
                let has_repo = std::path::Path::new(&mcp_crate).exists();

                let mut attempts: Vec<(String, String)> = Vec::new();
                let mut fixed = false;

                // ── Attempt 1: Direct respawn ──
                if std::path::Path::new(&bin_path).exists() {
                    report.push_str("**Attempt 1:** Respawning process...\n");
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: report.clone(),
                        css_class: "message hydra diagnostics".into(),
                    });

                    match SisterConnection::spawn(name, &bin_path, args).await {
                        Ok(conn) => {
                            report.push_str(&format!("**{} is back online!** ({} tools)\n\n", name, conn.tools.len()));
                            fixed = true;
                        }
                        Err(e) => {
                            let err = e.to_string();
                            let short = safe_truncate(&err, 120).to_string();
                            report.push_str(&format!("Failed: {}\n", short));
                            attempts.push(("Respawn".into(), short));
                        }
                    }
                } else {
                    attempts.push(("Respawn".into(), "Binary not found".into()));
                }

                // ── Attempt 2: Fix corrupted data, then respawn ──
                if !fixed {
                    let db_candidates = vec![
                        format!("{}/.hydra/{}.db", home, name_lower),
                        format!("{}/.hydra/{}.sqlite", home, name_lower),
                    ];
                    let mut db_fixed = false;
                    for db_path in &db_candidates {
                        if std::path::Path::new(db_path).exists() {
                            report.push_str(&format!("**Attempt 2:** Moving aside DB `{}`...\n", db_path));
                            let backup = format!("{}.bak.{}", db_path, chrono::Utc::now().timestamp());
                            if std::fs::rename(db_path, &backup).is_ok() {
                                report.push_str("DB backed up. Respawning...\n");
                                db_fixed = true;
                                match SisterConnection::spawn(name, &bin_path, args).await {
                                    Ok(conn) => {
                                        report.push_str(&format!("**{} is back online!** ({} tools)\n\n", name, conn.tools.len()));
                                        fixed = true;
                                    }
                                    Err(e) => {
                                        let err = e.to_string();
                                        let short = safe_truncate(&err, 120).to_string();
                                        report.push_str(&format!("Still failed: {}\n", short));
                                        attempts.push(("DB repair + respawn".into(), short));
                                    }
                                }
                                break;
                            }
                        }
                    }
                    if !db_fixed && !fixed {
                        attempts.push(("DB repair".into(), "No DB file found to repair".into()));
                    }
                }

                // ── Attempt 3: Rebuild from source, then respawn ──
                if !fixed && has_repo {
                    report.push_str(&format!("**Attempt 3:** Rebuilding from source `{}`...\n", mcp_crate));
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: report.clone(),
                        css_class: "message hydra diagnostics".into(),
                    });

                    let build_result = tokio::process::Command::new("cargo")
                        .args(["install", "--path", &mcp_crate])
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .output()
                        .await;

                    match build_result {
                        Ok(output) if output.status.success() => {
                            report.push_str("Rebuild succeeded. Respawning...\n");
                            match SisterConnection::spawn(name, &bin_path, args).await {
                                Ok(conn) => {
                                    report.push_str(&format!("**{} is back online!** ({} tools)\n\n", name, conn.tools.len()));
                                    fixed = true;
                                }
                                Err(e) => {
                                    let err = e.to_string();
                                    let short = safe_truncate(&err, 120).to_string();
                                    report.push_str(&format!("Rebuild OK but respawn failed: {}\n", short));
                                    attempts.push(("Rebuild + respawn".into(), short));
                                }
                            }
                        }
                        Ok(output) => {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let err_tail = if stderr.len() > 300 { &stderr[stderr.len() - 300..] } else { &stderr };
                            report.push_str(&format!("Rebuild failed: {}\n", err_tail.trim()));
                            attempts.push(("Rebuild".into(), err_tail.trim().to_string()));
                        }
                        Err(e) => {
                            report.push_str(&format!("Could not run cargo: {}\n", e));
                            attempts.push(("Rebuild".into(), e.to_string()));
                        }
                    }
                } else if !fixed && !has_repo {
                    attempts.push(("Rebuild".into(), format!("Repo not found at {}", mcp_crate)));
                }

                // ── Attempt 4: Try alternative args (--stdio, serve, no args) ──
                if !fixed && std::path::Path::new(&bin_path).exists() {
                    let alt_args_list: Vec<&[&str]> = vec![
                        &["--stdio"],
                        &["serve", "--stdio"],
                        &["serve"],
                        &[],
                    ];
                    for alt in &alt_args_list {
                        if *alt as &[&str] == *args { continue; }
                        report.push_str(&format!("**Attempt 4:** Trying args: `{}`...\n", alt.join(" ")));
                        match SisterConnection::spawn(name, &bin_path, alt).await {
                            Ok(conn) => {
                                report.push_str(&format!("**{} is back online!** ({} tools) — with args: `{}`\n\n",
                                    name, conn.tools.len(), alt.join(" ")));
                                fixed = true;
                                break;
                            }
                            Err(e) => {
                                let err = e.to_string();
                                let short = safe_truncate(&err, 80).to_string();
                                report.push_str(&format!("Failed: {}\n", short));
                                attempts.push((format!("Alt args `{}`", alt.join(" ")), short));
                            }
                        }
                    }
                }

                // ── Attempt 5: Clean rebuild (cargo clean first) ──
                if !fixed && has_repo {
                    report.push_str("**Attempt 5:** Clean rebuild (cargo clean + install)...\n");
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: report.clone(),
                        css_class: "message hydra diagnostics".into(),
                    });

                    let _ = tokio::process::Command::new("cargo")
                        .args(["clean"])
                        .current_dir(&sister_repo)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .output()
                        .await;

                    let build_result = tokio::process::Command::new("cargo")
                        .args(["install", "--path", &mcp_crate, "--force"])
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .output()
                        .await;

                    match build_result {
                        Ok(output) if output.status.success() => {
                            report.push_str("Clean rebuild succeeded. Respawning...\n");
                            match SisterConnection::spawn(name, &bin_path, args).await {
                                Ok(conn) => {
                                    report.push_str(&format!("**{} is back online!** ({} tools)\n\n", name, conn.tools.len()));
                                    fixed = true;
                                }
                                Err(e) => {
                                    let err = e.to_string();
                                    let short = safe_truncate(&err, 120).to_string();
                                    report.push_str(&format!("Clean rebuild OK but respawn still failed: {}\n", short));
                                    attempts.push(("Clean rebuild + respawn".into(), short));
                                }
                            }
                        }
                        Ok(output) => {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let err_tail = if stderr.len() > 300 { &stderr[stderr.len() - 300..] } else { &stderr };
                            attempts.push(("Clean rebuild".into(), err_tail.trim().to_string()));
                        }
                        Err(e) => {
                            attempts.push(("Clean rebuild".into(), e.to_string()));
                        }
                    }
                }

                // ── Attempt 6: Source code diagnosis (protocol mismatch) ──
                if !fixed && has_repo {
                    diagnose_protocol_mismatch(
                        name, bin_name, &name_lower, &workspace_root, &mcp_crate,
                        &attempts, &mut report, tx,
                    ).await;
                }

                // ── Final report if not fixed ──
                if !fixed {
                    emit_failure_report(name, &name_lower, bin_name, &attempts, &mut report);
                }
            }
        }

        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: report,
            css_class: "message hydra diagnostics".into(),
        });
    } else {
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: "No sisters available — running in offline mode.".into(),
            css_class: "message hydra error".into(),
        });
    }

    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

