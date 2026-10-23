//! Simulation kernel: scheduler, state storage, random streams, snapshots and
//! instruments.
//!
//! This crate is the bottom of the stack (P3): it must not depend on rendering,
//! UI, networking or teaching logic.
//!
//! M1 implements a single subsystem driven by one paradigm. The `Subsystem`
//! trait is the seam that later paradigms plug into (P13).

#![forbid(unsafe_code)]

pub mod dataset;
pub mod rng;
pub mod snapshot;

use std::sync::Arc;

use cetus_compile::{CompiledPlan, CompiledTable};
use cetus_expr::{CompiledExpr, EvalContext};
use cetus_math::canonical_bits;

pub use dataset::{Column, Dataset, DimAxis};
pub use rng::{RngStreams, derive_seed};
pub use snapshot::{Snapshot, SnapshotError};

/// A floating-point assertion failure or other runtime simulation error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SimError {
    /// A state slot became NaN or infinite (E4001).
    #[error("non-finite value in slot {slot} ({name})")]
    NonFinite {
        /// Slot index.
        slot: usize,
        /// Slot name.
        name: String,
    },
    /// An invariant failed.
    #[error("invariant `{id}` violated")]
    Invariant {
        /// Invariant id.
        id: String,
    },
    /// An observable or parameter name was not found.
    #[error("unknown name `{0}`")]
    UnknownName(String),
    /// The subsystem reported an error.
    #[error("subsystem error: {0}")]
    Subsystem(String),
}

/// Context for one macro step.
#[derive(Debug, Clone, Copy)]
pub struct StepContext {
    /// Tick being advanced to (the new tick after the step).
    pub tick: u64,
    /// Absolute time in seconds at the *start* of the step.
    pub t: f64,
    /// Step length in seconds.
    pub h: f64,
}

/// A stock bound was hit and the value clamped (§5.7.1).
#[derive(Debug, Clone, PartialEq)]
pub struct ClampEvent {
    /// Slot index.
    pub slot: usize,
    /// Slot name.
    pub name: String,
    /// Value before clamping.
    pub before: f64,
    /// Value after clamping.
    pub after: f64,
    /// Tick at which clamping happened.
    pub tick: u64,
}

/// An invariant failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation {
    /// Invariant id.
    pub id: String,
    /// Tick at which it failed.
    pub tick: u64,
}

/// Collects per-step diagnostics.
#[derive(Debug, Clone, Default)]
pub struct EventSink {
    /// Bound clamps.
    pub clamps: Vec<ClampEvent>,
    /// Invariant violations.
    pub violations: Vec<InvariantViolation>,
}

impl EventSink {
    /// Clears all events.
    pub fn clear(&mut self) {
        self.clamps.clear();
        self.violations.clear();
    }
}

/// A paradigm-provided subsystem.
pub trait Subsystem {
    /// Stable identifier (`sd`, `abm`, ...).
    fn id(&self) -> &'static str;

    /// Initialises derived state (auxiliaries) for the current stock/parameter
    /// values.
    ///
    /// # Errors
    /// Returns [`SimError`] when an expression cannot be evaluated.
    fn init(&mut self, state: &mut [f64], t: f64) -> Result<(), SimError>;

    /// Advances the subsystem by one macro step.
    ///
    /// Only stock slots may be written; parameters are read-only and
    /// auxiliaries are recomputed.
    ///
    /// # Errors
    /// Returns [`SimError`] on non-finite values or invariant failures.
    fn advance(
        &mut self,
        state: &mut [f64],
        ctx: &StepContext,
        events: &mut EventSink,
    ) -> Result<(), SimError>;
}

/// A recorded observable series.
#[derive(Debug, Clone)]
pub struct Probe {
    /// Observable name.
    pub name: String,
    /// Ticks recorded so far.
    pub ticks: Vec<u64>,
    /// Values in SI units.
    pub values: Vec<f64>,
}

/// What a single step produced.
#[derive(Debug, Clone, Default)]
pub struct StepReport {
    /// Tick after the step.
    pub tick: u64,
    /// Time in seconds after the step.
    pub time: f64,
    /// Bound clamps during the step.
    pub clamps: Vec<ClampEvent>,
    /// Invariant violations during the step.
    pub violations: Vec<InvariantViolation>,
}

