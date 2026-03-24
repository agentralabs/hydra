//! SessionManager — persists browser cookies per domain.
//! Saves to ~/.hydra/browser/cookies/<domain>.json.

use crate::constants::{COOKIE_DIR, SESSION_STALE_SECONDS};
use crate::errors::BrowserError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A stored cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub expires: Option<i64>,
}

/// Session state for a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSession {
    pub domain: String,
    pub cookies: Vec<StoredCookie>,
    pub saved_at: DateTime<Utc>,
    pub login_verified: bool,
}

impl DomainSession {
    /// Check if this session might be stale and needs re-validation.
    pub fn is_stale(&self) -> bool {
        let age = Utc::now().signed_duration_since(self.saved_at);
        age.num_seconds() > SESSION_STALE_SECONDS
    }
}

/// Manages cookie persistence across browser sessions.
pub struct SessionManager {
    sessions: HashMap<String, DomainSession>,
    base_dir: PathBuf,
}

impl SessionManager {
    pub fn new() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".hydra")
            .join(COOKIE_DIR);
        Self {
            sessions: HashMap::new(),
            base_dir,
        }
    }

    /// Save cookies for a domain.
    pub fn save_cookies(
        &mut self,
        domain: &str,
        cookies: Vec<StoredCookie>,
    ) -> Result<(), BrowserError> {
        let session = DomainSession {
            domain: domain.to_string(),
            cookies,
            saved_at: Utc::now(),
            login_verified: true,
        };

        // Persist to disk
        if let Err(e) = std::fs::create_dir_all(&self.base_dir) {
            eprintln!("hydra-browser: failed to create cookie dir: {e}");
            return Err(BrowserError::SessionError {
                domain: domain.into(),
                reason: format!("Cannot create cookie directory: {e}"),
            });
        }

        let path = self.cookie_path(domain);
        let json = serde_json::to_string_pretty(&session).map_err(|e| {
            BrowserError::SessionError {
                domain: domain.into(),
                reason: format!("Serialization error: {e}"),
            }
        })?;

        std::fs::write(&path, json).map_err(|e| BrowserError::SessionError {
            domain: domain.into(),
            reason: format!("Write error: {e}"),
        })?;

        eprintln!("hydra-browser: saved session for {domain} ({} cookies)", session.cookies.len());
        self.sessions.insert(domain.to_string(), session);
        Ok(())
    }

    /// Load cookies for a domain (from memory cache or disk).
    pub fn load_cookies(&mut self, domain: &str) -> Option<&DomainSession> {
        if self.sessions.contains_key(domain) {
            return self.sessions.get(domain);
        }

        // Try loading from disk
        let path = self.cookie_path(domain);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(session) = serde_json::from_str::<DomainSession>(&content) {
                eprintln!(
                    "hydra-browser: loaded session for {domain} from disk ({} cookies)",
                    session.cookies.len()
                );
                self.sessions.insert(domain.to_string(), session);
                return self.sessions.get(domain);
            }
        }
        None
    }

    /// Clear session for a domain.
    pub fn clear_session(&mut self, domain: &str) -> Result<(), BrowserError> {
        self.sessions.remove(domain);
        let path = self.cookie_path(domain);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| BrowserError::SessionError {
                domain: domain.into(),
                reason: format!("Delete error: {e}"),
            })?;
        }
        eprintln!("hydra-browser: cleared session for {domain}");
        Ok(())
    }

    /// Check if we have a valid (non-stale) session for this domain.
    pub fn has_valid_session(&mut self, domain: &str) -> bool {
        if let Some(session) = self.load_cookies(domain) {
            !session.is_stale() && session.login_verified
        } else {
            false
        }
    }

    pub fn cached_domains(&self) -> Vec<&str> {
        self.sessions.keys().map(|s| s.as_str()).collect()
    }

    fn cookie_path(&self, domain: &str) -> PathBuf {
        let safe_name = domain.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        self.base_dir.join(format!("{safe_name}.json"))
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_session_mgr() -> SessionManager {
        let dir = std::env::temp_dir().join(format!("hydra-test-{}", uuid::Uuid::new_v4()));
        SessionManager {
            sessions: HashMap::new(),
            base_dir: dir,
        }
    }

    #[test]
    fn save_and_load_cookies() {
        let mut mgr = temp_session_mgr();
        let cookies = vec![StoredCookie {
            name: "session_id".into(),
            value: "abc123".into(),
            domain: "example.com".into(),
            path: "/".into(),
            secure: true,
            http_only: true,
            expires: None,
        }];

        mgr.save_cookies("example.com", cookies).unwrap();
        let loaded = mgr.load_cookies("example.com");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().cookies.len(), 1);
        assert_eq!(loaded.unwrap().cookies[0].value, "abc123");

        // Cleanup
        let _ = std::fs::remove_dir_all(&mgr.base_dir);
    }

    #[test]
    fn clear_session_removes_data() {
        let mut mgr = temp_session_mgr();
        mgr.save_cookies("test.com", vec![]).unwrap();
        assert!(mgr.has_valid_session("test.com"));
        mgr.clear_session("test.com").unwrap();
        assert!(!mgr.has_valid_session("test.com"));
        let _ = std::fs::remove_dir_all(&mgr.base_dir);
    }

    #[test]
    fn stale_session_detected() {
        let session = DomainSession {
            domain: "old.com".into(),
            cookies: vec![],
            saved_at: Utc::now() - chrono::Duration::hours(2),
            login_verified: true,
        };
        assert!(session.is_stale());
    }

    #[test]
    fn fresh_session_not_stale() {
        let session = DomainSession {
            domain: "new.com".into(),
            cookies: vec![],
            saved_at: Utc::now(),
            login_verified: true,
        };
        assert!(!session.is_stale());
    }
}
