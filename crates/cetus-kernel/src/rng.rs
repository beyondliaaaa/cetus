//! Deterministic random substreams (§5.9, D7).
//!
//! Every entity gets a named substream derived from the global seed, so adding
//! an unrelated agent never perturbs another entity's sequence.

use indexmap::IndexMap;
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::{Deserialize, Serialize};

/// Derives a 32-byte seed for a substream.
///
/// `purpose` must be one of the registered names in §5.9 (`activation`, `rule`,
/// `init`, `observation_noise`, `breakthrough`, `story`, `consult_variant`, ...).
#[must_use]
pub fn derive_seed(global_seed: u64, subsystem: &str, purpose: &str, entity: u64) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cetus-rng-v1");
    hasher.update(&global_seed.to_le_bytes());
    hasher.update(subsystem.as_bytes());
    hasher.update(&[0]);
    hasher.update(purpose.as_bytes());
    hasher.update(&[0]);
    hasher.update(&entity.to_le_bytes());
    *hasher.finalize().as_bytes()
}

/// A set of named substreams.
///
/// Streams are stored in insertion order so that snapshots are deterministic
/// (D1, D12).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RngStreams {
    global_seed: u64,
    streams: IndexMap<String, ChaCha8Rng>,
}

impl RngStreams {
    /// Builds an empty set for a global seed.
    #[must_use]
    pub fn new(global_seed: u64) -> Self {
        Self {
            global_seed,
            streams: IndexMap::new(),
        }
    }

    /// The global seed.
    #[must_use]
    pub fn global_seed(&self) -> u64 {
        self.global_seed
    }

    /// Number of active streams.
    #[must_use]
    pub fn len(&self) -> usize {
        self.streams.len()
    }

    /// Whether no stream has been created.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.streams.is_empty()
    }

    /// Returns (creating if needed) the stream for `(subsystem, purpose, entity)`.
    pub fn stream(&mut self, subsystem: &str, purpose: &str, entity: u64) -> &mut ChaCha8Rng {
        let key = format!("{subsystem}\u{1f}{purpose}\u{1f}{entity}");
        if !self.streams.contains_key(&key) {
            let seed = derive_seed(self.global_seed, subsystem, purpose, entity);
            self.streams
                .insert(key.clone(), ChaCha8Rng::from_seed(seed));
        }
        self.streams
            .get_mut(&key)
            .expect("stream was just inserted")
    }

    /// Resets every stream to its initial state.
    pub fn reset(&mut self) {
        let global = self.global_seed;
        let keys: Vec<String> = self.streams.keys().cloned().collect();
        self.streams.clear();
        for key in keys {
            let mut parts = key.split('\u{1f}');
            let subsystem = parts.next().unwrap_or_default().to_owned();
            let purpose = parts.next().unwrap_or_default().to_owned();
            let entity = parts.next().and_then(|text| text.parse().ok()).unwrap_or(0);
            self.stream(subsystem.as_str(), purpose.as_str(), entity);
        }
        debug_assert_eq!(self.global_seed, global);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derivation_is_stable_and_distinct() {
        let a = derive_seed(1, "sd", "init", 0);
        let b = derive_seed(1, "sd", "init", 0);
        let c = derive_seed(1, "sd", "init", 1);
        let d = derive_seed(2, "sd", "init", 0);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn streams_are_independent() {
        let mut left = RngStreams::new(42);
        let mut right = RngStreams::new(42);
        let first: Vec<u64> = (0..4)
            .map(|_| rand::RngExt::random(left.stream("sd", "rule", 0)))
            .collect();
        let second: Vec<u64> = (0..4)
            .map(|_| rand::RngExt::random(right.stream("sd", "rule", 0)))
            .collect();
        assert_eq!(first, second);
    }

    #[test]
    fn new_entity_does_not_perturb_existing_stream() {
        let mut left = RngStreams::new(7);
        let mut right = RngStreams::new(7);
        let baseline: Vec<u64> = (0..3)
            .map(|_| rand::RngExt::random(left.stream("abm", "rule", 1)))
            .collect();
        let _ = rand::RngExt::random::<u64>(right.stream("abm", "rule", 2));
        let after: Vec<u64> = (0..3)
            .map(|_| rand::RngExt::random(right.stream("abm", "rule", 1)))
            .collect();
        assert_eq!(baseline, after);
    }
}
