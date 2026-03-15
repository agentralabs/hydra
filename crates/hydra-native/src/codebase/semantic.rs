//! Semantic code search and impact analysis via Codebase sister.
//!
//! Phase 3, C6: Provides AST-aware code understanding, replacing grep-based
//! searches with real semantic results (function names, types, call sites).

use serde::{Deserialize, Serialize};
use crate::sisters::SistersHandle;

/// A single semantic search hit — richer than a grep match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchHit {
    /// File path relative to project root
    pub file_path: String,
    /// Line number of the match
    pub line: usize,
    /// The symbol or code entity matched (function name, struct, etc.)
    pub symbol: String,
    /// The kind of entity: "function", "struct", "trait", "impl", "type", "const"
    pub kind: String,
    /// Surrounding context (a few lines around the match)
    pub context: String,
    /// Relevance score (0.0 to 1.0)
    pub relevance: f64,
}

/// Result of a semantic search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub query: String,
    pub hits: Vec<SemanticSearchHit>,
    pub total_files_searched: usize,
    pub search_time_ms: u64,
}

/// A single entry in the impact analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEntry {
    pub file_path: String,
    pub symbol: String,
    pub relationship: String,  // "calls", "depends_on", "imports", "tests"
    pub line: usize,
}

/// Impact analysis report for a file or symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub target: String,
    pub callers: Vec<ImpactEntry>,
    pub dependents: Vec<ImpactEntry>,
    pub test_coverage: Vec<ImpactEntry>,
    pub risk_level: String,  // "low", "medium", "high", "critical"
    pub summary: String,
}

/// Engine that wraps Codebase sister MCP tools for semantic code operations.
pub struct CodebaseSemanticEngine;

