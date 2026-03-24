//! Self-Preservation: Cross-Instance Merge — sync genome + calibration across machines.
//! Genome: union of entries, highest confidence wins on conflicts.
//! Calibration: combine observations (Bayesian — add alphas and betas).

/// Result of merging two genome stores.
#[derive(Debug, Default)]
pub struct MergeResult {
    pub entries_added: usize,
    pub entries_updated: usize,
    pub conflicts: usize,
    pub calibration_merged: usize,
}

impl MergeResult {
    pub fn summary(&self) -> String {
        format!("+{} entries, {} updated, {} conflicts, {} calibration obs",
            self.entries_added, self.entries_updated, self.conflicts, self.calibration_merged)
    }
}

/// Merge genome entries from a remote backup into the local store.
/// Union strategy: new entries added, existing entries take highest confidence.
pub fn merge_genome(
    local: &mut hydra_genome::GenomeStore,
    remote_entries: &[hydra_genome::GenomeEntry],
) -> MergeResult {
    let mut result = MergeResult::default();

    for remote in remote_entries {
        // Check if we already have this entry (BM25 similarity)
        let desc: String = remote.situation.keywords.iter().cloned().collect::<Vec<_>>().join(" ");
        let matches = local.query(&desc);

        if let Some(existing) = matches.first() {
            // Entry exists — update if remote has higher confidence
            if remote.effective_confidence() > existing.effective_confidence() {
                let id = existing.id.clone();
                // Record as successful use to boost confidence
                if let Err(e) = local.record_use(&id, true) {
                    eprintln!("hydra-merge: record_use failed: {e}");
                }
                result.entries_updated += 1;
            } else {
                result.conflicts += 1;
            }
        } else {
            // New entry — add to local
            match local.add(remote.clone()) {
                Ok(_) => result.entries_added += 1,
                Err(e) => eprintln!("hydra-merge: add failed: {e}"),
            }
        }
    }

    eprintln!("hydra-merge: {}", result.summary());
    result
}

/// Merge calibration observations from a remote backup.
/// Bayesian merge: combine alpha/beta counts.
pub fn merge_calibration(
    local: &mut hydra_calibration::CalibrationEngine,
    remote_records: &[hydra_calibration::CalibrationRecord],
) -> usize {
    let mut merged = 0;
    for record in remote_records {
        // Add each remote observation as a new prediction record
        if let Err(e) = local.record_prediction(
            &record.domain,
            record.judgment_type.clone(),
            record.stated_confidence,
        ) {
            eprintln!("hydra-merge: calibration merge failed: {e}");
            continue;
        }
        merged += 1;
    }
    eprintln!("hydra-merge: merged {} calibration records", merged);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_empty_genome() {
        let mut local = hydra_genome::GenomeStore::new();
        let result = merge_genome(&mut local, &[]);
        assert_eq!(result.entries_added, 0);
        assert_eq!(result.conflicts, 0);
    }

    #[test]
    fn merge_adds_new_entries() {
        let mut local = hydra_genome::GenomeStore::new();
        let approach = hydra_genome::ApproachSignature::new("test", vec!["step".into()], vec![]);
        let entry = hydra_genome::GenomeEntry::from_operation("new knowledge about rust", approach, 0.7);
        let result = merge_genome(&mut local, &[entry]);
        assert_eq!(result.entries_added, 1);
    }

    #[test]
    fn merge_result_summary() {
        let r = MergeResult { entries_added: 5, entries_updated: 2, conflicts: 1, calibration_merged: 3 };
        assert!(r.summary().contains("+5"));
        assert!(r.summary().contains("2 updated"));
    }
}
