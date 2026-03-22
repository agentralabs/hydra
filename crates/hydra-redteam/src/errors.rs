use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum RedTeamError {
    #[error("Red team analysis failed: {reason}")]
    AnalysisFailed { reason: String },

    #[error("No attack surfaces identified for target '{target}'")]
    NoAttackSurfaces { target: String },
}
