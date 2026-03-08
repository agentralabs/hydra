use dashmap::DashMap;
use hydra_core::types::CompiledIntent;

/// Fuzzy intent matcher — Layer 3 of the 4-layer escalation (0 tokens)
/// Uses Jaccard similarity on word sets with a configurable threshold.
pub struct FuzzyMatcher {
    templates: DashMap<String, CompiledIntent>,
    threshold: f64,
}

impl FuzzyMatcher {
    pub fn new(threshold: f64) -> Self {
        Self {
            templates: DashMap::new(),
            threshold,
        }
    }

    /// Add a template for fuzzy matching
    pub fn add_template(&self, text: &str, intent: CompiledIntent) {
        self.templates.insert(Self::normalize(text), intent);
    }

    /// Find a fuzzy match above the threshold (0 tokens)
    pub fn find_match(&self, text: &str) -> Option<(CompiledIntent, f64)> {
        let normalized = Self::normalize(text);
        let input_words = Self::word_set(&normalized);

        let mut best: Option<(CompiledIntent, f64)> = None;

        for entry in self.templates.iter() {
            let template_words = Self::word_set(entry.key());
            let similarity = Self::jaccard_similarity(&input_words, &template_words);

            if similarity >= self.threshold
                && (best.is_none() || similarity > best.as_ref().unwrap().1)
            {
                let mut intent = entry.value().clone();
                intent.raw_text = text.to_string();
                intent.confidence = similarity;
                intent.tokens_used = 0; // Fuzzy match — zero tokens
                best = Some((intent, similarity));
            }
        }

        best
    }

    /// Jaccard similarity: |A ∩ B| / |A ∪ B|
    fn jaccard_similarity(
        a: &std::collections::HashSet<String>,
        b: &std::collections::HashSet<String>,
    ) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        let intersection = a.intersection(b).count();
        let union = a.union(b).count();
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    fn word_set(text: &str) -> std::collections::HashSet<String> {
        text.split_whitespace().map(|w| w.to_lowercase()).collect()
    }

    fn normalize(text: &str) -> String {
        text.trim()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn template_count(&self) -> usize {
        self.templates.len()
    }
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::new(0.85)
    }
}
