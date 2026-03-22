//! TransformEngine — the unified conversion pipeline.

use crate::{
    converter::{convert, ConversionResult},
    errors::TransformError,
    format::DataFormat,
    registry::FormatRegistry,
};

/// The transform engine.
pub struct TransformEngine {
    pub registry: FormatRegistry,
}

impl TransformEngine {
    pub fn new() -> Self {
        Self {
            registry: FormatRegistry::new(),
        }
    }

    /// Convert data from one format to another.
    pub fn transform(
        &self,
        data: &str,
        from: &DataFormat,
        to: &DataFormat,
    ) -> Result<ConversionResult, TransformError> {
        convert(data, from, to)
    }

    /// Convert data with auto-detected source format.
    pub fn transform_auto(
        &self,
        data: &str,
        from_hint: &str,
        to: &DataFormat,
    ) -> Result<ConversionResult, TransformError> {
        let from = self
            .registry
            .detect(from_hint)
            .or_else(|| self.registry.detect(data))
            .unwrap_or(DataFormat::Json); // default assumption
        convert(data, &from, to)
    }

    /// Convert a sister output to Animus Prime (the universal substrate).
    pub fn sister_to_animus(&self, output: &str) -> ConversionResult {
        convert(output, &DataFormat::Json, &DataFormat::Animus)
            .unwrap_or_else(|_| ConversionResult {
                data: format!("\u{27e8}animus:raw\u{27e9}{}", output),
                from: "unknown".into(),
                to: "animus".into(),
                confidence: 0.5,
                chain: vec!["unknown".into(), "animus".into()],
            })
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!("transform: formats={}", self.registry.count())
    }
}

impl Default for TransformEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_to_animus() {
        let engine = TransformEngine::new();
        let r = engine
            .transform(
                r#"{"event":"deploy_failed","cause":"auth"}"#,
                &DataFormat::Json,
                &DataFormat::Animus,
            )
            .expect("ok");
        assert!(r.data.contains("animus"));
        assert!(r.confidence > 0.5);
    }

    #[test]
    fn csv_to_json() {
        let engine = TransformEngine::new();
        let r = engine
            .transform(
                "name,value\nalpha,1\nbeta,2",
                &DataFormat::Csv,
                &DataFormat::Json,
            )
            .expect("ok");
        assert!(r.data.contains("alpha") || r.data.contains("name"));
    }

    #[test]
    fn sister_output_to_animus() {
        let engine = TransformEngine::new();
        let r = engine
            .sister_to_animus(r#"{"memories":3,"confidence":0.87}"#);
        assert!(!r.data.is_empty());
    }

    #[test]
    fn summary_format() {
        let engine = TransformEngine::new();
        let s = engine.summary();
        assert!(s.contains("transform:"));
    }
}
