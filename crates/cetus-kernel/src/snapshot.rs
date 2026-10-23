//! Snapshots (§5.11, D12).
//!
//! Format: magic (6 bytes) + format version (u16 LE) + LZ4-compressed postcard
//! payload. The payload contains everything needed to resume: tick, all state
//! slots, random substream states, and the model identity.

use serde::{Deserialize, Serialize};

use crate::rng::RngStreams;

/// Magic bytes at the start of every snapshot.
pub const MAGIC: &[u8; 6] = b"CETUSS";
/// Current snapshot format version.
pub const FORMAT_VERSION: u16 = 1;

/// The serialisable snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Format version.
    pub format_version: u16,
    /// Kernel crate version.
    pub kernel_version: String,
    /// Model id (`energy.solar-battery`).
    pub model_id: String,
    /// Model semantic version.
    pub model_version: String,
    /// Content hash of the compiled plan.
    pub content_hash: [u8; 32],
    /// Current tick.
    pub tick: u64,
    /// All state slots.
    pub state: Vec<f64>,
    /// Random substreams.
    pub rng: RngStreams,
    /// Probe buffers: `(observable name, ticks, values)`.
    #[serde(default)]
    pub probes: Vec<(String, Vec<u64>, Vec<f64>)>,
}

/// Snapshot encoding errors.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    /// Serialisation failed.
    #[error("snapshot serialisation failed: {0}")]
    Encode(String),
    /// Deserialisation failed.
    #[error("snapshot decode failed: {0}")]
    Decode(String),
    /// Magic bytes or format version mismatch.
    #[error("not a Cetus snapshot (bad magic or format version)")]
    BadMagic,
    /// The snapshot truncates before the header.
    #[error("snapshot is too short")]
    TooShort,
}

/// Encodes a snapshot to bytes.
///
/// # Errors
/// Returns [`SnapshotError::Encode`] if postcard or LZ4 fails.
pub fn encode(snapshot: &Snapshot) -> Result<Vec<u8>, SnapshotError> {
    let payload = postcard::to_allocvec(snapshot)
        .map_err(|error| SnapshotError::Encode(error.to_string()))?;
    let compressed = lz4_flex::compress_prepend_size(&payload);
    let mut out = Vec::with_capacity(8 + compressed.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

/// Decodes a snapshot from bytes.
///
/// # Errors
/// Returns [`SnapshotError`] when the header, LZ4 frame or postcard payload is
/// invalid.
pub fn decode(bytes: &[u8]) -> Result<Snapshot, SnapshotError> {
    if bytes.len() < 8 {
        return Err(SnapshotError::TooShort);
    }
    if &bytes[..6] != MAGIC {
        return Err(SnapshotError::BadMagic);
    }
    let version = u16::from_le_bytes([bytes[6], bytes[7]]);
    if version != FORMAT_VERSION {
        return Err(SnapshotError::BadMagic);
    }
    let payload = lz4_flex::decompress_size_prepended(&bytes[8..])
        .map_err(|error| SnapshotError::Decode(error.to_string()))?;
    postcard::from_bytes(&payload).map_err(|error| SnapshotError::Decode(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Snapshot {
        Snapshot {
            format_version: FORMAT_VERSION,
            kernel_version: "0.0.0".to_owned(),
            model_id: "energy.solar-battery".to_owned(),
            model_version: "1.0.0".to_owned(),
            content_hash: [7; 32],
            tick: 42,
            state: vec![1.0, -0.0, f64::NAN, 3.5],
            rng: RngStreams::new(9),
            probes: vec![("charge".to_owned(), vec![0, 1], vec![1.0, 2.0])],
        }
    }

    #[test]
    fn roundtrips() {
        let snapshot = sample();
        let bytes = encode(&snapshot).expect("encode");
        let decoded = decode(&bytes).expect("decode");
        assert_eq!(decoded.tick, snapshot.tick);
        assert_eq!(decoded.state.len(), snapshot.state.len());
        assert_eq!(decoded.model_id, snapshot.model_id);
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(decode(b"nope"), Err(SnapshotError::TooShort)));
        assert!(matches!(
            decode(b"XXXXXX\x01\x00rest"),
            Err(SnapshotError::BadMagic)
        ));
    }
}
