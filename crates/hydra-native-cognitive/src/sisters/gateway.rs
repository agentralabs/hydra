//! SisterGateway — enforces sister-first pattern for every operation.
//!
//! Every capability calls gateway methods, NEVER raw local logic directly.
//! The gateway tries the sister first and falls back to local automatically.
//! The caller NEVER knows which path executed — they just get the result.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use super::cognitive::Sisters;
use super::connection::{extract_text, SisterConnection};
use super::gateway_helpers::{
    extract_file_path_from_result, local_find_file, local_grep, local_risk_assessment,
    local_safety_check, local_time_context, parse_memory_results, parse_risk_level,
    parse_safety_result, parse_time_context, EnvironmentInfo, RiskLevel, SafetyResult,
    TimeContext,
};

/// SisterGateway — enforces sister-first, local-fallback for all operations.
pub struct SisterGateway {
    sisters: Arc<Sisters>,
    sister_count: AtomicU32,
    fallback_count: AtomicU32,
    per_sister: [AtomicU32; 17],
    per_fallback: [AtomicU32; 17],
}

/// Sister index constants for per-sister stats.
const S_MEMORY: usize = 0;
const S_IDENTITY: usize = 1;
const S_CODEBASE: usize = 2;
const S_VISION: usize = 3;
const S_COMM: usize = 4;
const S_CONTRACT: usize = 5;
const S_TIME: usize = 6;
const S_PLANNING: usize = 7;
const S_COGNITION: usize = 8;
const S_REALITY: usize = 9;
const S_FORGE: usize = 10;
const S_AEGIS: usize = 11;
const S_VERITAS: usize = 12;
const S_EVOLVE: usize = 13;
const S_DATA: usize = 14;
const S_CONNECT: usize = 15;
const S_WORKFLOW: usize = 16;

const SISTER_NAMES: [&str; 17] = [
    "Memory", "Identity", "Codebase", "Vision", "Comm", "Contract", "Time",
    "Planning", "Cognition", "Reality", "Forge", "Aegis", "Veritas", "Evolve",
    "Data", "Connect", "Workflow",
];

fn sister_index(name: &str) -> usize {
    match name.to_lowercase().as_str() {
        "memory" => S_MEMORY,
        "identity" => S_IDENTITY,
        "codebase" => S_CODEBASE,
        "vision" => S_VISION,
        "comm" => S_COMM,
        "contract" => S_CONTRACT,
        "time" => S_TIME,
        "planning" => S_PLANNING,
        "cognition" => S_COGNITION,
        "reality" => S_REALITY,
        "forge" => S_FORGE,
        "aegis" => S_AEGIS,
        "veritas" => S_VERITAS,
        "evolve" => S_EVOLVE,
        "data" => S_DATA,
        "connect" => S_CONNECT,
        "workflow" => S_WORKFLOW,
        _ => 0,
    }
}

impl SisterGateway {
    /// Create a new gateway wrapping the given sisters handle.
    pub fn new(sisters: Arc<Sisters>) -> Self {
        Self {
            sisters,
            sister_count: AtomicU32::new(0),
            fallback_count: AtomicU32::new(0),
            per_sister: Default::default(),
            per_fallback: Default::default(),
        }
    }

    /// Get the underlying sisters handle (for operations not yet in gateway).
    pub fn sisters(&self) -> &Arc<Sisters> {
        &self.sisters
    }

    // ── FILE OPERATIONS ──

    /// Find a file by name. Codebase sister first, local find fallback.
    pub async fn find_file(&self, name: &str, project_root: &Path) -> Option<PathBuf> {
        if let Some(result) = self.try_sister("Codebase", "file_search", serde_json::json!({
            "query": name, "max_results": 5
        })).await {
            if let Some(path) = extract_file_path_from_result(&result) {
                self.record_sister("Codebase");
                return Some(path);
            }
        }
        // Also try semantic search
        if let Some(result) = self.try_sister("Codebase", "search_semantic", serde_json::json!({
            "query": name, "max_results": 5
        })).await {
            if let Some(path) = extract_file_path_from_result(&result) {
                self.record_sister("Codebase");
                return Some(path);
            }
        }
        self.record_fallback("Codebase");
        eprintln!("[hydra:gateway] find_file fallback to local for: {}", name);
        local_find_file(name, project_root)
    }

    /// Read a file's contents. Local fs with codebase sister as context enrichment.
    pub async fn read_file(&self, path: &Path) -> Option<String> {
        // Files on disk are best read locally; use sister for enrichment if needed
        if let Ok(content) = std::fs::read_to_string(path) {
            return Some(content);
        }
        // If path doesn't exist, try finding it via sister
        let name = path.file_name()?.to_string_lossy().to_string();
        if let Some(found) = self.find_file(&name, Path::new(".")).await {
            return std::fs::read_to_string(found).ok();
        }
        None
    }

