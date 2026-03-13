use rusqlite::params;

use crate::store::HydraDb;
use crate::store_types::DbError;

// ═══════════════════════════════════════════════════════════
// CODE INDEX TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CodeSymbolRow {
    pub id: i64,
    pub file_path: String,
    pub symbol_name: String,
    pub symbol_type: String,
    pub line_number: i64,
    pub visibility: String,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CodeEdgeRow {
    pub id: i64,
    pub from_symbol: String,
    pub to_symbol: String,
    pub edge_type: String,
    pub file_path: String,
    pub updated_at: String,
}

// ═══════════════════════════════════════════════════════════
// CODE INDEX STORE METHODS
// ═══════════════════════════════════════════════════════════

impl HydraDb {
    /// Insert or replace a code symbol entry.
    pub fn upsert_code_symbol(
        &self,
        file_path: &str,
        symbol_name: &str,
        symbol_type: &str,
        line_number: i64,
        visibility: &str,
        signature: Option<&str>,
        doc_comment: Option<&str>,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO code_symbols \
             (file_path, symbol_name, symbol_type, line_number, visibility, signature, doc_comment, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))",
            params![file_path, symbol_name, symbol_type, line_number, visibility, signature, doc_comment],
        )?;
        Ok(())
    }

    /// Delete all symbols for a given file path. Returns count deleted.
    pub fn delete_symbols_for_file(&self, file_path: &str) -> Result<usize, DbError> {
        let conn = self.conn.lock();
        let count = conn.execute(
            "DELETE FROM code_symbols WHERE file_path = ?1",
            params![file_path],
        )?;
        Ok(count)
    }

    /// Query symbols by name pattern (LIKE match). Returns up to `limit` rows.
    pub fn query_symbols(&self, name_pattern: &str, limit: usize) -> Result<Vec<CodeSymbolRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, symbol_name, symbol_type, line_number, visibility, \
             signature, doc_comment, updated_at \
             FROM code_symbols WHERE symbol_name LIKE ?1 LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![name_pattern, limit as i64], |row| {
            Ok(CodeSymbolRow {
                id: row.get(0)?,
                file_path: row.get(1)?,
                symbol_name: row.get(2)?,
                symbol_type: row.get(3)?,
                line_number: row.get(4)?,
                visibility: row.get(5)?,
                signature: row.get(6)?,
                doc_comment: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Query all symbols in a specific file.
    pub fn query_symbols_in_file(&self, file_path: &str) -> Result<Vec<CodeSymbolRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, symbol_name, symbol_type, line_number, visibility, \
             signature, doc_comment, updated_at \
             FROM code_symbols WHERE file_path = ?1 ORDER BY line_number",
        )?;
        let rows = stmt.query_map(params![file_path], |row| {
            Ok(CodeSymbolRow {
                id: row.get(0)?,
                file_path: row.get(1)?,
                symbol_name: row.get(2)?,
                symbol_type: row.get(3)?,
                line_number: row.get(4)?,
                visibility: row.get(5)?,
                signature: row.get(6)?,
                doc_comment: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Insert or replace a code edge.
    pub fn upsert_code_edge(
        &self,
        from_symbol: &str,
        to_symbol: &str,
        edge_type: &str,
        file_path: &str,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO code_edges \
             (from_symbol, to_symbol, edge_type, file_path, updated_at) \
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![from_symbol, to_symbol, edge_type, file_path],
        )?;
        Ok(())
    }

    /// Query all edges originating from a symbol.
    pub fn query_edges_from(&self, symbol: &str) -> Result<Vec<CodeEdgeRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, from_symbol, to_symbol, edge_type, file_path, updated_at \
             FROM code_edges WHERE from_symbol = ?1",
        )?;
        let rows = stmt.query_map(params![symbol], |row| {
            Ok(CodeEdgeRow {
                id: row.get(0)?,
                from_symbol: row.get(1)?,
                to_symbol: row.get(2)?,
                edge_type: row.get(3)?,
                file_path: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Return total number of indexed symbols.
    pub fn symbol_count(&self) -> Result<usize, DbError> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM code_symbols",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> HydraDb {
        HydraDb::in_memory().expect("in-memory db")
    }

    #[test]
    fn test_upsert_and_query_symbol() {
        let db = test_db();
        db.upsert_code_symbol("src/lib.rs", "init", "function", 10, "public", Some("pub fn init()"), None)
            .unwrap();
        let results = db.query_symbols("init", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol_name, "init");
        assert_eq!(results[0].symbol_type, "function");
        assert_eq!(results[0].visibility, "public");
    }

    #[test]
    fn test_delete_symbols_for_file() {
        let db = test_db();
        db.upsert_code_symbol("src/a.rs", "foo", "function", 1, "public", None, None).unwrap();
        db.upsert_code_symbol("src/a.rs", "bar", "struct", 20, "private", None, None).unwrap();
        db.upsert_code_symbol("src/b.rs", "baz", "function", 5, "public", None, None).unwrap();
        let deleted = db.delete_symbols_for_file("src/a.rs").unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(db.symbol_count().unwrap(), 1);
    }

    #[test]
    fn test_query_symbols_in_file() {
        let db = test_db();
        db.upsert_code_symbol("src/lib.rs", "alpha", "function", 10, "public", None, None).unwrap();
        db.upsert_code_symbol("src/lib.rs", "beta", "struct", 30, "private", None, None).unwrap();
        db.upsert_code_symbol("src/other.rs", "gamma", "enum", 1, "public", None, None).unwrap();
        let syms = db.query_symbols_in_file("src/lib.rs").unwrap();
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].symbol_name, "alpha"); // line 10 first
        assert_eq!(syms[1].symbol_name, "beta");  // line 30 second
    }

    #[test]
    fn test_upsert_and_query_edges() {
        let db = test_db();
        db.upsert_code_edge("main", "init", "calls", "src/main.rs").unwrap();
        db.upsert_code_edge("main", "run", "calls", "src/main.rs").unwrap();
        let edges = db.query_edges_from("main").unwrap();
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].to_symbol, "init");
    }

    #[test]
    fn test_symbol_count_empty() {
        let db = test_db();
        assert_eq!(db.symbol_count().unwrap(), 0);
    }
}
