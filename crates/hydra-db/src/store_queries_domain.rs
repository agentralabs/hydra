use rusqlite::params;

use crate::store_types::*;

/// Query/read methods for HydraDb — domain entities (anomalies, trust, cursor, beliefs, MCP, federation, repair)
impl crate::store::HydraDb {
    // ═══════════════════════════════════════════════════════
    // ANOMALY EVENTS
    // ═══════════════════════════════════════════════════════

    pub fn list_anomaly_events(&self, limit: usize) -> Result<Vec<AnomalyEventRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT event_type, command, detail, severity, kill_switch_engaged FROM anomaly_events ORDER BY created_at DESC LIMIT ?1"
        )?;
        let iter = stmt.query_map(params![limit as i64], |row| {
            let ks: i32 = row.get(4)?;
            Ok(AnomalyEventRow {
                event_type: row.get(0)?,
                command: row.get(1)?,
                detail: row.get(2)?,
                severity: row.get(3)?,
                kill_switch_engaged: ks != 0,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter { rows.push(r?); }
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════
    // TRUST SCORES
    // ═══════════════════════════════════════════════════════

    pub fn get_trust_score(&self, domain: &str) -> Result<Option<TrustScoreRow>, DbError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT domain, score, total_actions, successful_actions, failed_actions, autonomy_level FROM trust_scores WHERE domain = ?1",
            params![domain],
            |row| Ok(TrustScoreRow {
                domain: row.get(0)?,
                score: row.get(1)?,
                total_actions: row.get(2)?,
                successful_actions: row.get(3)?,
                failed_actions: row.get(4)?,
                autonomy_level: row.get(5)?,
            }),
        );
        match result {
            Ok(ts) => Ok(Some(ts)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }

    // ═══════════════════════════════════════════════════════
    // CURSOR SESSIONS & EVENTS
    // ═══════════════════════════════════════════════════════

    pub fn list_cursor_sessions(&self, task_id: Option<&str>, limit: usize) -> Result<Vec<CursorSessionRow>, DbError> {
        let conn = self.conn.lock();
        let mut rows = Vec::new();
        if let Some(tid) = task_id {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, mode, started_at, ended_at, event_count, total_duration_ms FROM cursor_sessions WHERE task_id = ?1 ORDER BY started_at DESC LIMIT ?2"
            )?;
            let iter = stmt.query_map(params![tid, limit as i64], |row| {
                Ok(CursorSessionRow {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    mode: row.get(2)?,
                    started_at: row.get(3)?,
                    ended_at: row.get(4)?,
                    event_count: row.get(5)?,
                    total_duration_ms: row.get(6)?,
                })
            })?;
            for r in iter { rows.push(r?); }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, mode, started_at, ended_at, event_count, total_duration_ms FROM cursor_sessions ORDER BY started_at DESC LIMIT ?1"
            )?;
            let iter = stmt.query_map(params![limit as i64], |row| {
                Ok(CursorSessionRow {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    mode: row.get(2)?,
                    started_at: row.get(3)?,
                    ended_at: row.get(4)?,
                    event_count: row.get(5)?,
                    total_duration_ms: row.get(6)?,
                })
            })?;
            for r in iter { rows.push(r?); }
        }
        Ok(rows)
    }

    pub fn get_cursor_events(&self, session_id: &str) -> Result<Vec<CursorEventRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT timestamp_ms, event_type, x, y, payload FROM cursor_events WHERE session_id = ?1 ORDER BY timestamp_ms"
        )?;
        let iter = stmt.query_map(params![session_id], |row| {
            Ok(CursorEventRow {
                timestamp_ms: row.get(0)?,
                event_type: row.get(1)?,
                x: row.get(2)?,
                y: row.get(3)?,
                payload: row.get(4)?,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter { rows.push(r?); }
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════
    // BELIEFS
    // ═══════════════════════════════════════════════════════

    pub fn get_active_beliefs(&self, limit: usize) -> Result<Vec<BeliefRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, category, subject, content, confidence, source, confirmations, contradictions, active, supersedes, superseded_by, created_at, updated_at FROM beliefs WHERE active = 1 ORDER BY confidence DESC, updated_at DESC LIMIT ?1"
        )?;
        let iter = stmt.query_map(params![limit as i64], |row| {
            let active_i: i32 = row.get(8)?;
            Ok(BeliefRow {
                id: row.get(0)?, category: row.get(1)?, subject: row.get(2)?,
                content: row.get(3)?, confidence: row.get(4)?, source: row.get(5)?,
                confirmations: row.get(6)?, contradictions: row.get(7)?,
                active: active_i != 0, supersedes: row.get(9)?,
                superseded_by: row.get(10)?, created_at: row.get(11)?, updated_at: row.get(12)?,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter { rows.push(r?); }
        Ok(rows)
    }

    pub fn get_beliefs_by_subject(&self, subject: &str) -> Result<Vec<BeliefRow>, DbError> {
        let conn = self.conn.lock();
        let pattern = format!("%{}%", subject);
        let mut stmt = conn.prepare(
            "SELECT id, category, subject, content, confidence, source, confirmations, contradictions, active, supersedes, superseded_by, created_at, updated_at FROM beliefs WHERE active = 1 AND subject LIKE ?1 ORDER BY confidence DESC"
        )?;
        let iter = stmt.query_map(params![pattern], |row| {
            let active_i: i32 = row.get(8)?;
            Ok(BeliefRow {
                id: row.get(0)?, category: row.get(1)?, subject: row.get(2)?,
                content: row.get(3)?, confidence: row.get(4)?, source: row.get(5)?,
                confirmations: row.get(6)?, contradictions: row.get(7)?,
                active: active_i != 0, supersedes: row.get(9)?,
                superseded_by: row.get(10)?, created_at: row.get(11)?, updated_at: row.get(12)?,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter { rows.push(r?); }
        Ok(rows)
    }

    pub fn belief_count(&self) -> Result<i64, DbError> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM beliefs WHERE active = 1", [], |row| row.get(0),
        )?;
        Ok(count)
    }

    // ═══════════════════════════════════════════════════════
    // MCP DISCOVERED SKILLS
    // ═══════════════════════════════════════════════════════

    pub fn list_mcp_skills(&self, server: Option<&str>) -> Result<Vec<McpDiscoveredSkillRow>, DbError> {
        let conn = self.conn.lock();
        let mut rows = Vec::new();
        if let Some(srv) = server {
            let mut stmt = conn.prepare(
                "SELECT id, server_name, tool_name, description, input_schema, discovered_at, last_used_at, use_count, active FROM mcp_discovered_skills WHERE active = 1 AND server_name = ?1 ORDER BY use_count DESC"
            )?;
            let iter = stmt.query_map(params![srv], |row| {
                let active_i: i32 = row.get(8)?;
                Ok(McpDiscoveredSkillRow {
                    id: row.get(0)?, server_name: row.get(1)?, tool_name: row.get(2)?,
                    description: row.get(3)?, input_schema: row.get(4)?,
                    discovered_at: row.get(5)?, last_used_at: row.get(6)?,
                    use_count: row.get(7)?, active: active_i != 0,
                })
            })?;
            for r in iter { rows.push(r?); }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, server_name, tool_name, description, input_schema, discovered_at, last_used_at, use_count, active FROM mcp_discovered_skills WHERE active = 1 ORDER BY use_count DESC"
            )?;
            let iter = stmt.query_map([], |row| {
                let active_i: i32 = row.get(8)?;
                Ok(McpDiscoveredSkillRow {
                    id: row.get(0)?, server_name: row.get(1)?, tool_name: row.get(2)?,
                    description: row.get(3)?, input_schema: row.get(4)?,
                    discovered_at: row.get(5)?, last_used_at: row.get(6)?,
                    use_count: row.get(7)?, active: active_i != 0,
                })
            })?;
            for r in iter { rows.push(r?); }
        }
        Ok(rows)
    }

    pub fn mcp_skill_count(&self) -> Result<i64, DbError> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM mcp_discovered_skills WHERE active = 1", [], |row| row.get(0),
        )?;
        Ok(count)
    }

    // ═══════════════════════════════════════════════════════
    // FEDERATION STATE
    // ═══════════════════════════════════════════════════════

    pub fn list_federation_peers(&self) -> Result<Vec<FederationStateRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT peer_id, peer_name, endpoint, trust_level, capabilities, federation_type, last_sync_version, last_seen, active_tasks, active FROM federation_state WHERE active = 1 ORDER BY last_seen DESC"
        )?;
        let iter = stmt.query_map([], |row| {
            let active_i: i32 = row.get(9)?;
            Ok(FederationStateRow {
                peer_id: row.get(0)?, peer_name: row.get(1)?, endpoint: row.get(2)?,
                trust_level: row.get(3)?, capabilities: row.get(4)?,
                federation_type: row.get(5)?, last_sync_version: row.get(6)?,
                last_seen: row.get(7)?, active_tasks: row.get(8)?, active: active_i != 0,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter { rows.push(r?); }
        Ok(rows)
    }

    pub fn federation_peer_count(&self) -> Result<i64, DbError> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM federation_state WHERE active = 1", [], |row| row.get(0),
        )?;
        Ok(count)
    }

    // ═══════════════════════════════════════════════════════
    // REPAIR RUNS
    // ═══════════════════════════════════════════════════════

    pub fn list_repair_runs(&self, limit: usize) -> Result<Vec<RepairRunRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, spec_file, task, status, iteration, max_iterations, checks_total, checks_passed, failure_log, started_at, completed_at, duration_ms FROM repair_runs ORDER BY started_at DESC LIMIT ?1"
        )?;
        let iter = stmt.query_map(params![limit as i64], |row| {
            Ok(RepairRunRow {
                id: row.get(0)?, spec_file: row.get(1)?, task: row.get(2)?,
                status: row.get(3)?, iteration: row.get(4)?, max_iterations: row.get(5)?,
                checks_total: row.get(6)?, checks_passed: row.get(7)?,
                failure_log: row.get(8)?, started_at: row.get(9)?,
                completed_at: row.get(10)?, duration_ms: row.get(11)?,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter { rows.push(r?); }
        Ok(rows)
    }
}
