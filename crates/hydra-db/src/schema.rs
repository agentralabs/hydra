pub use crate::schema_tables::CREATE_TABLES;

pub const SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version() {
        assert_eq!(SCHEMA_VERSION, 1);
    }

    #[test]
    fn test_create_tables_contains_runs() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS runs"));
    }

    #[test]
    fn test_create_tables_contains_steps() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS steps"));
    }

    #[test]
    fn test_create_tables_contains_checkpoints() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS checkpoints"));
    }

    #[test]
    fn test_create_tables_contains_approvals() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS approvals"));
    }

    #[test]
    fn test_create_tables_contains_schema_version() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS schema_version"));
    }

    #[test]
    fn test_create_tables_has_foreign_keys() {
        assert!(CREATE_TABLES.contains("REFERENCES runs(id)"));
    }

    #[test]
    fn test_create_tables_has_indexes() {
        assert!(CREATE_TABLES.contains("CREATE INDEX IF NOT EXISTS"));
    }

    #[test]
    fn test_create_tables_has_status_checks() {
        assert!(CREATE_TABLES.contains("CHECK(status IN"));
    }

    #[test]
    fn test_create_tables_contains_skills() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS skills"));
    }

    #[test]
    fn test_create_tables_contains_patterns() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS patterns"));
    }

    #[test]
    fn test_create_tables_contains_reflections() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS reflections"));
    }

    #[test]
    fn test_create_tables_contains_temporal_memories() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS temporal_memories"));
    }

    #[test]
    fn test_create_tables_contains_compression_logs() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS compression_logs"));
    }

    #[test]
    fn test_create_tables_contains_receipts() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS receipts"));
    }

    #[test]
    fn test_create_tables_contains_anomaly_events() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS anomaly_events"));
    }

    #[test]
    fn test_create_tables_contains_cursor_sessions() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS cursor_sessions"));
    }

    #[test]
    fn test_create_tables_contains_cursor_events() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS cursor_events"));
    }

    #[test]
    fn test_create_tables_contains_budget_usage() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS budget_usage"));
    }

    #[test]
    fn test_create_tables_contains_evolution_log() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS evolution_log"));
    }

    #[test]
    fn test_create_tables_contains_mutation_log() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS mutation_log"));
    }

    #[test]
    fn test_create_tables_contains_beliefs() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS beliefs"));
    }

    #[test]
    fn test_create_tables_contains_mcp_discovered_skills() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS mcp_discovered_skills"));
    }

    #[test]
    fn test_create_tables_contains_federation_state() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS federation_state"));
    }

    #[test]
    fn test_create_tables_contains_repair_runs() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS repair_runs"));
    }

    #[test]
    fn test_create_tables_contains_repair_checks() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS repair_checks"));
    }
}
