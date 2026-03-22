//! Binary serialization and deserialization of Prime graphs and Signals.
//! Format: ANMA header (12 bytes) + JSON-encoded body (for v0.1.0).

use crate::{
    constants::BINARY_HEADER_SIZE, errors::AnimusError, graph::PrimeGraph,
    semiring::signal::Signal, serial::schema::AnimusHeader,
};

/// Serialize a PrimeGraph to Animus binary format.
pub fn serialize_graph(graph: &PrimeGraph) -> Result<Vec<u8>, AnimusError> {
    let header = AnimusHeader::current().to_bytes();
    let body = serde_json::to_vec(graph).map_err(|e| AnimusError::SerializationFailed {
        reason: e.to_string(),
    })?;

    let mut out = Vec::with_capacity(BINARY_HEADER_SIZE + body.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Deserialize a PrimeGraph from Animus binary format.
pub fn deserialize_graph(bytes: &[u8]) -> Result<PrimeGraph, AnimusError> {
    let header = AnimusHeader::from_bytes(bytes)?;
    header.validate_version()?;

    let body = &bytes[BINARY_HEADER_SIZE..];
    serde_json::from_slice(body).map_err(|e| AnimusError::DeserializationFailed {
        offset: BINARY_HEADER_SIZE,
        reason: e.to_string(),
    })
}

/// Serialize a Signal to Animus binary format.
pub fn serialize_signal(signal: &Signal) -> Result<Vec<u8>, AnimusError> {
    let header = AnimusHeader::current().to_bytes();
    let body = serde_json::to_vec(signal).map_err(|e| AnimusError::SerializationFailed {
        reason: e.to_string(),
    })?;

    let mut out = Vec::with_capacity(BINARY_HEADER_SIZE + body.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Deserialize a Signal from Animus binary format.
pub fn deserialize_signal(bytes: &[u8]) -> Result<Signal, AnimusError> {
    let header = AnimusHeader::from_bytes(bytes)?;
    header.validate_version()?;

    let body = &bytes[BINARY_HEADER_SIZE..];
    serde_json::from_slice(body).map_err(|e| AnimusError::DeserializationFailed {
        offset: BINARY_HEADER_SIZE,
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        graph::{Node, NodeType},
        semiring::signal::{SignalId, SignalTier, SignalWeight},
    };

    #[test]
    fn graph_round_trips() {
        let mut g = PrimeGraph::new();
        g.add_node(Node::new(NodeType::Intent, serde_json::json!("deploy")))
            .unwrap();

        let bytes = serialize_graph(&g).unwrap();
        let restored = deserialize_graph(&bytes).unwrap();
        assert_eq!(restored.node_count(), 1);
    }

    #[test]
    fn signal_round_trips() {
        let s = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Fleet,
            3,
        );
        let bytes = serialize_signal(&s).unwrap();
        let restored = deserialize_signal(&bytes).unwrap();
        assert_eq!(restored.id, s.id);
        assert_eq!(restored.tier, s.tier);
    }

    #[test]
    fn corrupted_magic_rejected() {
        let g = PrimeGraph::new();
        let bytes = serialize_graph(&g).unwrap();
        let mut corrupted = bytes;
        corrupted[0] = b'X';
        assert!(deserialize_graph(&corrupted).is_err());
    }

    #[test]
    fn empty_bytes_rejected() {
        assert!(deserialize_graph(&[]).is_err());
        assert!(deserialize_signal(&[]).is_err());
    }
}
