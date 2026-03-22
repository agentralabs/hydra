//! Threat classification and signals.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Classification of threat types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreatClass {
    /// Prompt injection attack.
    PromptInjection,
    /// Data exfiltration attempt.
    DataExfiltration,
    /// Privilege escalation.
    PrivilegeEscalation,
    /// Resource exhaustion (DoS).
    ResourceExhaustion,
    /// Identity spoofing.
    IdentitySpoofing,
    /// Supply chain compromise.
    SupplyChain,
    /// Model poisoning.
    ModelPoisoning,
    /// Constitutional violation attempt.
    ConstitutionalViolation,
    /// Causal chain manipulation.
    CausalChainManipulation,
    /// Receipt tampering.
    ReceiptTampering,
    /// Memory corruption.
    MemoryCorruption,
    /// Trust score manipulation.
    TrustManipulation,
    /// Side channel attack.
    SideChannel,
    /// Social engineering.
    SocialEngineering,
    /// Unknown or novel threat.
    Unknown,
}

impl ThreatClass {
    /// Return the severity of this threat class (0.0 to 1.0).
    pub fn severity(&self) -> f64 {
        match self {
            Self::ConstitutionalViolation => 1.0,
            Self::CausalChainManipulation => 0.95,
            Self::ReceiptTampering => 0.95,
            Self::PrivilegeEscalation => 0.9,
            Self::IdentitySpoofing => 0.85,
            Self::ModelPoisoning => 0.85,
            Self::MemoryCorruption => 0.8,
            Self::TrustManipulation => 0.8,
            Self::DataExfiltration => 0.75,
            Self::SupplyChain => 0.75,
            Self::PromptInjection => 0.7,
            Self::ResourceExhaustion => 0.6,
            Self::SideChannel => 0.6,
            Self::SocialEngineering => 0.5,
            Self::Unknown => 0.5,
        }
    }

    /// Return true if this threat class targets the constitution.
    pub fn is_constitutional(&self) -> bool {
        matches!(
            self,
            Self::ConstitutionalViolation | Self::CausalChainManipulation | Self::ReceiptTampering
        )
    }

    /// Return a human-readable label.
    pub fn label(&self) -> &str {
        match self {
            Self::PromptInjection => "prompt-injection",
            Self::DataExfiltration => "data-exfiltration",
            Self::PrivilegeEscalation => "privilege-escalation",
            Self::ResourceExhaustion => "resource-exhaustion",
            Self::IdentitySpoofing => "identity-spoofing",
            Self::SupplyChain => "supply-chain",
            Self::ModelPoisoning => "model-poisoning",
            Self::ConstitutionalViolation => "constitutional-violation",
            Self::CausalChainManipulation => "causal-chain-manipulation",
            Self::ReceiptTampering => "receipt-tampering",
            Self::MemoryCorruption => "memory-corruption",
            Self::TrustManipulation => "trust-manipulation",
            Self::SideChannel => "side-channel",
            Self::SocialEngineering => "social-engineering",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for ThreatClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A signal indicating a potential threat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatSignal {
    /// Unique identifier.
    pub id: Uuid,
    /// Classification of the threat.
    pub class: ThreatClass,
    /// Feature vector for matching against antibodies.
    pub features: Vec<f64>,
    /// Source identifier (agent, external, etc.).
    pub source: String,
    /// Human-readable description.
    pub description: String,
    /// When this signal was generated.
    pub timestamp: DateTime<Utc>,
}

impl ThreatSignal {
    /// Create a new threat signal.
    pub fn new(
        class: ThreatClass,
        features: Vec<f64>,
        source: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            class,
            features,
            source: source.into(),
            description: description.into(),
            timestamp: Utc::now(),
        }
    }
}