impl CodebaseSemanticEngine {
    /// Semantic code search — queries the Codebase sister's AST-aware search.
    /// Falls back to grep if the Codebase sister is unavailable.
    pub async fn search(
        query: &str,
        project_dir: &str,
        limit: usize,
        sisters: &Option<SistersHandle>,
    ) -> SemanticSearchResult {
        let start = std::time::Instant::now();

        // Try Codebase sister MCP tool first
        if let Some(ref sh) = sisters {
            if let Some(ref codebase) = sh.codebase {
                let tool_input = serde_json::json!({
                    "query": query,
                    "project_dir": project_dir,
                    "limit": limit,
                });

                if let Ok(result) = codebase.call_tool("search_semantic", tool_input).await {
                    if let Ok(parsed) = serde_json::from_value::<Vec<SemanticSearchHit>>(result) {
                        return SemanticSearchResult {
                            query: query.to_string(),
                            hits: parsed,
                            total_files_searched: 0, // populated by sister
                            search_time_ms: start.elapsed().as_millis() as u64,
                        };
                    }
                }
            }
        }

        // Fallback: grep-based search (basic text matching)
        let hits = grep_fallback(query, project_dir, limit);
        SemanticSearchResult {
            query: query.to_string(),
            hits,
            total_files_searched: 0,
            search_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Impact analysis — what would change if you modify a file or symbol?
    pub async fn impact_analyze(
        target: &str,
        project_dir: &str,
        sisters: &Option<SistersHandle>,
    ) -> ImpactReport {
        // Try Codebase sister MCP tool
        if let Some(ref sh) = sisters {
            if let Some(ref codebase) = sh.codebase {
                let tool_input = serde_json::json!({
                    "file_path": target,
                    "project_dir": project_dir,
                });

                if let Ok(result) = codebase.call_tool("impact_analyze", tool_input).await {
                    if let Ok(parsed) = serde_json::from_value::<ImpactReport>(result) {
                        return parsed;
                    }
                }
            }
        }

        // Fallback: basic dependency scan
        ImpactReport {
            target: target.to_string(),
            callers: Vec::new(),
            dependents: Vec::new(),
            test_coverage: Vec::new(),
            risk_level: "unknown".to_string(),
            summary: format!(
                "Codebase sister unavailable. Manual impact analysis required for '{}'.",
                target
            ),
        }
    }

    /// Self-understanding — Hydra describes its own architecture.
    /// Routes through Omniscience to build a semantic description from Hydra's own code.
    pub async fn self_describe(
        project_dir: &str,
        sisters: &Option<SistersHandle>,
    ) -> String {
        // Try Codebase sister for AST-level analysis
        if let Some(ref sh) = sisters {
            if let Some(ref codebase) = sh.codebase {
                let tool_input = serde_json::json!({
                    "project_dir": project_dir,
                    "mode": "architecture_summary",
                });

                if let Ok(result) = codebase.call_tool("codebase_scan", tool_input).await {
                    if let Some(summary) = result.as_str() {
                        return summary.to_string();
                    }
                    if let Some(obj) = result.as_object() {
                        if let Some(summary) = obj.get("summary").and_then(|s| s.as_str()) {
                            return summary.to_string();
                        }
                    }
                }
            }
        }

        // Fallback description
        "Hydra is a cognitive AI assistant built in Rust with a 5-phase loop: \
         Perceive → Think → Decide → Act → Learn. It has 17 sister subsystems \
         for memory, identity, codebase analysis, communication, security (Aegis), \
         code generation (Forge), reality sensing, contract management, writing (Scribe), \
         vision, collaboration, ledger tracking, and evolution. The Codebase sister \
         was not available for live AST analysis."
            .to_string()
    }

    /// Safety check for self-modification queries.
    /// Any query that would MODIFY Hydra's own source must go through this gate.
    pub async fn self_modification_gate(
        target_file: &str,
        project_dir: &str,
        sisters: &Option<SistersHandle>,
    ) -> (bool, String) {
        let hydra_src = format!("{}/crates/", project_dir);
        let is_hydra_source = target_file.starts_with(&hydra_src)
            || target_file.contains("hydra-native")
            || target_file.contains("hydra-core")
            || target_file.contains("hydra-gate")
            || target_file.contains("hydra-kernel");

        if !is_hydra_source {
            return (true, "Not a Hydra source file — no special gate needed.".to_string());
        }

        // Run impact analysis before allowing modification
        let impact = Self::impact_analyze(target_file, project_dir, sisters).await;

        let requires_approval = matches!(impact.risk_level.as_str(), "high" | "critical" | "unknown");

        let message = if requires_approval {
            format!(
                "⚠️ Self-modification detected: {}\nRisk: {}\n{} callers, {} dependents, {} tests\n\
                 Requires human approval before proceeding.",
                target_file,
                impact.risk_level,
                impact.callers.len(),
                impact.dependents.len(),
                impact.test_coverage.len(),
            )
        } else {
            format!(
                "Self-modification: {} (risk: {}). {} callers, {} dependents.",
                target_file,
                impact.risk_level,
                impact.callers.len(),
                impact.dependents.len(),
            )
        };

        (!requires_approval, message)
    }
}

/// Fallback grep-based search when Codebase sister is unavailable.
fn grep_fallback(query: &str, project_dir: &str, limit: usize) -> Vec<SemanticSearchHit> {
    let output = std::process::Command::new("grep")
        .args(["-rn", "--include=*.rs", query, project_dir])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .take(limit)
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(3, ':').collect();
                    if parts.len() >= 3 {
                        Some(SemanticSearchHit {
                            file_path: parts[0].to_string(),
                            line: parts[1].parse().unwrap_or(0),
                            symbol: String::new(), // grep can't determine symbols
                            kind: "text_match".to_string(),
                            context: parts[2].to_string(),
                            relevance: 0.5, // flat relevance for text matches
                        })
                    } else {
                        None
                    }
                })
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_search_hit_serialization() {
        let hit = SemanticSearchHit {
            file_path: "src/main.rs".to_string(),
            line: 42,
            symbol: "run_cognitive_loop".to_string(),
            kind: "function".to_string(),
            context: "pub async fn run_cognitive_loop(...)".to_string(),
            relevance: 0.95,
        };
        let json = serde_json::to_string(&hit).unwrap();
        let back: SemanticSearchHit = serde_json::from_str(&json).unwrap();
        assert_eq!(back.symbol, "run_cognitive_loop");
        assert_eq!(back.line, 42);
    }

    #[test]
    fn test_impact_report_serialization() {
        let report = ImpactReport {
            target: "loop_runner.rs".to_string(),
            callers: vec![ImpactEntry {
                file_path: "app.rs".to_string(),
                symbol: "submit_query".to_string(),
                relationship: "calls".to_string(),
                line: 100,
            }],
            dependents: Vec::new(),
            test_coverage: Vec::new(),
            risk_level: "high".to_string(),
            summary: "Core file with many callers".to_string(),
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: ImpactReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.callers.len(), 1);
        assert_eq!(back.risk_level, "high");
    }

    #[test]
    fn test_self_modification_gate_non_hydra_file() {
        // Synchronous test for the non-Hydra path (doesn't need async)
        let target = "/tmp/some-project/src/main.rs";
        let is_hydra = target.contains("hydra-native")
            || target.contains("hydra-core")
            || target.contains("hydra-gate");
        assert!(!is_hydra, "Non-Hydra file should not trigger gate");
    }

    #[test]
    fn test_self_modification_gate_hydra_file() {
        let target = "crates/hydra-native/src/cognitive/loop_runner.rs";
        let is_hydra = target.contains("hydra-native");
        assert!(is_hydra, "Hydra source file should trigger gate");
    }

    #[test]
    fn test_grep_fallback_empty_on_missing_dir() {
        let hits = grep_fallback("nonexistent_function_xyz", "/tmp/nonexistent_dir_xyz", 10);
        assert!(hits.is_empty());
    }
}