/// A running simulation.
pub struct Sim {
    plan: Arc<CompiledPlan>,
    subsystem: Box<dyn Subsystem>,
    state: Vec<f64>,
    tick: u64,
    rng: RngStreams,
    events: EventSink,
    probes: Vec<Probe>,
}

impl std::fmt::Debug for Sim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sim")
            .field("model", &self.plan.model_id)
            .field("tick", &self.tick)
            .field("slots", &self.state.len())
            .finish_non_exhaustive()
    }
}

impl Sim {
    /// Creates a simulation at the plan's start tick.
    ///
    /// # Errors
    /// Returns [`SimError`] if initialisation produces a non-finite value.
    pub fn new(
        plan: Arc<CompiledPlan>,
        seed: u64,
        mut subsystem: Box<dyn Subsystem>,
    ) -> Result<Self, SimError> {
        let mut state = vec![0.0; plan.layout.len()];
        for stock in &plan.stocks {
            state[stock.slot] = stock.initial;
        }
        for parameter in &plan.parameters {
            state[parameter.slot] = parameter.value;
        }
        let tick = plan.time.start_tick;
        let t = plan.time.time_of(tick);
        subsystem.init(&mut state, t)?;
        let sim = Self {
            plan,
            subsystem,
            state,
            tick,
            rng: RngStreams::new(seed),
            events: EventSink::default(),
            probes: Vec::new(),
        };
        sim.check_finite()?;
        Ok(sim)
    }

    /// The compiled plan.
    #[must_use]
    pub fn plan(&self) -> &Arc<CompiledPlan> {
        &self.plan
    }

    /// Current tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Current time in seconds.
    #[must_use]
    pub fn time(&self) -> f64 {
        self.plan.time.time_of(self.tick)
    }

    /// Current time in the model's display unit.
    #[must_use]
    pub fn display_time(&self) -> f64 {
        self.plan.time.display(self.time())
    }

    /// The full state vector.
    #[must_use]
    pub fn state(&self) -> &[f64] {
        &self.state
    }

    /// Random substreams.
    pub fn rng(&mut self) -> &mut RngStreams {
        &mut self.rng
    }

    /// Sets a parameter by name, validating the declared range.
    ///
    /// # Errors
    /// Returns an error for unknown names, out-of-range values, or when
    /// re-initialising derived state fails.
    pub fn set_param(&mut self, name: &str, value: f64) -> Result<(), SimError> {
        let Some(parameter) = self.plan.parameters.iter().find(|p| p.name == name) else {
            return Err(SimError::UnknownName(name.to_owned()));
        };
        if let Some((min, max)) = parameter.range
            && !(value >= min && value <= max)
        {
            return Err(SimError::Subsystem(format!(
                "parameter `{name}` = {value} is outside [{min}, {max}]"
            )));
        }
        self.state[parameter.slot] = value;
        let t = self.time();
        self.subsystem.init(&mut self.state, t)?;
        Ok(())
    }

