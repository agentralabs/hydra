use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ContinuityError {
    #[error("No checkpoints in arc — cannot generate proof")]
    EmptyArc,

    #[error("Arc '{lineage_id}' not found")]
    ArcNotFound { lineage_id: String },

    #[error("Checkpoint at day {day} would break continuity (gap from day {last})")]
    ContinuityBreak { day: u32, last: u32 },
}
