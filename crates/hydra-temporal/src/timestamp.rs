//! Nanosecond-precision timestamps for Hydra's temporal layer.

use crate::constants::{NANOS_PER_MS, NANOS_PER_SECOND};
use crate::errors::TemporalError;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A nanosecond-precision timestamp. Zero is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Create a timestamp from a raw nanosecond value.
    ///
    /// Returns an error if `nanos` is zero.
    pub fn from_nanos(nanos: u64) -> Result<Self, TemporalError> {
        if nanos == 0 {
            return Err(TemporalError::InvalidTimestamp(
                "zero is not a valid timestamp".to_string(),
            ));
        }
        Ok(Self(nanos))
    }

    /// Create a timestamp representing the current time.
    pub fn now() -> Self {
        let nanos = Utc::now().timestamp_nanos_opt().unwrap_or(1) as u64;
        // Safety: current time is never zero
        Self(if nanos == 0 { 1 } else { nanos })
    }

    /// Return the raw nanosecond value.
    pub fn as_nanos(&self) -> u64 {
        self.0
    }

    /// Convert to a `chrono::DateTime<Utc>`.
    pub fn to_datetime(&self) -> DateTime<Utc> {
        let secs = (self.0 / NANOS_PER_SECOND) as i64;
        let nsec = (self.0 % NANOS_PER_SECOND) as u32;
        Utc.timestamp_opt(secs, nsec).single().unwrap_or_default()
    }

    /// Create a timestamp from a `chrono::DateTime<Utc>`.
    pub fn from_datetime(dt: DateTime<Utc>) -> Result<Self, TemporalError> {
        let nanos = dt.timestamp_nanos_opt().ok_or_else(|| {
            TemporalError::InvalidTimestamp("datetime out of nanosecond range".to_string())
        })? as u64;
        Self::from_nanos(nanos)
    }

    /// Format as an RFC 3339 string.
    pub fn to_rfc3339(&self) -> String {
        self.to_datetime().to_rfc3339()
    }

    /// Absolute difference in nanoseconds between two timestamps.
    pub fn delta_nanos(&self, other: &Timestamp) -> u64 {
        self.0.abs_diff(other.0)
    }

    /// Absolute difference in milliseconds between two timestamps.
    pub fn delta_ms(&self, other: &Timestamp) -> u64 {
        self.delta_nanos(other) / NANOS_PER_MS
    }

    /// Check whether two timestamps are within `threshold_ns` nanoseconds.
    pub fn is_near(&self, other: &Timestamp, threshold_ns: u64) -> bool {
        self.delta_nanos(other) <= threshold_ns
    }

    /// Gaussian similarity: e^(-(delta^2) / (2 * sigma^2)).
    ///
    /// Returns a value in (0.0, 1.0] where 1.0 means identical timestamps.
    pub fn gaussian_similarity(&self, other: &Timestamp, sigma_ns: f64) -> f64 {
        let delta = self.delta_nanos(other) as f64;
        (-delta * delta / (2.0 * sigma_ns * sigma_ns)).exp()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_rfc3339())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_invalid() {
        assert!(Timestamp::from_nanos(0).is_err());
    }

    #[test]
    fn roundtrip_nanos() {
        let ts = Timestamp::from_nanos(1_700_000_000_000_000_000).unwrap();
        assert_eq!(ts.as_nanos(), 1_700_000_000_000_000_000);
    }

    #[test]
    fn now_is_valid() {
        let ts = Timestamp::now();
        assert!(ts.as_nanos() > 0);
    }

    #[test]
    fn datetime_roundtrip() {
        let ts = Timestamp::from_nanos(1_700_000_000_123_456_789).unwrap();
        let dt = ts.to_datetime();
        let ts2 = Timestamp::from_datetime(dt).unwrap();
        assert_eq!(ts.as_nanos(), ts2.as_nanos());
    }

    #[test]
    fn delta_and_near() {
        let a = Timestamp::from_nanos(1_000_000_000).unwrap();
        let b = Timestamp::from_nanos(1_000_500_000).unwrap();
        assert_eq!(a.delta_nanos(&b), 500_000);
        assert!(a.is_near(&b, 1_000_000));
        assert!(!a.is_near(&b, 100));
    }

    #[test]
    fn gaussian_identical_is_one() {
        let ts = Timestamp::from_nanos(1_000_000_000).unwrap();
        let sim = ts.gaussian_similarity(&ts, 1_000_000.0);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn gaussian_distant_approaches_zero() {
        let a = Timestamp::from_nanos(1_000_000_000).unwrap();
        let b = Timestamp::from_nanos(9_000_000_000_000_000_000).unwrap();
        let sim = a.gaussian_similarity(&b, 1_000_000.0);
        assert!(sim < 1e-10);
    }
}
