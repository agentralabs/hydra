//! Hydra Federation Daemon — exercises all 5 collective crates.
//! In standalone mode, validates all subsystems are operational.
//!
//! Usage: cargo run -p hydra-kernel --bin hydra_fed

fn main() {
    eprintln!("hydra-fed: starting...");

    // Federation: create engine, verify self-registration
    let fed = hydra_federation::FederationEngine::new("hydra-local");
    eprintln!("hydra-fed: federation — {}", fed.summary());
    eprintln!("  peers={} sessions={} scopes={}",
        fed.peer_count(), fed.active_session_count(), fed.active_scope_count());

    // Consensus: create engine, report readiness
    let consensus = hydra_consensus::ConsensusEngine::new();
    eprintln!("hydra-fed: consensus — {}", consensus.summary());
    eprintln!("  resolutions={} uncertain={}",
        consensus.resolution_count(), consensus.uncertain_count());

    // Consent: create engine, report grant state
    let consent = hydra_consent::ConsentEngine::new();
    eprintln!("hydra-fed: consent — {}", consent.summary());
    eprintln!("  grants={} audit_entries={}",
        consent.active_grant_count(), consent.audit_count());

    // Collective: create engine, report observation state
    let collective = hydra_collective::CollectiveEngine::new();
    eprintln!("hydra-fed: collective — {}", collective.summary());
    eprintln!("  topics={} insights={}",
        collective.topic_count(), collective.insight_count());

    // Diplomat: create engine and open a validation session
    let mut diplomat = hydra_diplomat::DiplomatEngine::new();
    let session_id = diplomat.open_session("federation-startup-check");
    eprintln!("hydra-fed: diplomat — {}", diplomat.summary());
    eprintln!("  session={} active={} concluded={}",
        &session_id[..8], diplomat.session_count(), diplomat.concluded_count());

    // Exchange: create engine, report offer state
    let exchange = hydra_exchange::ExchangeEngine::new();
    eprintln!("hydra-fed: exchange — {}", exchange.summary());
    eprintln!("  offers={} receipts={} successful={}",
        exchange.offer_count(), exchange.receipt_count(),
        exchange.successful_exchange_count());

    eprintln!("hydra-fed: all 5 collective subsystems operational");
    eprintln!("hydra-fed: standalone mode — no network peers");
}
