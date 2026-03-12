use hydra_sisters::bridge::*;
use hydra_sisters::live_bridge::{BridgeConfig, LiveMcpBridge};

/// Helper: create a live HTTP bridge for a sister
pub(crate) fn live_bridge(sister_id: SisterId, port: u16) -> LiveMcpBridge {
    let caps = hydra_sisters::bridges::all_bridges()
        .into_iter()
        .find(|b| b.sister_id() == sister_id)
        .map(|b| b.capabilities())
        .unwrap_or_default();

    LiveMcpBridge::http(
        sister_id,
        format!("http://localhost:{}", port),
        caps,
        BridgeConfig::default(),
    )
}

/// Sister port mapping (convention for live testing)
pub(crate) fn port_for(id: SisterId) -> u16 {
    match id {
        SisterId::Memory => 3001,
        SisterId::Vision => 3002,
        SisterId::Codebase => 3003,
        SisterId::Identity => 3004,
        SisterId::Time => 3005,
        SisterId::Contract => 3006,
        SisterId::Comm => 3007,
        SisterId::Planning => 3008,
        SisterId::Cognition => 3009,
        SisterId::Reality => 3010,
        SisterId::Forge => 3011,
        SisterId::Aegis => 3012,
        SisterId::Veritas => 3013,
        SisterId::Evolve => 3014,
    }
}
