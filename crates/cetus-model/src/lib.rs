//! Declarative model schema `cetus.model/0.1` (SD subset) and migrations.
//!
//! A model is data, never code (P1). The engine interprets it; LLM pipelines and
//! domain experts only ever write these files.
//!
//! Deserialisation is strict: unknown fields are rejected so that a typo in a
//! model file fails loudly (ADR-0002).

#![forbid(unsafe_code)]

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Localised text keyed by BCP-47 language tag (`zh-CN`, `en`).
pub type LocalizedText = IndexMap<String, String>;

/// Whether the model describes empirical science or a speculative construction
/// (P10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Empirical science: has a real-world reference.
    Empirical,
    /// Speculative laboratory: every conclusion depends on declared assumptions.
    Speculative,
}

/// The simulation paradigm a model uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParadigmId {
    /// System dynamics / ODE.
    Sd,
    /// Chemical reaction networks.
    Reactions,
    /// Discrete-time maps.
    DiscreteMap,
    /// Discrete-event simulation.
    Des,
    /// Agent-based models.
    Abm,
    /// Cellular automata.
    Ca,
    /// Network dynamics.
    Network,
    /// Structured-grid PDE.
    Pde,
    /// Rigid bodies.
    Rigid,
    /// Particles / molecular dynamics.
    Particles,
    /// Equilibrium and equation solving.
    Solve,
    /// Optimisation and fitting.
    Optim,
    /// Games.
    Game,
    /// Geometry and ray optics.
    Geometry,
    /// Algorithm traces.
    Algo,
    /// Digital logic.
    Logic,
    /// Automata and formal languages.
    Automata,
    /// Quantum circuits.
    QuantumCircuit,
    /// Derivation and symbolic work.
    Derivation,
    /// Statistics and data analysis.
    Data,
}

/// A front-end view kind a paradigm can render (K12).
///
/// The enum is the contract between the engine and the view registry in
/// `apps/web`: a paradigm declares what it can produce, and the front-end maps
/// each kind to a component. M2 only declares the kinds the shipped paradigms
/// actually use; new paradigms add declarations rather than editing the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    /// A time series chart.
    Series,
    /// A phase portrait / state-space plot.
    Phase,
    /// A scalar field on a 2D grid (heat map).
    Field2d,
    /// A vector field on a 2D grid.
    VectorField,
    /// A node-link network.
    Network,
    /// A particle cloud.
    Particles,
    /// A 2D scene with sprites and geometry.
    Scene2d,
    /// A tree or hierarchy.
    Tree,
    /// A matrix or table view.
    Matrix,
    /// A Bloch sphere for quantum states.
    BlochSphere,
    /// A quantum circuit diagram.
    Circuit,
    /// An automaton state diagram.
    Automaton,
    /// An execution trace / timeline.
    Trace,
}

impl ParadigmId {
    /// View kinds this paradigm can render.
    ///
    /// M2 only declares views for system dynamics and discrete maps; every
    /// other paradigm returns an empty slice rather than guessing (§5.13,
    /// ADR-0004 K12).
    #[must_use]
    pub fn views(self) -> &'static [ViewKind] {
        match self {
            Self::Sd | Self::DiscreteMap => &[ViewKind::Series, ViewKind::Phase],
            _ => &[],
        }
    }
}

/// Time model (K2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeKind {
    /// Physical time with units (default).
    #[default]
    Continuous,
    /// Iteration steps (maps, games, rounds).
    Discrete,
    /// No time: equilibrium, optimisation, geometry.
    None,
}

/// ODE integrator selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Integrator {
    /// Explicit Euler.
    Euler,
    /// Classical fourth-order Runge-Kutta (default).
    #[default]
    Rk4,
    /// Dormand-Prince 5(4) with adaptive sub-steps inside a macro step.
    Dopri5,
    /// Second-order Rosenbrock for stiff systems.
    Ros2,
}

/// `time:` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeSpec {
    /// Display/integration unit, e.g. `h`.
    pub unit: String,
    /// Macro step in `unit`.
    pub dt: f64,
    /// Simulated horizon in `unit` (used by `transient` when unset by the caller).
    pub horizon: Option<f64>,
    /// Integrator (default `rk4`).
    #[serde(default)]
    pub integrator: Integrator,
    /// Time kind (default `continuous`).
    #[serde(default)]
    pub kind: TimeKind,
    /// Optional anchor time in `unit`; defaults to 0.
    #[serde(default)]
    pub start: Option<f64>,
}

/// Parameter role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterRole {
    /// Fixed by the author; must cite a source (P6).
    #[default]
    Fixed,
    /// Player-adjustable within `range`.
    Control,
    /// Driven by a port / world allocation.
    Input,
}

