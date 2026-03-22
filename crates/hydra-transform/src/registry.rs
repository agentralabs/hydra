//! FormatRegistry — skill-registered format vocabularies.
//! Skills register new formats here on load.

use crate::{constants::MAX_FORMAT_VOCABULARIES, format::DataFormat};
use std::collections::HashMap;

/// A skill-registered format vocabulary.
#[derive(Debug, Clone)]
pub struct FormatVocabulary {
    pub skill: String,
    pub format: DataFormat,
    pub description: String,
    pub keywords: Vec<String>, // file extensions, MIME types, identifiers
}

/// The format registry.
#[derive(Debug, Default)]
pub struct FormatRegistry {
    vocabularies: HashMap<String, FormatVocabulary>,
}

impl FormatRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, vocab: FormatVocabulary) -> bool {
        if self.vocabularies.len() >= MAX_FORMAT_VOCABULARIES {
            return false;
        }
        self.vocabularies.insert(vocab.format.label(), vocab);
        true
    }

    pub fn unregister_skill(&mut self, skill: &str) -> usize {
        let before = self.vocabularies.len();
        self.vocabularies.retain(|_, v| v.skill != skill);
        before - self.vocabularies.len()
    }

    /// Detect format from file extension or content hint.
    pub fn detect(&self, hint: &str) -> Option<DataFormat> {
        let lower = hint.to_lowercase();
        // Check registered vocabularies first
        for vocab in self.vocabularies.values() {
            if vocab
                .keywords
                .iter()
                .any(|k| lower.contains(k.as_str()))
            {
                return Some(vocab.format.clone());
            }
        }
        // Fall back to built-in detection
        if lower.ends_with(".json")
            || lower.starts_with('{')
            || lower.starts_with('[')
        {
            return Some(DataFormat::Json);
        }
        if lower.ends_with(".toml") {
            return Some(DataFormat::Toml);
        }
        if lower.ends_with(".csv") {
            return Some(DataFormat::Csv);
        }
        if lower.ends_with(".yaml") || lower.ends_with(".yml") {
            return Some(DataFormat::Yaml);
        }
        None
    }

    pub fn count(&self) -> usize {
        self.vocabularies.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_detect() {
        let mut reg = FormatRegistry::new();
        reg.register(FormatVocabulary {
            skill: "video-skill".into(),
            format: DataFormat::Custom("prores".into()),
            description: "Apple ProRes video format".into(),
            keywords: vec!["prores".into(), ".mov".into()],
        });
        let detected = reg.detect("video.prores");
        assert!(matches!(detected, Some(DataFormat::Custom(_))));
    }

    #[test]
    fn builtin_json_detection() {
        let reg = FormatRegistry::new();
        let d = reg.detect("{\"key\": \"value\"}");
        assert_eq!(d, Some(DataFormat::Json));
    }

    #[test]
    fn unregister_skill_removes_formats() {
        let mut reg = FormatRegistry::new();
        reg.register(FormatVocabulary {
            skill: "skill-a".into(),
            format: DataFormat::Custom("format-a".into()),
            description: "test".into(),
            keywords: vec!["fa".into()],
        });
        assert_eq!(reg.count(), 1);
        reg.unregister_skill("skill-a");
        assert_eq!(reg.count(), 0);
    }
}