    // ── RISK ASSESSMENT ──

    /// Assess risk of a command. Contract + Aegis sisters first, local regex fallback.
    pub async fn assess_risk(&self, command: &str) -> RiskLevel {
        if let Some(result) = self.try_sister("Contract", "contract_precognition", serde_json::json!({
            "planned_action": command
        })).await {
            self.record_sister("Contract");
            return parse_risk_level(&result);
        }
        if let Some(result) = self.try_sister("Aegis", "aegis_check_input", serde_json::json!({
            "input": command
        })).await {
            self.record_sister("Aegis");
            return parse_risk_level(&result);
        }
        self.record_fallback("Contract");
        local_risk_assessment(command)
    }

    // ── MEMORY OPERATIONS ──

    /// Store something in memory. Memory sister first, local log fallback.
    pub async fn store(&self, content: &str, event_type: &str) -> bool {
        if let Some(_) = self.try_sister("Memory", "memory_add", serde_json::json!({
            "content": content, "event_type": event_type, "confidence": 0.8
        })).await {
            self.record_sister("Memory");
            return true;
        }
        self.record_fallback("Memory");
        eprintln!("[hydra:gateway] FALLBACK: memory store failed, logging locally");
        eprintln!("[hydra:gateway:local-store] {}: {}", event_type, &content[..content.len().min(200)]);
        false
    }

    /// Query memory. Memory sister first, empty vec fallback.
    pub async fn recall(&self, query: &str, max: usize) -> Vec<String> {
        let mut results = Vec::new();
        if let Some(r) = self.try_sister("Memory", "memory_query", serde_json::json!({
            "query": query, "max_results": max
        })).await {
            results.extend(parse_memory_results(&r));
            self.record_sister("Memory");
        }
        if let Some(r) = self.try_sister("Memory", "memory_query", serde_json::json!({
            "query": query, "max_results": 10, "sort_by": "most_recent"
        })).await {
            for item in parse_memory_results(&r) {
                if !results.contains(&item) { results.push(item); }
            }
        }
        if results.is_empty() { self.record_fallback("Memory"); }
        results
    }

    // ── TIME AWARENESS ──

    /// Get time context. Time sister first, local chrono fallback.
    pub async fn time_context(&self) -> TimeContext {
        if let Some(result) = self.try_sister("Time", "time_stats", serde_json::json!({})).await {
            self.record_sister("Time");
            return parse_time_context(&result);
        }
        self.record_fallback("Time");
        local_time_context()
    }

    // ── ENVIRONMENT ──

    /// Get environment info. Reality sister first, local probe fallback.
    pub async fn environment(&self) -> EnvironmentInfo {
        if let Some(result) = self.try_sister("Reality", "reality_context", serde_json::json!({
            "input": "environment"
        })).await {
            self.record_sister("Reality");
            return EnvironmentInfo { raw: result };
        }
        self.record_fallback("Reality");
        EnvironmentInfo { raw: "local fallback — no Reality sister".into() }
    }

    // ── ERROR LEARNING ──

    /// Learn from an error. Cognition + Memory sisters.
    pub async fn learn_from_error(&self, error: &str, resolution: &str) {
        let belief = format!("Error '{}' resolved by: {}", &error[..error.len().min(100)], resolution);
        let _ = self.try_sister("Cognition", "cognition_belief_revise", serde_json::json!({
            "belief": belief, "confidence": 0.9
        })).await;
        let content = format!("Learned: {} -> fix: {}", &error[..error.len().min(100)], resolution);
        let _ = self.try_sister("Memory", "memory_add", serde_json::json!({
            "content": content, "event_type": "episode", "confidence": 0.9
        })).await;
    }

    /// Check if we've seen this error before. Cognition first, Memory fallback.
    pub async fn known_resolution(&self, error: &str) -> Option<String> {
        if let Some(result) = self.try_sister("Cognition", "cognition_belief_query", serde_json::json!({
            "query": error
        })).await {
            if !result.is_empty() && !result.contains("No beliefs") {
                self.record_sister("Cognition");
                return Some(result);
            }
        }
        if let Some(result) = self.try_sister("Memory", "memory_query", serde_json::json!({
            "query": format!("error resolution {}", &error[..error.len().min(80)]),
            "max_results": 3
        })).await {
            if !result.is_empty() && !result.contains("No memories") {
                self.record_sister("Memory");
                return Some(result);
            }
        }
        None
    }

    // ── CODE UNDERSTANDING ──

    /// Search code semantically. Codebase sister first, local grep fallback.
    pub async fn code_search(&self, query: &str, project_root: &Path) -> Vec<String> {
        if let Some(result) = self.try_sister("Codebase", "search_semantic", serde_json::json!({
            "query": query, "max_results": 10
        })).await {
            self.record_sister("Codebase");
            return result.lines().map(String::from).collect();
        }
        self.record_fallback("Codebase");
        local_grep(query, project_root)
    }

