//! Code templates for new sister projects — core library and MCP registry.
//!
//! Split from project_creation_templates.rs to stay under 400 lines.

use super::project_creation::ProjectConfig;

/// Render the core library source (lib.rs) with Store, send/query/history/stats/clear.
pub(super) fn render_core_lib(c: &ProjectConfig) -> String {
    let ext = &c.file_ext;
    format!(
        r#"//! {name} — core library.
//!
//! Provides the storage backend for {ext} files (SQLite-backed).

use std::path::Path;
use rusqlite::{{Connection, params}};
use chrono::{{DateTime, Utc}};
use serde::{{Deserialize, Serialize}};

/// A stored message with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {{
    pub id: i64,
    pub content: String,
    pub word_count: usize,
    pub timestamp: DateTime<Utc>,
}}

/// Stats about stored messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {{
    pub message_count: usize,
    pub total_words: usize,
    pub avg_length: f64,
}}

/// The {name} store.
pub struct Store {{
    conn: Connection,
}}

impl Store {{
    /// Open or create a {ext} file.
    pub fn open(path: &Path) -> Result<Self, String> {{
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                word_count INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );"
        ).map_err(|e| e.to_string())?;
        Ok(Self {{ conn }})
    }}

    /// Open an in-memory store (for testing).
    pub fn open_memory() -> Result<Self, String> {{
        Self::open(Path::new(":memory:"))
    }}

    /// Store a message and return it with metadata.
    pub fn send(&self, content: &str) -> Result<StoredMessage, String> {{
        let word_count = content.split_whitespace().count();
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO messages (content, word_count, created_at) VALUES (?1, ?2, ?3)",
            params![content, word_count as i64, now.to_rfc3339()],
        ).map_err(|e| e.to_string())?;
        let id = self.conn.last_insert_rowid();
        Ok(StoredMessage {{ id, content: content.to_string(), word_count, timestamp: now }})
    }}

    /// Query messages by keyword.
    pub fn query(&self, keyword: &str) -> Result<Vec<StoredMessage>, String> {{
        let mut stmt = self.conn.prepare(
            "SELECT id, content, word_count, created_at FROM messages WHERE content LIKE ?1 ORDER BY id DESC"
        ).map_err(|e| e.to_string())?;
        let pattern = format!("%{{}}%", keyword);
        let rows = stmt.query_map(params![pattern], |row| {{
            Ok(StoredMessage {{
                id: row.get(0)?,
                content: row.get(1)?,
                word_count: row.get::<_, i64>(2)? as usize,
                timestamp: parse_dt(&row.get::<_, String>(3)?),
            }})
        }}).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }}

    /// List recent messages.
    pub fn history(&self, limit: usize) -> Result<Vec<StoredMessage>, String> {{
        let mut stmt = self.conn.prepare(
            "SELECT id, content, word_count, created_at FROM messages ORDER BY id DESC LIMIT ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![limit as i64], |row| {{
            Ok(StoredMessage {{
                id: row.get(0)?,
                content: row.get(1)?,
                word_count: row.get::<_, i64>(2)? as usize,
                timestamp: parse_dt(&row.get::<_, String>(3)?),
            }})
        }}).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }}

    /// Compute stats.
    pub fn stats(&self) -> Result<Stats, String> {{
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages", [], |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        let total: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(word_count), 0) FROM messages", [], |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        let avg = if count > 0 {{ total as f64 / count as f64 }} else {{ 0.0 }};
        Ok(Stats {{ message_count: count as usize, total_words: total as usize, avg_length: avg }})
    }}

    /// Clear all messages.
    pub fn clear(&self) -> Result<usize, String> {{
        self.conn.execute("DELETE FROM messages", []).map_err(|e| e.to_string())
    }}
}}

fn parse_dt(s: &str) -> DateTime<Utc> {{
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}}

#[cfg(test)]
mod tests {{
    use super::*;
    #[test]
    fn test_send_and_query() {{
        let store = Store::open_memory().unwrap();
        let msg = store.send("hello world").unwrap();
        assert_eq!(msg.word_count, 2);
        let results = store.query("hello").unwrap();
        assert_eq!(results.len(), 1);
    }}
    #[test]
    fn test_history_and_stats() {{
        let store = Store::open_memory().unwrap();
        store.send("one two three").unwrap();
        store.send("four five").unwrap();
        let stats = store.stats().unwrap();
        assert_eq!(stats.message_count, 2);
        assert_eq!(stats.total_words, 5);
    }}
    #[test]
    fn test_clear() {{
        let store = Store::open_memory().unwrap();
        store.send("test").unwrap();
        assert_eq!(store.clear().unwrap(), 1);
        assert_eq!(store.stats().unwrap().message_count, 0);
    }}
}}
"#,
        name = c.name,
        ext = ext,
    )
}