    /// Reads a parameter by name.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<f64> {
        self.plan
            .parameters
            .iter()
            .find(|p| p.name == name)
            .map(|p| self.state[p.slot])
    }

    /// Reads a stock by name.
    #[must_use]
    pub fn stock(&self, name: &str) -> Option<f64> {
        self.plan
            .stocks
            .iter()
            .find(|s| s.name == name)
            .map(|s| self.state[s.slot])
    }

    /// Reads an auxiliary by name.
    #[must_use]
    pub fn auxiliary(&self, name: &str) -> Option<f64> {
        self.plan
            .auxiliaries
            .iter()
            .find(|a| a.name == name)
            .map(|a| self.state[a.slot])
    }

    /// Evaluates an observable by name.
    ///
    /// # Errors
    /// Returns [`SimError::UnknownName`] if the name is not declared.
    pub fn observable(&self, name: &str) -> Result<f64, SimError> {
        let Some(observable) = self.plan.observables.iter().find(|o| o.name == name) else {
            return Err(SimError::UnknownName(name.to_owned()));
        };
        Ok(observable.expr.eval(&StateContext {
            state: &self.state,
            t: self.time(),
            dt: self.plan.time.quantum,
            tables: &self.plan.tables,
        }))
    }

    /// Registers a probe on an observable.
    ///
    /// # Errors
    /// Returns [`SimError::UnknownName`] if the name is not declared.
    pub fn add_probe(&mut self, name: &str) -> Result<(), SimError> {
        if self.plan.observable_index(name).is_none() {
            return Err(SimError::UnknownName(name.to_owned()));
        }
        if self.probes.iter().any(|probe| probe.name == name) {
            return Ok(());
        }
        self.probes.push(Probe {
            name: name.to_owned(),
            ticks: Vec::new(),
            values: Vec::new(),
        });
        Ok(())
    }

    /// All probes.
    #[must_use]
    pub fn probes(&self) -> &[Probe] {
        &self.probes
    }

    /// Advances one macro step.
    ///
    /// # Errors
    /// Returns [`SimError`] on non-finite state or invariant failure.
    pub fn step(&mut self) -> Result<StepReport, SimError> {
        self.events.clear();
        let ctx = StepContext {
            tick: self.tick + 1,
            t: self.time(),
            h: self.plan.time.quantum,
        };
        self.subsystem
            .advance(&mut self.state, &ctx, &mut self.events)?;
        self.tick += 1;
        self.check_finite()?;

        let mut probe_updates = Vec::with_capacity(self.probes.len());
        for probe in &self.probes {
            let value = self.observable(&probe.name)?;
            probe_updates.push(value);
        }
        for (probe, value) in self.probes.iter_mut().zip(probe_updates) {
            probe.ticks.push(self.tick);
            probe.values.push(value);
        }

        Ok(StepReport {
            tick: self.tick,
            time: self.time(),
            clamps: self.events.clamps.clone(),
            violations: self.events.violations.clone(),
        })
    }

    /// Advances until `tick` (inclusive) or until a step fails.
    ///
    /// # Errors
    /// Returns the first [`SimError`] encountered.
    pub fn run_until(&mut self, tick: u64) -> Result<StepReport, SimError> {
        let mut last = StepReport {
            tick: self.tick,
            time: self.time(),
            clamps: Vec::new(),
            violations: Vec::new(),
        };
        while self.tick < tick {
            last = self.step()?;
        }
        Ok(last)
    }

    /// Advances for `seconds` worth of macro steps.
    ///
    /// # Errors
    /// Returns the first [`SimError`] encountered.
    pub fn advance_seconds(&mut self, seconds: f64) -> Result<StepReport, SimError> {
        let target = self.tick + self.plan.time.tick_of(seconds);
        self.run_until(target)
    }

    /// Canonical state hash (D9).
    #[must_use]
    pub fn state_hash(&self) -> [u8; 32] {
        let mut bytes = Vec::with_capacity(8 + self.state.len() * 8);
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        for value in &self.state {
            bytes.extend_from_slice(&canonical_bits(*value).to_le_bytes());
        }
        hash_bytes(&bytes)
    }

    /// Hex form of [`Sim::state_hash`].
    #[must_use]
    pub fn state_hash_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.state_hash() {
            use std::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    /// Creates a snapshot.
    ///
    /// # Errors
    /// Returns [`SnapshotError`] when encoding fails.
    pub fn snapshot(&self) -> Result<Vec<u8>, SnapshotError> {
        let payload = Snapshot {
            format_version: snapshot::FORMAT_VERSION,
            kernel_version: env!("CARGO_PKG_VERSION").to_owned(),
            model_id: self.plan.model_id.clone(),
            model_version: self.plan.model_version.clone(),
            content_hash: self.plan.content_hash,
            tick: self.tick,
            state: self.state.clone(),
            rng: self.rng.clone(),
            probes: self
                .probes
                .iter()
                .map(|probe| {
                    (
                        probe.name.clone(),
                        probe.ticks.clone(),
                        probe.values.clone(),
                    )
                })
                .collect(),
        };
        snapshot::encode(&payload)
    }

    /// Restores a simulation from a snapshot.
    ///
    /// # Errors
    /// Returns [`SnapshotError`] when the snapshot is corrupt or belongs to a
    /// different model.
    pub fn restore(
        plan: Arc<CompiledPlan>,
        mut subsystem: Box<dyn Subsystem>,
        bytes: &[u8],
    ) -> Result<Self, SnapshotError> {
        let snapshot = snapshot::decode(bytes)?;
        if snapshot.model_id != plan.model_id || snapshot.content_hash != plan.content_hash {
            return Err(SnapshotError::BadMagic);
        }
        if snapshot.state.len() != plan.layout.len() {
            return Err(SnapshotError::Decode(
                "state length does not match the plan".to_owned(),
            ));
        }
        let t = plan.time.time_of(snapshot.tick);
        let mut restored_state = snapshot.state.clone();
        subsystem
            .init(&mut restored_state, t)
            .map_err(|error| SnapshotError::Decode(error.to_string()))?;
        let probes = snapshot
            .probes
            .iter()
            .map(|(name, ticks, values)| Probe {
                name: name.clone(),
                ticks: ticks.clone(),
                values: values.clone(),
            })
            .collect();
        Ok(Self {
            plan,
            subsystem,
            state: snapshot.state,
            tick: snapshot.tick,
            rng: snapshot.rng,
            events: EventSink::default(),
            probes,
        })
    }

    fn check_finite(&self) -> Result<(), SimError> {
        for (slot, value) in self.state.iter().enumerate() {
            if !value.is_finite() {
                return Err(SimError::NonFinite {
                    slot,
                    name: self.plan.layout.slots[slot].name.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Evaluation context over a state slice.
#[derive(Debug)]
pub struct StateContext<'a> {
    /// Slots.
    pub state: &'a [f64],
    /// Absolute time in seconds.
    pub t: f64,
    /// Macro step in seconds.
    pub dt: f64,
    /// Compiled tables.
    pub tables: &'a [CompiledTable],
}

impl EvalContext for StateContext<'_> {
    fn var(&self, id: u32) -> f64 {
        self.state.get(id as usize).copied().unwrap_or(0.0)
    }

    fn time(&self) -> f64 {
        self.t
    }

    fn dt(&self) -> f64 {
        self.dt
    }

    fn state(&self, _id: u32) -> f64 {
        0.0
    }

    fn lookup(&self, table: u32, x: f64) -> f64 {
        self.tables
            .get(table as usize)
            .map_or(0.0, |table| table.lookup(x))
    }
}

/// Evaluates an expression against a state slice (used by checkers).
#[must_use]
pub fn eval_expr(expr: &CompiledExpr, state: &[f64], t: f64, tables: &[CompiledTable]) -> f64 {
    expr.eval(&StateContext {
        state,
        t,
        dt: 0.0,
        tables,
    })
}

/// Hashes canonical bytes with BLAKE3.
#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

/// The M0 cross-target determinism witness.
///
/// Concatenates the canonical bits of [`cetus_math::transcendental_battery`]
/// and hashes them. The native CLI (`cetus hello`) and the wasm export must
/// print the same hex string; CI enforces it (`scripts/check-cross-target.sh`).
#[must_use]
pub fn hello_digest() -> [u8; 32] {
    let battery = cetus_math::transcendental_battery();
    let mut bytes = Vec::with_capacity(battery.len() * 8);
    for value in battery {
        bytes.extend_from_slice(&canonical_bits(value).to_le_bytes());
    }
    hash_bytes(&bytes)
}

/// Hex rendering of [`hello_digest`].
#[must_use]
pub fn hello_digest_hex() -> String {
    let digest = hello_digest();
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_64_hex_chars() {
        assert_eq!(hello_digest_hex().len(), 64);
    }

    #[test]
    fn digest_is_stable() {
        assert_eq!(hello_digest(), hello_digest());
    }

    #[test]
    fn hash_depends_on_input() {
        assert_ne!(hash_bytes(b"a"), hash_bytes(b"b"));
    }
}
