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
/// Largest decompressed payload a snapshot may declare.
///
/// LZ4's size prefix is read before anything is validated and decides the
/// allocation, so a few bytes of garbage could otherwise ask the worker for
/// 4 GiB. Real snapshots are far smaller: the state vector plus probe buffers
/// (about 11 bytes per recorded point).
pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;

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
    /// The snapshot declares a payload larger than [`MAX_PAYLOAD_BYTES`].
    #[error("snapshot declares a {declared}-byte payload; the limit is {limit} bytes")]
    TooLarge {
        /// Size from the LZ4 prefix.
        declared: usize,
        /// [`MAX_PAYLOAD_BYTES`].
        limit: usize,
    },
    /// A valid snapshot of a different model.
    #[error("snapshot belongs to model `{snapshot_model}`, not to the loaded `{loaded_model}`")]
    WrongModel {
        /// Model id stored in the snapshot.
        snapshot_model: String,
        /// Model id of the plan being restored into.
        loaded_model: String,
    },
    /// A snapshot of this model taken before its content changed (edited or
    /// updated), so the state layout and equations may not match.
    #[error(
        "snapshot of `{model}` was taken from a different version of the model (it was edited or updated since)"
    )]
    ModelChanged {
        /// The model id (same in the snapshot and the plan).
        model: String,
    },
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
    let body = &bytes[8..];
    let Some(prefix) = body.first_chunk::<4>() else {
        return Err(SnapshotError::TooShort);
    };
    let declared = u32::from_le_bytes(*prefix) as usize;
    if declared > MAX_PAYLOAD_BYTES {
        return Err(SnapshotError::TooLarge {
            declared,
            limit: MAX_PAYLOAD_BYTES,
        });
    }
    let payload = lz4_flex::decompress_size_prepended(body)
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

    fn header() -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes
    }

    #[test]
    fn rejects_a_header_without_a_size_prefix() {
        let mut bytes = header();
        bytes.extend_from_slice(&[1, 2]);
        assert!(matches!(decode(&bytes), Err(SnapshotError::TooShort)));
    }

    #[test]
    fn refuses_to_allocate_an_oversized_payload() {
        // 12 bytes that would otherwise make LZ4 allocate 4 GiB.
        let mut bytes = header();
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            decode(&bytes),
            Err(SnapshotError::TooLarge { declared, limit })
                if declared == u32::MAX as usize && limit == MAX_PAYLOAD_BYTES
        ));
    }

    #[test]
    fn accepts_large_probe_buffers() {
        // A snapshot with a large probe buffer still decodes.
        let mut snapshot = sample();
        snapshot.probes = vec![("big".to_owned(), (0..200_000).collect(), vec![0.5; 200_000])];
        let decoded = decode(&encode(&snapshot).expect("encode")).expect("decode");
        assert_eq!(decoded.probes[0].1.len(), 200_000);
    }
}
