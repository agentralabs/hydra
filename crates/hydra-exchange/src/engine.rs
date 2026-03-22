//! ExchangeEngine — the capability exchange coordinator.
//! THE FINAL LAYER 5 CRATE.

use crate::{
    constants::*,
    errors::ExchangeError,
    offer::{ExchangeOffer, OfferKind},
    receipt::{ExchangeOutcome, ExchangeReceipt},
    registry::ExchangeRegistry,
    request::ExchangeRequest,
};

/// Result of processing one exchange request.
#[derive(Debug)]
pub struct ExchangeResult {
    pub request_id: String,
    pub receipt_id: Option<String>,
    pub outcome: String,
    pub value: f64,
}

/// The exchange engine.
pub struct ExchangeEngine {
    pub registry: ExchangeRegistry,
}

impl ExchangeEngine {
    /// Create a new exchange engine.
    pub fn new() -> Self {
        Self {
            registry: ExchangeRegistry::new(),
        }
    }

    /// Register an offer — declare what Hydra provides.
    pub fn register_offer(&mut self, offer: ExchangeOffer) -> String {
        let id = offer.id.clone();
        self.registry.register_offer(offer);
        id
    }

    /// Process an incoming exchange request.
    /// Trust-gated. Wisdom-checked. Receipted.
    pub fn process_request(
        &mut self,
        mut request: ExchangeRequest,
    ) -> Result<ExchangeResult, ExchangeError> {
        // Step 1: Trust gate
        if !request.meets_trust_threshold(MIN_TRUST_FOR_EXCHANGE) {
            request.reject(format!(
                "Trust score {:.2} below minimum {:.2}",
                request.trust_score, MIN_TRUST_FOR_EXCHANGE
            ));
            return Err(ExchangeError::InsufficientTrust {
                counterparty: request.counterparty.clone(),
                score: request.trust_score,
                min: MIN_TRUST_FOR_EXCHANGE,
            });
        }

        // Step 2: Find matching offer
        let offer = self
            .registry
            .available_offers()
            .into_iter()
            .find(|o| {
                o.kind.label() == request.capability.label()
                    && o.min_trust_required <= request.trust_score
            })
            .ok_or_else(|| ExchangeError::CapabilityUnavailable {
                capability: request.capability.label(),
            })?;

        let offer_id = offer.id.clone();
        let offer_cost = offer.cost_per_exchange;
        let capability = offer.kind.label();

        // Step 3: Escalation check
        if offer_cost > MAX_UNESCALATED_EXCHANGE_VALUE {
            request.escalate(format!(
                "Exchange value {:.1} exceeds threshold {:.1} — requires principal approval",
                offer_cost, MAX_UNESCALATED_EXCHANGE_VALUE
            ));
            return Err(ExchangeError::EscalationRequired {
                value: offer_cost,
                max: MAX_UNESCALATED_EXCHANGE_VALUE,
            });
        }

        // Step 4: Approve and execute
        request.approve();

        // Step 5: Fulfill the offer (update its count)
        if let Some(o) = self.registry.get_offer_mut(&offer_id) {
            o.fulfill();
        }

        // Step 6: Issue receipt (write-ahead — constitutional)
        let receipt = ExchangeReceipt::new(
            &request.id,
            &offer_id,
            &request.counterparty,
            &capability,
            offer_cost,
            ExchangeOutcome::Fulfilled {
                description: format!(
                    "Exchange fulfilled: {} for {}",
                    capability, request.counterparty
                ),
            },
        );

        let receipt_id = receipt.id.clone();
        self.registry.record_receipt(receipt)?;

        // Mark request fulfilled
        let req_id = request.id.clone();
        request.fulfill(&receipt_id);

        Ok(ExchangeResult {
            request_id: req_id,
            receipt_id: Some(receipt_id),
            outcome: "fulfilled".into(),
            value: offer_cost,
        })
    }

    /// Make an outgoing exchange request to another system.
    pub fn request_capability(
        &self,
        counterparty: &str,
        capability: OfferKind,
        context: &str,
        trust_score: f64,
    ) -> ExchangeRequest {
        ExchangeRequest::new(counterparty, capability, context, trust_score)
    }

    /// Number of registered offers.
    pub fn offer_count(&self) -> usize {
        self.registry.offer_count()
    }

    /// Number of recorded receipts.
    pub fn receipt_count(&self) -> usize {
        self.registry.receipt_count()
    }

    /// Number of successful exchanges.
    pub fn successful_exchange_count(&self) -> usize {
        self.registry.successful_exchange_count()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "exchange: offers={} receipts={} successful={}",
            self.offer_count(),
            self.receipt_count(),
            self.successful_exchange_count(),
        )
    }
}

impl Default for ExchangeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> ExchangeEngine {
        let mut engine = ExchangeEngine::new();
        engine.register_offer(ExchangeOffer::new(
            OfferKind::SettlementExecution {
                skill_name: "agentra-settlement".into(),
            },
            "Execute settlements on behalf of authorized agents",
            0.70,
            15.0,
            None,
        ));
        engine.register_offer(ExchangeOffer::new(
            OfferKind::RedTeamAnalysis,
            "Pre-execution adversarial analysis",
            0.65,
            8.0,
            None,
        ));
        engine
    }

    #[test]
    fn request_fulfilled_above_trust_threshold() {
        let mut engine = setup();
        let request = ExchangeRequest::new(
            "agent-beta",
            OfferKind::RedTeamAnalysis,
            "analyze deployment plan for staging",
            0.75,
        );
        let result = engine.process_request(request).expect("should fulfill");
        assert_eq!(result.outcome, "fulfilled");
        assert!(result.receipt_id.is_some());
        assert_eq!(engine.receipt_count(), 1);
        assert_eq!(engine.successful_exchange_count(), 1);
    }

    #[test]
    fn request_rejected_below_trust() {
        let mut engine = setup();
        let request = ExchangeRequest::new(
            "unknown-agent",
            OfferKind::RedTeamAnalysis,
            "context",
            0.40, // below MIN_TRUST_FOR_EXCHANGE
        );
        let result = engine.process_request(request);
        assert!(result.is_err());
        assert!(matches!(
            result.expect_err("should err"),
            ExchangeError::InsufficientTrust { .. }
        ));
    }

    #[test]
    fn unavailable_capability_errors() {
        let mut engine = setup();
        let request = ExchangeRequest::new(
            "agent",
            OfferKind::GenomeSharing {
                domain: "cobol".into(),
                max_entries: 10,
            },
            "context",
            0.80,
        );
        let result = engine.process_request(request);
        assert!(matches!(
            result.expect_err("should err"),
            ExchangeError::CapabilityUnavailable { .. }
        ));
    }

    #[test]
    fn receipt_integrity_verified() {
        let mut engine = setup();
        let request =
            ExchangeRequest::new("trusted-agent", OfferKind::RedTeamAnalysis, "context", 0.85);
        engine.process_request(request).expect("should fulfill");
        let receipts = engine.registry.receipts_for_counterparty("trusted-agent");
        assert_eq!(receipts.len(), 1);
        assert!(receipts[0].verify_integrity());
    }

    #[test]
    fn summary_format() {
        let engine = ExchangeEngine::new();
        let s = engine.summary();
        assert!(s.contains("exchange:"));
        assert!(s.contains("offers="));
        assert!(s.contains("receipts="));
    }
}
