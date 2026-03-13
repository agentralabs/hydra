//! Intelligence persistence — save/load OutcomeTracker data, calibration
//! buckets, and learned user traits to/from SQLite.

use rusqlite::params;

use crate::store::HydraDb;
use crate::store_types::DbError;

/// A row from the outcome_history table.
#[derive(Debug, Clone)]
pub struct OutcomeRow {
    pub intent_category: String,
    pub topic: String,
    pub model_used: String,
    pub outcome: String,
    pub tokens_used: i64,
}

/// A row from the user_profile_learned table.
#[derive(Debug, Clone)]
pub struct UserTraitRow {
    pub trait_key: String,
    pub trait_value: String,
    pub confidence: f64,
    pub observation_count: i64,
}

impl HydraDb {
    // ── OUTCOME HISTORY ──

    /// Save a single outcome to the history table.
    pub fn save_outcome(
        &self,
        category: &str,
        topic: &str,
        model: &str,
        outcome: &str,
        tokens: u64,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO outcome_history (intent_category, topic, model_used, outcome, tokens_used) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![category, topic, model, outcome, tokens as i64],
        )?;
        Ok(())
    }

    /// Load recent outcomes (most recent first).
    pub fn load_outcomes(&self, limit: u64) -> Result<Vec<OutcomeRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT intent_category, topic, model_used, outcome, tokens_used \
             FROM outcome_history ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(OutcomeRow {
                intent_category: row.get(0)?,
                topic: row.get(1)?,
                model_used: row.get(2)?,
                outcome: row.get(3)?,
                tokens_used: row.get(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Aggregate category stats: (category, total, successes).
    pub fn load_category_stats(&self) -> Result<Vec<(String, u64, u64)>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT intent_category, COUNT(*) as total, \
             SUM(CASE WHEN outcome = 'success' THEN 1 ELSE 0 END) as successes \
             FROM outcome_history GROUP BY intent_category",
        )?;
        let rows = stmt.query_map([], |row| {
            let cat: String = row.get(0)?;
            let total: i64 = row.get(1)?;
            let successes: i64 = row.get(2)?;
            Ok((cat, total as u64, successes as u64))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    // ── CALIBRATION BUCKETS ──

    /// Save all 10 calibration buckets (upsert).
    pub fn save_calibration_buckets(&self, buckets: &[(u64, u64); 10]) -> Result<(), DbError> {
        let conn = self.conn.lock();
        for (i, (total, successes)) in buckets.iter().enumerate() {
            conn.execute(
                "INSERT INTO calibration_buckets (bucket_index, total, successes, updated_at) \
                 VALUES (?1, ?2, ?3, datetime('now')) \
                 ON CONFLICT(bucket_index) DO UPDATE SET \
                 total = ?2, successes = ?3, updated_at = datetime('now')",
                params![i as i64, *total as i64, *successes as i64],
            )?;
        }
        Ok(())
    }

    /// Load calibration buckets from DB. Returns default zeros if empty.
    pub fn load_calibration_buckets(&self) -> Result<[(u64, u64); 10], DbError> {
        let conn = self.conn.lock();
        let mut buckets = [(0u64, 0u64); 10];
        let mut stmt = conn.prepare(
            "SELECT bucket_index, total, successes FROM calibration_buckets ORDER BY bucket_index",
        )?;
        let rows = stmt.query_map([], |row| {
            let idx: i64 = row.get(0)?;
            let total: i64 = row.get(1)?;
            let successes: i64 = row.get(2)?;
            Ok((idx as usize, total as u64, successes as u64))
        })?;
        for row in rows.flatten() {
            if row.0 < 10 {
                buckets[row.0] = (row.1, row.2);
            }
        }
        Ok(buckets)
    }

    // ── USER TRAITS ──

    /// Save or update a learned user trait.
    pub fn save_user_trait(
        &self,
        key: &str,
        value: &str,
        confidence: f64,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO user_profile_learned (trait_key, trait_value, confidence, updated_at) \
             VALUES (?1, ?2, ?3, datetime('now')) \
             ON CONFLICT(trait_key) DO UPDATE SET \
             trait_value = ?2, confidence = ?3, \
             observation_count = observation_count + 1, \
             updated_at = datetime('now')",
            params![key, value, confidence],
        )?;
        Ok(())
    }

    /// Load all learned user traits.
    pub fn load_user_traits(&self) -> Result<Vec<UserTraitRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT trait_key, trait_value, confidence, observation_count \
             FROM user_profile_learned ORDER BY confidence DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(UserTraitRow {
                trait_key: row.get(0)?,
                trait_value: row.get(1)?,
                confidence: row.get(2)?,
                observation_count: row.get(3)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_load_outcome() {
        let db = HydraDb::in_memory().unwrap();
        db.save_outcome("CodeBuild", "rust_async", "sonnet", "success", 1500).unwrap();
        db.save_outcome("CodeBuild", "rust_async", "sonnet", "failure", 800).unwrap();
        db.save_outcome("Deploy", "docker", "haiku", "correction", 500).unwrap();

        let outcomes = db.load_outcomes(10).unwrap();
        assert_eq!(outcomes.len(), 3);
        assert_eq!(outcomes[0].intent_category, "Deploy"); // Most recent first
    }

    #[test]
    fn test_category_stats() {
        let db = HydraDb::in_memory().unwrap();
        for _ in 0..8 {
            db.save_outcome("CodeBuild", "", "sonnet", "success", 1000).unwrap();
        }
        for _ in 0..2 {
            db.save_outcome("CodeBuild", "", "sonnet", "failure", 1000).unwrap();
        }
        db.save_outcome("Deploy", "", "haiku", "success", 500).unwrap();

        let stats = db.load_category_stats().unwrap();
        let code_stat = stats.iter().find(|s| s.0 == "CodeBuild").unwrap();
        assert_eq!(code_stat.1, 10); // total
        assert_eq!(code_stat.2, 8);  // successes
    }

    #[test]
    fn test_save_load_calibration() {
        let db = HydraDb::in_memory().unwrap();
        let mut buckets = [(0u64, 0u64); 10];
        buckets[9] = (20, 18); // 90-100% bucket: 20 predictions, 18 successes
        buckets[5] = (10, 5);  // 50-60% bucket: 10 predictions, 5 successes
        db.save_calibration_buckets(&buckets).unwrap();

        let loaded = db.load_calibration_buckets().unwrap();
        assert_eq!(loaded[9], (20, 18));
        assert_eq!(loaded[5], (10, 5));
        assert_eq!(loaded[0], (0, 0)); // untouched bucket
    }

    #[test]
    fn test_calibration_upsert() {
        let db = HydraDb::in_memory().unwrap();
        let mut buckets = [(0u64, 0u64); 10];
        buckets[9] = (10, 8);
        db.save_calibration_buckets(&buckets).unwrap();

        buckets[9] = (20, 16);
        db.save_calibration_buckets(&buckets).unwrap();

        let loaded = db.load_calibration_buckets().unwrap();
        assert_eq!(loaded[9], (20, 16)); // Updated, not doubled
    }

    #[test]
    fn test_save_load_user_trait() {
        let db = HydraDb::in_memory().unwrap();
        db.save_user_trait("expertise_level", "expert", 0.8).unwrap();
        db.save_user_trait("preferred_language", "rust", 0.9).unwrap();

        let traits = db.load_user_traits().unwrap();
        assert_eq!(traits.len(), 2);
        assert_eq!(traits[0].trait_key, "preferred_language"); // Highest confidence first
    }

    #[test]
    fn test_user_trait_upsert_increments() {
        let db = HydraDb::in_memory().unwrap();
        db.save_user_trait("verbosity", "concise", 0.6).unwrap();
        db.save_user_trait("verbosity", "very_concise", 0.8).unwrap();

        let traits = db.load_user_traits().unwrap();
        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].trait_value, "very_concise");
        assert_eq!(traits[0].observation_count, 2);
    }
}
