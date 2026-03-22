//! DataFormat — known format types and their properties.

use serde::{Deserialize, Serialize};

/// A data format Hydra can work with.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataFormat {
    // Structural
    Json,
    Toml,
    Yaml,
    Csv,
    Xml,
    // Binary/protocol
    Protobuf,
    Avro,
    Parquet,
    // Domain-specific
    Fix, // Financial Information eXchange
    Hl7, // Health Level 7
    Animus, // Hydra's internal universal format
    // Skill-registered formats
    Custom(String),
}

impl DataFormat {
    pub fn label(&self) -> String {
        match self {
            Self::Json => "json".into(),
            Self::Toml => "toml".into(),
            Self::Yaml => "yaml".into(),
            Self::Csv => "csv".into(),
            Self::Xml => "xml".into(),
            Self::Protobuf => "protobuf".into(),
            Self::Avro => "avro".into(),
            Self::Parquet => "parquet".into(),
            Self::Fix => "fix".into(),
            Self::Hl7 => "hl7".into(),
            Self::Animus => "animus".into(),
            Self::Custom(s) => s.clone(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => Self::Json,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "csv" => Self::Csv,
            "xml" => Self::Xml,
            "protobuf" => Self::Protobuf,
            "avro" => Self::Avro,
            "parquet" => Self::Parquet,
            "fix" => Self::Fix,
            "hl7" => Self::Hl7,
            "animus" => Self::Animus,
            other => Self::Custom(other.to_string()),
        }
    }

    /// Is this a text-based format (vs binary)?
    pub fn is_text(&self) -> bool {
        matches!(
            self,
            Self::Json
                | Self::Toml
                | Self::Yaml
                | Self::Csv
                | Self::Xml
                | Self::Fix
                | Self::Hl7
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_roundtrip() {
        assert_eq!(DataFormat::from_str("json"), DataFormat::Json);
        assert_eq!(DataFormat::from_str("TOML"), DataFormat::Toml);
        assert_eq!(DataFormat::from_str("yml"), DataFormat::Yaml);
        assert_eq!(DataFormat::from_str("xml"), DataFormat::Xml);
        assert_eq!(DataFormat::from_str("protobuf"), DataFormat::Protobuf);
        assert_eq!(DataFormat::from_str("avro"), DataFormat::Avro);
        assert_eq!(DataFormat::from_str("parquet"), DataFormat::Parquet);
        assert_eq!(DataFormat::from_str("fix"), DataFormat::Fix);
        assert_eq!(DataFormat::from_str("hl7"), DataFormat::Hl7);
        assert_eq!(
            DataFormat::from_str("custom-format"),
            DataFormat::Custom("custom-format".into())
        );
    }

    #[test]
    fn text_formats_identified() {
        assert!(DataFormat::Json.is_text());
        assert!(DataFormat::Csv.is_text());
        assert!(DataFormat::Xml.is_text());
        assert!(DataFormat::Fix.is_text());
        assert!(DataFormat::Hl7.is_text());
        assert!(!DataFormat::Protobuf.is_text());
        assert!(!DataFormat::Avro.is_text());
        assert!(!DataFormat::Parquet.is_text());
        assert!(!DataFormat::Animus.is_text());
    }
}