    /// Analyze code impact. Codebase sister only (too complex for local).
    pub async fn code_impact(&self, change: &str) -> Option<String> {
        self.try_sister("Codebase", "impact_analyze", serde_json::json!({
            "query": change
        })).await
    }

    // ── VERIFICATION ──

    /// Verify a claim. Veritas sister only.
    pub async fn verify_claim(&self, claim: &str) -> Option<String> {
        self.try_sister("Veritas", "veritas_claim_check", serde_json::json!({
            "claim": claim
        })).await
    }

    /// Validate input safety. Aegis sister first, local blocklist fallback.
    pub async fn validate_input(&self, input: &str) -> SafetyResult {
        if let Some(result) = self.try_sister("Aegis", "aegis_check_input", serde_json::json!({
            "input": input
        })).await {
            self.record_sister("Aegis");
            return parse_safety_result(&result);
        }
        self.record_fallback("Aegis");
        local_safety_check(input)
    }

    /// Validate output safety. Aegis sister only.
    pub async fn validate_output(&self, output: &str) -> SafetyResult {
        if let Some(result) = self.try_sister("Aegis", "aegis_check_output", serde_json::json!({
            "output": output
        })).await {
            return parse_safety_result(&result);
        }
        SafetyResult::Unknown
    }

    // ── CORE HELPER ──

    /// Try calling a sister tool. Returns extracted text on success, None on failure.
    async fn try_sister(&self, sister: &str, tool: &str, params: serde_json::Value) -> Option<String> {
        let conn = self.get_sister(sister)?;
        match conn.call_tool(tool, params).await {
            Ok(result) => {
                let text = extract_text(&result);
                if text.is_empty() || text == "null" { None } else { Some(text) }
            }
            Err(e) => {
                eprintln!("[hydra:gateway] {}::{} failed: {}", sister, tool, e);
                None
            }
        }
    }

    /// Route sister name to the corresponding connection field.
    fn get_sister(&self, name: &str) -> Option<&SisterConnection> {
        match name.to_lowercase().as_str() {
            "memory" => self.sisters.memory.as_ref(),
            "identity" => self.sisters.identity.as_ref(),
            "codebase" => self.sisters.codebase.as_ref(),
            "vision" => self.sisters.vision.as_ref(),
            "comm" => self.sisters.comm.as_ref(),
            "contract" => self.sisters.contract.as_ref(),
            "time" => self.sisters.time.as_ref(),
            "planning" => self.sisters.planning.as_ref(),
            "cognition" => self.sisters.cognition.as_ref(),
            "reality" => self.sisters.reality.as_ref(),
            "forge" => self.sisters.forge.as_ref(),
            "aegis" => self.sisters.aegis.as_ref(),
            "veritas" => self.sisters.veritas.as_ref(),
            "evolve" => self.sisters.evolve.as_ref(),
            "data" => self.sisters.data.as_ref(),
            "connect" => self.sisters.connect.as_ref(),
            "workflow" => self.sisters.workflow.as_ref(),
            _ => None,
        }
    }

    fn record_sister(&self, name: &str) {
        self.sister_count.fetch_add(1, Ordering::Relaxed);
        self.per_sister[sister_index(name)].fetch_add(1, Ordering::Relaxed);
    }

    fn record_fallback(&self, name: &str) {
        self.fallback_count.fetch_add(1, Ordering::Relaxed);
        self.per_fallback[sister_index(name)].fetch_add(1, Ordering::Relaxed);
    }

    // ── STATS ──

    /// Total (sister_calls, fallback_calls).
    pub fn stats(&self) -> (u32, u32) {
        (self.sister_count.load(Ordering::Relaxed), self.fallback_count.load(Ordering::Relaxed))
    }

    /// Per-sister stats: Vec of (name, sister_calls, fallback_calls).
    pub fn stats_per_sister(&self) -> Vec<(&'static str, u32, u32)> {
        (0..17).map(|i| {
            (SISTER_NAMES[i], self.per_sister[i].load(Ordering::Relaxed), self.per_fallback[i].load(Ordering::Relaxed))
        }).filter(|(_, s, f)| *s > 0 || *f > 0).collect()
    }

    /// Format stats for /stats command display.
    pub fn stats_display(&self) -> String {
        let (s, f) = self.stats();
        let total = s + f;
        let pct = if total > 0 { (s as f64 / total as f64) * 100.0 } else { 0.0 };
        let mut out = format!("Sister Intelligence:\n  Sister calls:   {} ({:.0}%)\n  Local fallbacks: {} ({:.0}%)\n",
            s, pct, f, 100.0 - pct);
        for (name, sc, fc) in self.stats_per_sister() {
            out.push_str(&format!("  {:12} {} calls ({} fallbacks)\n", name, sc, fc));
        }
        out
    }
}