/// A parameter declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterSpec {
    /// Unit expression.
    pub unit: String,
    /// Default value in `unit`.
    pub value: f64,
    /// Allowed range in `unit` (required for `control` in V1).
    pub range: Option<[f64; 2]>,
    /// Role (default `fixed`).
    #[serde(default)]
    pub role: ParameterRole,
    /// Source reference (`param:...`, `world:...`, `src:...`) — required for
    /// `fixed` parameters (P6).
    pub source: Option<String>,
    /// Typical magnitude in SI units (K13).
    pub scale: Option<f64>,
    /// Value type: `scalar` (default) or `complex` (K1). M1 type-checks complex
    /// values but rejects evaluating them with E1901.
    pub kind: Option<String>,
}

/// Stock bounds in `unit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundsSpec {
    /// Lower bound.
    pub min: Option<f64>,
    /// Upper bound; may reference a parameter or constant expression.
    pub max: Option<String>,
}

/// A stock declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StockSpec {
    /// Unit expression.
    pub unit: String,
    /// Initial value as an expression (may reference parameters).
    pub initial: String,
    /// Optional bounds.
    pub bounds: Option<BoundsSpec>,
    /// Typical magnitude in SI units (K13).
    pub scale: Option<f64>,
}

/// An auxiliary (algebraic) declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuxiliarySpec {
    /// Unit expression.
    pub unit: String,
    /// Defining equation.
    pub eq: String,
    /// Typical magnitude in SI units (K13).
    pub scale: Option<f64>,
}

/// A flow declaration: exactly one of `from` / `to` must be present.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowSpec {
    /// Stock this flow drains.
    pub from: Option<String>,
    /// Stock this flow fills.
    pub to: Option<String>,
    /// Unit expression (must be the stock unit per time unit).
    pub unit: String,
    /// Rate equation.
    pub eq: String,
}

/// A graphical function (piecewise linear lookup table).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableSpec {
    /// X axis.
    pub x: TableAxis,
    /// Y axis.
    pub y: TableAxis,
}

/// One axis of a [`TableSpec`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableAxis {
    /// Unit expression.
    pub unit: String,
    /// Sample points, strictly increasing.
    pub points: Vec<f64>,
}

/// A quantity exposed to instruments and the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservableSpec {
    /// Unit expression.
    pub unit: String,
    /// Defining equation.
    pub eq: String,
    /// Display hint (`percent`, `sci`, ...).
    pub display: Option<String>,
}

/// Invariant kinds checked every macro step (§5.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvariantKind {
    /// Expression must hold.
    Assert,
    /// Value must stay within bounds.
    Bounds,
    /// Must be non-negative.
    Nonnegative,
    /// Monotonic in time.
    Monotonic,
    /// Sum of listed quantities is conserved.
    ConservedSum,
}

/// A runtime invariant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvariantSpec {
    /// Stable identifier.
    pub id: String,
    /// Kind.
    pub kind: InvariantKind,
    /// Expression (for `assert` / `bounds`).
    #[serde(default)]
    pub expr: Option<String>,
}

/// Authoring metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetaSpec {
    /// Authors.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Source references from the source library.
    #[serde(default)]
    pub sources: Vec<String>,
    /// Explicit modelling assumptions.
    #[serde(default)]
    pub assumptions: Vec<String>,
}

/// A complete `cetus.model/0.1` document (SD subset).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// Schema identifier, must be `cetus.model/0.1`.
    pub schema: String,
    /// Stable module id, e.g. `energy.solar-battery`.
    pub id: String,
    /// Semantic version of this module.
    pub version: String,
    /// Human title.
    pub title: LocalizedText,
    /// Empirical or speculative.
    pub tier: Tier,
    /// Paradigms used by the model.
    pub paradigms: Vec<ParadigmId>,
    /// Time block.
    pub time: TimeSpec,
    /// Parameters.
    #[serde(default)]
    pub parameters: IndexMap<String, ParameterSpec>,
    /// Stocks.
    #[serde(default)]
    pub stocks: IndexMap<String, StockSpec>,
    /// Auxiliaries.
    #[serde(default)]
    pub auxiliaries: IndexMap<String, AuxiliarySpec>,
    /// Flows.
    #[serde(default)]
    pub flows: IndexMap<String, FlowSpec>,
    /// Lookup tables.
    #[serde(default)]
    pub tables: IndexMap<String, TableSpec>,
    /// Observables.
    #[serde(default)]
    pub observables: IndexMap<String, ObservableSpec>,
    /// Invariants.
    #[serde(default)]
    pub invariants: Vec<InvariantSpec>,
    /// Authoring metadata.
    #[serde(default)]
    pub meta: MetaSpec,
}

/// The schema identifier this crate understands.
pub const SCHEMA_ID: &str = "cetus.model/0.1";

/// A model loading error with an optional source location.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    /// YAML syntax error.
    #[error("YAML error at {location}: {message}")]
    Yaml {
        /// Human-readable location (`line:column`) when available.
        location: String,
        /// Parser message.
        message: String,
    },
    /// The schema field is missing or unsupported.
    #[error("unsupported schema `{0}`; expected `cetus.model/0.1`")]
    UnsupportedSchema(String),
}

