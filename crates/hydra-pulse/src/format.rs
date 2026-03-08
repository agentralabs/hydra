//! .apulse file format — persists pulse state across sessions.

use serde::{Deserialize, Serialize};

/// Magic bytes for .apulse files
pub const APULSE_MAGIC: &[u8; 6] = b"APULSE";

/// File format version
pub const APULSE_VERSION: u8 = 1;

/// A single entry in the pulse state file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: String,
}

/// The complete pulse state persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseState {
    pub version: u8,
    /// Cached patterns for the predictor
    pub patterns: Vec<PulsePatternEntry>,
    /// Learned resonance preferences
    pub preferences: Vec<PulsePreferenceEntry>,
    /// Watch specifications for proactive engine
    pub watches: Vec<PulseWatchEntry>,
    /// Metadata
    pub last_saved: String,
    pub session_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulsePatternEntry {
    pub input: String,
    pub response: String,
    pub confidence: f64,
    pub hit_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulsePreferenceEntry {
    pub dimension: String,
    pub value: f64,
    pub observations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseWatchEntry {
    pub id: String,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub description: String,
    pub enabled: bool,
}

impl PulseState {
    /// Create an empty pulse state
    pub fn empty() -> Self {
        Self {
            version: APULSE_VERSION,
            patterns: Vec::new(),
            preferences: Vec::new(),
            watches: Vec::new(),
            last_saved: chrono::Utc::now().to_rfc3339(),
            session_count: 0,
        }
    }

    /// Serialize to JSON bytes (with magic header for identification)
    pub fn to_bytes(&self) -> Vec<u8> {
        let json = serde_json::to_vec_pretty(self).unwrap_or_default();
        let mut bytes = Vec::with_capacity(APULSE_MAGIC.len() + 1 + json.len());
        bytes.extend_from_slice(APULSE_MAGIC);
        bytes.push(APULSE_VERSION);
        bytes.extend_from_slice(&json);
        bytes
    }

    /// Deserialize from bytes (validates magic header)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PulseFormatError> {
        if bytes.len() < APULSE_MAGIC.len() + 1 {
            return Err(PulseFormatError::TooShort);
        }

        let magic = &bytes[..APULSE_MAGIC.len()];
        if magic != APULSE_MAGIC {
            return Err(PulseFormatError::InvalidMagic);
        }

        let version = bytes[APULSE_MAGIC.len()];
        if version != APULSE_VERSION {
            return Err(PulseFormatError::UnsupportedVersion(version));
        }

        let json = &bytes[APULSE_MAGIC.len() + 1..];
        serde_json::from_slice(json).map_err(PulseFormatError::JsonError)
    }
}

#[derive(Debug)]
pub enum PulseFormatError {
    TooShort,
    InvalidMagic,
    UnsupportedVersion(u8),
    JsonError(serde_json::Error),
}

impl std::fmt::Display for PulseFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "File too short to be .apulse"),
            Self::InvalidMagic => write!(f, "Invalid .apulse magic bytes"),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported .apulse version: {}", v),
            Self::JsonError(e) => write!(f, "JSON parse error: {}", e),
        }
    }
}

impl std::error::Error for PulseFormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_state() {
        let state = PulseState::empty();
        assert_eq!(state.version, APULSE_VERSION);
        assert!(state.patterns.is_empty());
        assert!(state.preferences.is_empty());
    }

    #[test]
    fn test_roundtrip() {
        let mut state = PulseState::empty();
        state.patterns.push(PulsePatternEntry {
            input: "hello".into(),
            response: "hi there".into(),
            confidence: 0.8,
            hit_count: 5,
        });
        state.preferences.push(PulsePreferenceEntry {
            dimension: "verbosity".into(),
            value: 0.7,
            observations: 10,
        });
        state.session_count = 42;

        let bytes = state.to_bytes();
        let restored = PulseState::from_bytes(&bytes).unwrap();

        assert_eq!(restored.version, APULSE_VERSION);
        assert_eq!(restored.patterns.len(), 1);
        assert_eq!(restored.patterns[0].input, "hello");
        assert_eq!(restored.preferences.len(), 1);
        assert_eq!(restored.session_count, 42);
    }

    #[test]
    fn test_invalid_magic() {
        let result = PulseState::from_bytes(b"WRONG\x01{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_too_short() {
        let result = PulseState::from_bytes(b"AP");
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_version() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(APULSE_MAGIC);
        bytes.push(99); // unsupported version
        bytes.extend_from_slice(b"{}");
        let result = PulseState::from_bytes(&bytes);
        assert!(matches!(
            result,
            Err(PulseFormatError::UnsupportedVersion(99))
        ));
    }
}
