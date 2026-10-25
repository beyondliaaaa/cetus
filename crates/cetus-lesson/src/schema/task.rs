//! Tasks, their optional world setup and their checkers.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// What kind of work a task asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Observe the world through instruments.
    Measure,
    /// Build part of a model.
    Build,
    /// Estimate an unknown parameter.
    Estimate,
    /// Predict a future outcome.
    Predict,
    /// Fit a model to observations.
    Fit,
    /// Tinker with world controls.
    Tinker,
    /// Write a strategy or program.
    Code,
}

/// One task in a lesson.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    /// Stable identifier.
    pub id: String,
    /// Task kind.
    pub kind: TaskKind,
    /// Deterministic completion predicate.
    #[serde(default)]
    pub done_when: Option<String>,
    /// Single hole this task fills.
    #[serde(default)]
    pub hole: Option<String>,
    /// Multiple holes this task fills.
    #[serde(default)]
    pub holes: Vec<String>,
    /// Prompt i18n key.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Expected answer expression.
    #[serde(default)]
    pub answer: Option<String>,
    /// World setup for the attempt.
    #[serde(default)]
    pub setup: Option<Setup>,
    /// Controls the player may use.
    #[serde(default)]
    pub controls: Vec<String>,
    /// Checker applied to the attempt.
    #[serde(default)]
    pub check: Option<Check>,
}

/// Initial values and parameters for a task attempt.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    /// Parameter values keyed by parameter name.
    #[serde(default)]
    pub params: IndexMap<String, String>,
    /// Stock initial values keyed by stock name.
    #[serde(default)]
    pub initial: IndexMap<String, String>,
}

/// How a task attempt is judged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Check {
    /// The expression is numerically equivalent to the truth model.
    ExprEquiv {},
    /// The estimated parameter is within a relative tolerance.
    ParamWithin {
        /// Relative tolerance.
        rel_tol: f64,
    },
    /// A prediction is within an absolute tolerance.
    AbsTol {
        /// Tolerance expression (with units).
        tol: String,
    },
    /// A prediction is within a dimensionless relative tolerance.
    RelTol {
        /// Relative tolerance (0 < tol < 1).
        tol: f64,
    },
    /// The player's trajectory fits the observed one.
    TrajectoryFit {
        /// Observable compared against the observation.
        observable: String,
        /// Metric name (`nrmse`, ...).
        metric: String,
        /// Worst allowed metric value.
        max: f64,
        /// Comparison window as `[start, end]` time expressions.
        window: [String; 2],
    },
    /// The stock-flow or rule graph has the expected structure.
    Structure {},
    /// A code hole satisfies a named property.
    Property {
        /// Property specification identifier.
        spec: String,
    },
    /// The derivation chain is equivalent step by step.
    Derivation {},
    /// A structured explanation question.
    Explain {},
    /// An L0 world-state predicate.
    WorldState {
        /// Predicate expression.
        expr: String,
        /// Evaluation window as `[start, end]` time expressions.
        window: [String; 2],
    },
    /// The direction of an L1 causal arrow matches the truth model.
    CausalSign {},
    /// A discriminating experiment separates truth from a misconception.
    DiscriminatingExperiment {
        /// Required separation in units of instrument noise.
        min_separation: f64,
        /// Whether the learner must state a prediction first.
        require_prediction: bool,
    },
}