impl Model {
    /// Parses a model from YAML.
    ///
    /// # Errors
    /// Returns [`ModelError`] on syntax errors or an unsupported schema id.
    pub fn from_yaml(source: &str) -> Result<Self, ModelError> {
        let model: Self = serde_yaml_ng::from_str(source).map_err(|error| {
            let location = error.location().map_or_else(
                || "unknown".to_owned(),
                |loc| format!("{}:{}", loc.line(), loc.column()),
            );
            ModelError::Yaml {
                location,
                message: error.to_string(),
            }
        })?;
        if model.schema != SCHEMA_ID {
            return Err(ModelError::UnsupportedSchema(model.schema));
        }
        Ok(model)
    }

    /// Serialises the model back to YAML (used by tests and the author preview).
    ///
    /// # Errors
    /// Returns a YAML error.
    pub fn to_yaml(&self) -> Result<String, ModelError> {
        serde_yaml_ng::to_string(self).map_err(|error| ModelError::Yaml {
            location: "unknown".to_owned(),
            message: error.to_string(),
        })
    }

    /// Resolves the title for a locale, falling back to `en` then the first entry.
    #[must_use]
    pub fn title_for(&self, locale: &str) -> &str {
        self.title
            .get(locale)
            .or_else(|| self.title.get("en"))
            .or_else(|| self.title.values().next())
            .map_or("", String::as_str)
    }

    /// The `id@version` string used in saves.
    #[must_use]
    pub fn id_version(&self) -> String {
        format!("{}@{}", self.id, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOLAR: &str = r#"
schema: cetus.model/0.1
id: energy.solar-battery
version: 1.0.0
title: { zh-CN: "太阳能板与蓄电池", en: "Solar panel and battery" }
tier: empirical
paradigms: [sd]
time:
  unit: h
  dt: 0.05
  horizon: 72
  integrator: rk4
parameters:
  area: { unit: "m^2", value: 12, range: [1, 100], role: control }
  efficiency: { unit: "1", value: 0.20, range: [0.05, 0.47], source: "param:pv.module-efficiency" }
  peak_irradiance: { unit: "W/m^2", value: 1000, range: [0, 1400], source: "param:solar.peak-irradiance.earthlike" }
  day_length: { unit: h, value: 26, source: "world:planet.day_length" }
  capacity: { unit: "kW*h", value: 20, range: [1, 200], role: control }
  load: { unit: W, value: 800, range: [0, 5000], role: input }
stocks:
  charge:
    unit: "kW*h"
    initial: "0.5 * capacity"
    bounds: { min: 0, max: capacity }
auxiliaries:
  irradiance: { unit: "W/m^2", eq: "peak_irradiance * max(0, sin(2*pi*time/day_length))" }
  pv_power: { unit: W, eq: "efficiency * area * irradiance" }
flows:
  charging: { to: charge, unit: W, eq: "if charge < capacity then pv_power else 0" }
  discharging: { from: charge, unit: W, eq: "if charge > 0 then load else 0" }
observables:
  charge_pct: { unit: "1", eq: "charge / capacity", display: percent }
invariants:
  - { id: charge-bounds, kind: bounds, expr: "charge >= 0 and charge <= capacity" }
meta:
  authors: ["@handle"]
  sources: ["src:iea-pv-2024"]
  assumptions:
    - "辐照度用正弦半波近似日变化"
"#;

    #[test]
    fn parses_the_solar_battery_model() {
        let model = Model::from_yaml(SOLAR).expect("parse");
        assert_eq!(model.id, "energy.solar-battery");
        assert_eq!(model.parameters.len(), 6);
        assert_eq!(model.stocks.len(), 1);
        assert_eq!(model.flows.len(), 2);
        assert_eq!(model.parameters["efficiency"].role, ParameterRole::Fixed);
        assert_eq!(model.parameters["area"].role, ParameterRole::Control);
    }

    #[test]
    fn rejects_unknown_fields() {
        let broken = SOLAR.replace("tier: empirical", "tier_typo: empirical");
        let error = Model::from_yaml(&broken).expect_err("must reject unknown field");
        assert!(matches!(error, ModelError::Yaml { .. }));
    }

    #[test]
    fn rejects_unknown_schema() {
        let broken = SOLAR.replace("cetus.model/0.1", "cetus.model/9.9");
        let error = Model::from_yaml(&broken).expect_err("must reject schema");
        assert!(matches!(error, ModelError::UnsupportedSchema(_)));
    }

    #[test]
    fn roundtrips_through_yaml() {
        let model = Model::from_yaml(SOLAR).expect("parse");
        let yaml = model.to_yaml().expect("serialise");
        let again = Model::from_yaml(&yaml).expect("reparse");
        assert_eq!(model.id, again.id);
        assert_eq!(model.flows.len(), again.flows.len());
    }
}