/// Render the MCP tool registry.
pub(super) fn render_mcp_registry(c: &ProjectConfig) -> String {
    let key_under = c.key.replace('-', "_");
    let mut tool_cases = String::new();
    let mut tool_list_entries = String::new();
    let mut tool_handlers = String::new();

    for tool in &c.tools {
        tool_cases.push_str(&format!(
            r#"            "{tool}" => handle_{tool}(params, store),
"#, tool = tool,
        ));
        let desc = tool_description(tool);
        tool_list_entries.push_str(&format!(
            r#"                    {{"name": "{tool}", "description": "{desc}"}},
"#, tool = tool, desc = desc,
        ));
        tool_handlers.push_str(&render_tool_handler(tool));
    }

    format!(
        r#"//! MCP tool registry — dispatches JSON-RPC requests to tool handlers.

use serde_json::{{json, Value}};
use agentic_{key_under}::Store;

/// Handle a JSON-RPC request line.
pub fn handle_request(line: &str, store: &Store) -> String {{
    let req: Value = match serde_json::from_str(line) {{
        Ok(v) => v,
        Err(e) => return json!({{"jsonrpc":"2.0","id":null,"error":{{"code":-32700,"message":format!("Parse error: {{}}", e)}}}}).to_string(),
    }};
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req["method"].as_str().unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(json!({{}}));
    match method {{
        "tools/list" => {{
            json!({{"jsonrpc":"2.0","id":id,"result":{{"tools":[
{tool_list}                ]}}}}).to_string()
        }}
        "tools/call" => {{
            let name = params["name"].as_str().unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({{}}));
            match dispatch_tool(name, args, store) {{
                Ok(result) => json!({{"jsonrpc":"2.0","id":id,"result":{{"content":[{{"type":"text","text":result}}]}}}}).to_string(),
                Err(e) if e.starts_with("TOOL_NOT_FOUND:") => json!({{"jsonrpc":"2.0","id":id,"error":{{"code":-32803,"message":e}}}}).to_string(),
                Err(e) => json!({{"jsonrpc":"2.0","id":id,"result":{{"content":[{{"type":"text","text":e}}],"isError":true}}}}).to_string(),
            }}
        }}
        "initialize" => json!({{"jsonrpc":"2.0","id":id,"result":{{"protocolVersion":"2024-11-05","capabilities":{{"tools":{{}}}}}}}}).to_string(),
        _ => json!({{"jsonrpc":"2.0","id":id,"error":{{"code":-32601,"message":format!("Method not found: {{}}", method)}}}}).to_string(),
    }}
}}

fn dispatch_tool(name: &str, params: Value, store: &Store) -> Result<String, String> {{
    match name {{
{tool_cases}            _ => Err(format!("TOOL_NOT_FOUND: {{}}", name)),
    }}
}}

{tool_handlers}"#,
        key_under = key_under,
        tool_list = tool_list_entries,
        tool_cases = tool_cases,
        tool_handlers = tool_handlers,
    )
}

fn render_tool_handler(tool: &str) -> String {
    let parts: Vec<&str> = tool.splitn(2, '_').collect();
    let action = parts.get(1).copied().unwrap_or("unknown");
    match action {
        "send" => format!(
            r#"fn handle_{tool}(params: Value, store: &Store) -> Result<String, String> {{
    let content = params["content"].as_str().unwrap_or("");
    if content.is_empty() {{ return Err("Missing 'content' parameter".into()); }}
    let msg = store.send(content)?;
    Ok(serde_json::to_string_pretty(&msg).unwrap_or_default())
}}

"#, tool = tool),
        "query" => format!(
            r#"fn handle_{tool}(params: Value, store: &Store) -> Result<String, String> {{
    let keyword = params["keyword"].as_str().unwrap_or("");
    let results = store.query(keyword)?;
    Ok(serde_json::to_string_pretty(&results).unwrap_or_default())
}}

"#, tool = tool),
        "history" => format!(
            r#"fn handle_{tool}(params: Value, store: &Store) -> Result<String, String> {{
    let limit = params["limit"].as_u64().unwrap_or(10) as usize;
    let results = store.history(limit)?;
    Ok(serde_json::to_string_pretty(&results).unwrap_or_default())
}}

"#, tool = tool),
        "stats" => format!(
            r#"fn handle_{tool}(params: Value, store: &Store) -> Result<String, String> {{
    let stats = store.stats()?;
    Ok(serde_json::to_string_pretty(&stats).unwrap_or_default())
}}

"#, tool = tool),
        "clear" => format!(
            r#"fn handle_{tool}(params: Value, store: &Store) -> Result<String, String> {{
    let deleted = store.clear()?;
    Ok(format!("Cleared {{}} messages", deleted))
}}

"#, tool = tool),
        _ => format!(
            r#"fn handle_{tool}(_params: Value, _store: &Store) -> Result<String, String> {{
    Err("Not yet implemented".into())
}}

"#, tool = tool),
    }
}

fn tool_description(tool: &str) -> String {
    let parts: Vec<&str> = tool.splitn(2, '_').collect();
    let action = parts.get(1).copied().unwrap_or(tool);
    format!("Execute {} operation", action)
}
