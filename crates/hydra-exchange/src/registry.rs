//! ExchangeRegistry — all offers and receipts.
//! Offers: queryable, mutable (can be withdrawn).
//! Receipts: append-only, immutable.

use crate::{
    constants::MAX_EXCHANGE_RECEIPTS, errors::ExchangeError, offer::ExchangeOffer,
    receipt::ExchangeReceipt,
};

/// The exchange registry.
#[derive(Debug, Default)]
pub struct ExchangeRegistry {
    offers: Vec<ExchangeOffer>,
    receipts: Vec<ExchangeReceipt>,
}

impl ExchangeRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    // --- OFFERS ---

    /// Register a new offer.
    pub fn register_offer(&mut self, offer: ExchangeOffer) {
        self.offers.push(offer);
    }

    /// Find an offer by id (immutable).
    pub fn get_offer(&self, offer_id: &str) -> Option<&ExchangeOffer> {
        self.offers.iter().find(|o| o.id == offer_id)
    }

    /// Find an offer by id (mutable).
    pub fn get_offer_mut(&mut self, offer_id: &str) -> Option<&mut ExchangeOffer> {
        self.offers.iter_mut().find(|o| o.id == offer_id)
    }

    /// Return all currently available offers.
    pub fn available_offers(&self) -> Vec<&ExchangeOffer> {
        self.offers.iter().filter(|o| o.is_available()).collect()
    }

    /// Total number of registered offers.
    pub fn offer_count(&self) -> usize {
        self.offers.len()
    }

    // --- RECEIPTS ---

    /// Append a receipt. Immutable after this.
    pub fn record_receipt(&mut self, receipt: ExchangeReceipt) -> Result<(), ExchangeError> {
        if self.receipts.len() >= MAX_EXCHANGE_RECEIPTS {
            return Err(ExchangeError::RegistryFull {
                max: MAX_EXCHANGE_RECEIPTS,
            });
        }
        self.receipts.push(receipt);
        Ok(())
    }

    /// Total number of receipts.
    pub fn receipt_count(&self) -> usize {
        self.receipts.len()
    }

    /// Find all receipts for a given counterparty.
    pub fn receipts_for_counterparty(&self, cp: &str) -> Vec<&ExchangeReceipt> {
        self.receipts
            .iter()
            .filter(|r| r.counterparty == cp)
            .collect()
    }

    /// Count of receipts with a successful outcome.
    pub fn successful_exchange_count(&self) -> usize {
        self.receipts
            .iter()
            .filter(|r| r.outcome.is_successful())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offer::OfferKind;
    use crate::receipt::ExchangeOutcome;

    #[test]
    fn register_and_query_offers() {
        let mut reg = ExchangeRegistry::new();
        reg.register_offer(ExchangeOffer::new(
            OfferKind::RedTeamAnalysis,
            "RT",
            0.65,
            10.0,
            None,
        ));
        assert_eq!(reg.offer_count(), 1);
        assert_eq!(reg.available_offers().len(), 1);
    }

    #[test]
    fn receipts_append_only() {
        let mut reg = ExchangeRegistry::new();
        let r = ExchangeReceipt::new(
            "req",
            "offer",
            "agent",
            "capability",
            10.0,
            ExchangeOutcome::Fulfilled {
                description: "done".into(),
            },
        );
        reg.record_receipt(r).expect("should succeed");
        assert_eq!(reg.receipt_count(), 1);
    }
}
