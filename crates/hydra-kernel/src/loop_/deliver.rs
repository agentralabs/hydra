//! deliver.rs — Audit receipt + settlement after every cycle.
//! Write-ahead: receipt written BEFORE returning response.

use hydra_audit::{AuditEngine, EventKind};
use hydra_calibration::{CalibrationEngine, EpistemicClass, JudgmentType};
use hydra_settlement::SettlementEngine;

use crate::loop_::types::CycleResult;

pub struct Deliverer {
    audit: AuditEngine,
    settlement: SettlementEngine,
    calibration: CalibrationEngine,
}

impl Deliverer {
    pub fn new() -> Self {
        Self {
            audit: AuditEngine::open(),
            settlement: SettlementEngine::open(),
            calibration: CalibrationEngine::open(),
        }
    }

    /// Write receipt and settle cost. Called after every cycle.
    pub fn deliver(&mut self, cycle: &CycleResult) {
        // 1. Write-ahead audit receipt
        let events = vec![
            (
                EventKind::TaskStarted {
                    intent: cycle.intent_summary.clone(),
                },
                "genesis",
                0u64,
            ),
            (
                EventKind::TaskCompleted {
                    duration_total_ms: cycle.duration_ms,
                },
                "loop-receipt",
                cycle.duration_ms,
            ),
        ];

        let receipt_summary = self.audit.audit_manual(
            &cycle.session_id,
            &format!("loop.{}", cycle.path),
            events,
        );

        if let Ok(summary) = &receipt_summary {
            tracing::debug!("audit: {}", summary);
        }

        // 2. Settle cost
        if let Err(e) = self.settlement.settle_skill_action(
            "hydra-loop",
            &format!("loop.{}", cycle.path),
            &cycle.domain,
            &cycle.intent_summary,
            cycle.tokens_used as u64,
            cycle.duration_ms,
            cycle.success,
        ) {
            tracing::debug!("settlement failed: {:?}", e);
        }

        // 3. Calibration label — honest confidence assessment in receipt
        let cal_label = if cycle.tokens_used == 0 { "zero-token".to_string() } else {
            let profile = self.calibration.epistemic_profile(&cycle.domain, &JudgmentType::SuccessProbability);
            match profile.epistemic_class {
                EpistemicClass::WellCalibrated => format!("cal:high({:.0}%)", profile.calibrated_confidence * 100.0),
                EpistemicClass::Uncertain => format!("cal:uncertain({}obs)", profile.observations),
                EpistemicClass::Uncalibrated => "cal:uncalibrated".into(),
                EpistemicClass::Irreducible => "cal:irreducible".into(),
            }
        };
        eprintln!("[hydra] stream complete: {}tok {}ms {cal_label}", cycle.tokens_used, cycle.duration_ms);
    }

    pub fn audit_count(&self) -> usize {
        self.audit.record_count()
    }
    pub fn settlement_count(&self) -> usize {
        self.settlement.record_count()
    }
    pub fn audit_summary(&self) -> String {
        self.audit.summary()
    }
}

impl Default for Deliverer {
    fn default() -> Self {
        Self::new()
    }
}
