//! Signal types from each sister for threat correlation.

use chrono::{DateTime, Utc};

/// Which sister reported a signal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SisterName {
    Memory,
    Identity,
    Codebase,
    Vision,
    Comm,
    Contract,
    Time,
    Planning,
    Cognition,
    Reality,
    Forge,
    Aegis,
    Veritas,
    Evolve,
}

impl SisterName {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Memory => "Memory",
            Self::Identity => "Identity",
            Self::Codebase => "Codebase",
            Self::Vision => "Vision",
            Self::Comm => "Comm",
            Self::Contract => "Contract",
            Self::Time => "Time",
            Self::Planning => "Planning",
            Self::Cognition => "Cognition",
            Self::Reality => "Reality",
            Self::Forge => "Forge",
            Self::Aegis => "Aegis",
            Self::Veritas => "Veritas",
            Self::Evolve => "Evolve",
        }
    }
}

impl std::fmt::Display for SisterName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The kind of suspicious signal detected.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalType {
    /// Aegis: unusual input pattern
    InputAnomaly,
    /// Contract: boundary testing
    PolicyProbe,
    /// Identity: trust score changed unexpectedly
    TrustDrift,
    /// Comm: unusual message volume
    TrafficSpike,
    /// Memory: unauthorized belief modification
    MemoryTampering,
    /// Reality: unexpected process/file change
    EnvironmentChange,
    /// Cognition: agent acting outside baseline
    BehavioralDrift,
    /// Identity: repeated auth failures
    AuthFailure,
    /// Any: tool used in unexpected way
    ToolMisuse,
    /// Any: data leaving to unknown destination
    DataExfiltration,
}

impl SignalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InputAnomaly => "InputAnomaly",
            Self::PolicyProbe => "PolicyProbe",
            Self::TrustDrift => "TrustDrift",
            Self::TrafficSpike => "TrafficSpike",
            Self::MemoryTampering => "MemoryTampering",
            Self::EnvironmentChange => "EnvironmentChange",
            Self::BehavioralDrift => "BehavioralDrift",
            Self::AuthFailure => "AuthFailure",
            Self::ToolMisuse => "ToolMisuse",
            Self::DataExfiltration => "DataExfiltration",
        }
    }

    /// Base severity weight for this signal type.
    pub fn base_weight(&self) -> f32 {
        match self {
            Self::DataExfiltration => 0.9,
            Self::MemoryTampering => 0.85,
            Self::AuthFailure => 0.7,
            Self::PolicyProbe => 0.6,
            Self::ToolMisuse => 0.65,
            Self::TrustDrift => 0.5,
            Self::BehavioralDrift => 0.5,
            Self::InputAnomaly => 0.4,
            Self::TrafficSpike => 0.35,
            Self::EnvironmentChange => 0.3,
        }
    }
}

impl std::fmt::Display for SignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single threat signal reported by a sister.
#[derive(Debug, Clone)]
pub struct ThreatSignal {
    pub source: SisterName,
    pub signal_type: SignalType,
    pub severity: f32,
    pub timestamp: DateTime<Utc>,
    pub details: String,
}

impl ThreatSignal {
    pub fn new(source: SisterName, signal_type: SignalType, severity: f32, details: &str) -> Self {
        Self {
            source,
            signal_type,
            severity: severity.clamp(0.0, 1.0),
            timestamp: Utc::now(),
            details: details.to_string(),
        }
    }

    /// Weighted severity = base weight * reported severity.
    pub fn weighted_severity(&self) -> f32 {
        self.signal_type.base_weight() * self.severity
    }
}

/// A known attack pattern that has been seen before.
#[derive(Debug, Clone)]
pub struct AttackPattern {
    pub name: String,
    pub signal_sequence: Vec<SignalType>,
    pub min_sisters: usize,
    pub window_secs: u64,
    pub description: String,
}

impl AttackPattern {
    /// Built-in patterns from threat intelligence.
    pub fn known_patterns() -> Vec<Self> {
        vec![
            Self {
                name: "Prompt Injection Probe".into(),
                signal_sequence: vec![SignalType::InputAnomaly, SignalType::PolicyProbe],
                min_sisters: 2,
                window_secs: 10,
                description: "Input anomaly followed by policy boundary testing".into(),
            },
            Self {
                name: "Credential Stuffing".into(),
                signal_sequence: vec![SignalType::AuthFailure, SignalType::AuthFailure, SignalType::TrustDrift],
                min_sisters: 1,
                window_secs: 30,
                description: "Repeated auth failures followed by trust drift".into(),
            },
            Self {
                name: "Data Exfil Attempt".into(),
                signal_sequence: vec![SignalType::ToolMisuse, SignalType::DataExfiltration],
                min_sisters: 2,
                window_secs: 15,
                description: "Tool misuse followed by data exfiltration".into(),
            },
            Self {
                name: "Coordinated Disruption".into(),
                signal_sequence: vec![
                    SignalType::TrafficSpike, SignalType::BehavioralDrift,
                    SignalType::EnvironmentChange,
                ],
                min_sisters: 3,
                window_secs: 5,
                description: "Multi-sister anomalies within short window".into(),
            },
            Self {
                name: "Memory Poisoning".into(),
                signal_sequence: vec![SignalType::MemoryTampering, SignalType::BehavioralDrift],
                min_sisters: 2,
                window_secs: 60,
                description: "Belief manipulation causing behavioral changes".into(),
            },
        ]
    }
}

/// System baseline — normal operating parameters.
#[derive(Debug, Clone)]
pub struct SystemBaseline {
    pub avg_signals_per_hour: f32,
    pub avg_severity: f32,
    pub active_sisters: usize,
    pub normal_signal_types: Vec<SignalType>,
}

impl Default for SystemBaseline {
    fn default() -> Self {
        Self {
            avg_signals_per_hour: 2.0,
            avg_severity: 0.2,
            active_sisters: 14,
            normal_signal_types: vec![
                SignalType::EnvironmentChange,
                SignalType::TrafficSpike,
            ],
        }
    }
}
