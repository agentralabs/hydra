//! Social genome helpers — create genome entries for communication patterns.

use crate::entry::GenomeEntry;
use crate::signature::{ApproachSignature, SituationSignature};

/// Create a genome entry for a person's communication style.
pub fn create_communication_entry(person: &str, situation: &str, approach: &str, confidence: f64) -> GenomeEntry {
    let sit = SituationSignature::new(&format!("communication {person} {situation}"));
    let app = ApproachSignature::new("social", vec![approach.into()], vec!["communication".into()]);
    GenomeEntry::from_parts(sit, app, confidence)
}

/// Create a genome entry for an empathy pattern.
pub fn create_empathy_entry(situation: &str, approach: &str) -> GenomeEntry {
    let sit = SituationSignature::new(&format!("empathy {situation}"));
    let app = ApproachSignature::new("empathy", vec![approach.into()], vec!["social".into()]);
    GenomeEntry::from_parts(sit, app, 0.85)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn communication_entry_has_person_keywords() {
        let entry = create_communication_entry("john", "code review", "be direct", 0.9);
        assert!(entry.situation.keywords.contains("john") || entry.situation.keywords.iter().any(|k| k.contains("john")));
    }

    #[test]
    fn empathy_entry_has_social_tool() {
        let entry = create_empathy_entry("frustrated colleague", "acknowledge feeling first");
        assert_eq!(entry.approach.tools_used, vec!["social"]);
    }
}
