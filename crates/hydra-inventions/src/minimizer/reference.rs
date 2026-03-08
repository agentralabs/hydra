//! ReferenceSubstitution — replace repeated content with short references.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Map of reference IDs to their full content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubstitutionMap {
    entries: HashMap<String, String>,
    next_id: usize,
}

impl SubstitutionMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a substitution and return the reference ID
    pub fn add(&mut self, content: String) -> String {
        // Check if content already has a reference
        for (id, existing) in &self.entries {
            if existing == &content {
                return id.clone();
            }
        }

        let id = format!("$ref_{}", self.next_id);
        self.next_id += 1;
        self.entries.insert(id.clone(), content);
        id
    }

    /// Resolve a reference ID to its content
    pub fn resolve(&self, id: &str) -> Option<&str> {
        self.entries.get(id).map(|s| s.as_str())
    }

    /// Number of stored references
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Expand all references in text
    pub fn expand(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (id, content) in &self.entries {
            result = result.replace(id, content);
        }
        result
    }
}

/// Reference substitution engine
pub struct ReferenceSubstitution {
    /// Minimum content length to be eligible for substitution
    min_length: usize,
    /// Minimum occurrences before substituting
    min_occurrences: usize,
}

impl ReferenceSubstitution {
    pub fn new(min_length: usize, min_occurrences: usize) -> Self {
        Self {
            min_length,
            min_occurrences,
        }
    }

    /// Substitute repeated content with references
    pub fn substitute(&self, content: &str) -> (String, SubstitutionMap) {
        let mut map = SubstitutionMap::new();

        // Find repeated segments (line-based)
        let lines: Vec<&str> = content.lines().collect();
        let mut occurrences: HashMap<String, Vec<usize>> = HashMap::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim().to_string();
            if trimmed.len() >= self.min_length {
                occurrences.entry(trimmed).or_default().push(i);
            }
        }

        // Build substitution map for lines that appear enough times
        let mut line_refs: HashMap<String, String> = HashMap::new();
        for (text, positions) in &occurrences {
            if positions.len() >= self.min_occurrences {
                let ref_id = map.add(text.clone());
                line_refs.insert(text.clone(), ref_id);
            }
        }

        // Apply substitutions (keep first occurrence, replace rest)
        let mut first_seen: HashMap<String, bool> = HashMap::new();
        let result_lines: Vec<String> = lines
            .iter()
            .map(|line| {
                let trimmed = line.trim().to_string();
                if let Some(ref_id) = line_refs.get(&trimmed) {
                    if first_seen.contains_key(&trimmed) {
                        return ref_id.clone();
                    }
                    first_seen.insert(trimmed, true);
                }
                line.to_string()
            })
            .collect();

        (result_lines.join("\n"), map)
    }

    /// Calculate token savings from substitution
    pub fn estimate_savings(&self, content: &str) -> usize {
        let (substituted, _) = self.substitute(content);
        let original_tokens = (content.len() + 3) / 4;
        let new_tokens = (substituted.len() + 3) / 4;
        original_tokens.saturating_sub(new_tokens)
    }
}

impl Default for ReferenceSubstitution {
    fn default() -> Self {
        Self::new(20, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_substitution() {
        let sub = ReferenceSubstitution::new(10, 2);
        let content = "this is a repeated line of text\nunique line\nthis is a repeated line of text\nanother unique";
        let (result, map) = sub.substitute(content);
        assert!(!map.is_empty());
        assert!(result.contains("$ref_"));
    }

    #[test]
    fn test_no_substitution_for_short() {
        let sub = ReferenceSubstitution::new(100, 2);
        let content = "short\nshort\nshort";
        let (_, map) = sub.substitute(content);
        assert!(map.is_empty());
    }

    #[test]
    fn test_expand_references() {
        let mut map = SubstitutionMap::new();
        let id = map.add("hello world".to_string());
        let text = format!("prefix {} suffix", id);
        let expanded = map.expand(&text);
        assert_eq!(expanded, "prefix hello world suffix");
    }
}
