//! Binary format schema: header validation and version negotiation.

use crate::{
    constants::{ANIMUS_MAGIC, ANIMUS_VERSION, BINARY_HEADER_SIZE},
    errors::AnimusError,
};

/// The 12-byte binary header for Animus Prime messages.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimusHeader {
    /// Magic bytes.
    pub magic: [u8; 4],
    /// Format version.
    pub version: u32,
    /// Reserved flags.
    pub flags: u32,
}

impl AnimusHeader {
    /// Create the canonical header for the current version.
    pub fn current() -> Self {
        Self {
            magic: *ANIMUS_MAGIC,
            version: ANIMUS_VERSION,
            flags: 0,
        }
    }

    /// Serialize the header to exactly BINARY_HEADER_SIZE bytes.
    pub fn to_bytes(&self) -> [u8; BINARY_HEADER_SIZE] {
        let mut buf = [0u8; BINARY_HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.magic);
        buf[4..8].copy_from_slice(&self.version.to_be_bytes());
        buf[8..12].copy_from_slice(&self.flags.to_be_bytes());
        buf
    }

    /// Parse a header from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AnimusError> {
        if bytes.len() < BINARY_HEADER_SIZE {
            return Err(AnimusError::DeserializationFailed {
                offset: 0,
                reason: format!(
                    "header too short: {} bytes (need {})",
                    bytes.len(),
                    BINARY_HEADER_SIZE
                ),
            });
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);

        if &magic != ANIMUS_MAGIC {
            return Err(AnimusError::InvalidMagicHeader {
                expected: ANIMUS_MAGIC.to_vec(),
                got: magic.to_vec(),
            });
        }

        let version = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let flags = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        Ok(Self {
            magic,
            version,
            flags,
        })
    }

    /// Validate that the version is compatible with this runtime.
    pub fn validate_version(&self) -> Result<(), AnimusError> {
        if self.version != ANIMUS_VERSION {
            return Err(AnimusError::VersionMismatch {
                expected: ANIMUS_VERSION,
                got: self.version,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trips() {
        let h = AnimusHeader::current();
        let bytes = h.to_bytes();
        let parsed = AnimusHeader::from_bytes(&bytes).unwrap();
        assert_eq!(h, parsed);
    }

    #[test]
    fn header_magic_validated() {
        let mut bytes = AnimusHeader::current().to_bytes();
        bytes[0] = b'X'; // corrupt magic
        assert!(matches!(
            AnimusHeader::from_bytes(&bytes),
            Err(AnimusError::InvalidMagicHeader { .. })
        ));
    }

    #[test]
    fn header_too_short_rejected() {
        let bytes = [0u8; 4];
        assert!(AnimusHeader::from_bytes(&bytes).is_err());
    }

    #[test]
    fn version_mismatch_detected() {
        let mut h = AnimusHeader::current();
        h.version = 0x00_02_00_00; // future version
        assert!(h.validate_version().is_err());
    }

    #[test]
    fn correct_version_passes() {
        assert!(AnimusHeader::current().validate_version().is_ok());
    }
}
