//! Teaching tasks: teaching a subagent or convincing a native.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{Check, LayerId, Setup};

/// A teaching task, tagged by who is being taught.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "learner", rename_all = "snake_case", deny_unknown_fields)]
pub enum TeachingTask {
    /// Convince a native who holds a misconception.
    Native {
        /// Stable identifier.
        id: String,
        /// Minimum representation layer this task targets.
        layer: LayerId,
        /// Misconception the native holds.
        holds: String,
        /// The native's claim (i18n key).
        claim: String,
        /// Experiment settings the player may choose.
        ///
        /// Boxed so the two variants stay close in size.
        setup_space: Box<SetupSpace>,
        /// Checker applied to the experiment.
        check: Check,
        /// Dialog shown when the native is convinced.
        ///
        /// Review variants may omit it and reuse the primary task's dialog.
        #[serde(default)]
        on_success: Option<String>,
        /// Review variants of this task.
        #[serde(default)]
        variants: Vec<String>,
    },
    /// Hand a working model to a subagent.
    Subagent {
        /// Stable identifier.
        id: String,
        /// Representation layer of the handed-over model.
        layer: LayerId,
        /// Holes whose answers are handed to the subagent.
        gives: Vec<String>,
        /// Scenarios the subagent must handle.
        scenarios: Vec<Scenario>,
        /// Checker applied to the subagent's runs.
        check: Check,
        /// Operational events used for review.
        #[serde(default)]
        new_situations: Vec<String>,
    },
}

impl TeachingTask {
    /// The stable identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Native { id, .. } | Self::Subagent { id, .. } => id,
        }
    }

    /// The misconception a native holds, if this is a native task.
    #[must_use]
    pub fn holds(&self) -> Option<&str> {
        match self {
            Self::Native { holds, .. } => Some(holds),
            Self::Subagent { .. } => None,
        }
    }

    /// Review variants of a native task (empty for a subagent task).
    #[must_use]
    pub fn variants(&self) -> &[String] {
        match self {
            Self::Native { variants, .. } => variants,
            Self::Subagent { .. } => &[],
        }
    }

    /// Experiment settings of a native task.
    #[must_use]
    pub fn setup_space(&self) -> Option<&SetupSpace> {
        match self {
            Self::Native { setup_space, .. } => Some(setup_space.as_ref()),
            Self::Subagent { .. } => None,
        }
    }
}

/// Experiment settings the player may choose for a native task.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupSpace {
    /// Adjustable controls and their ranges.
    #[serde(default)]
    pub controls: IndexMap<String, ControlRange>,
    /// Adjustable initial values and their ranges.
    #[serde(default)]
    pub initial: IndexMap<String, InitialRange>,
    /// Instrument ids the player may use.
    #[serde(default)]
    pub instruments: Vec<String>,
    /// Maximum observation duration.
    #[serde(default)]
    pub duration: Option<DurationSpec>,
    /// Inherit a setup space from another teaching task.
    #[serde(default)]
    pub inherit: Option<String>,
}

/// A control range written with units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlRange {
    /// Inclusive `[low, high]` range.
    pub range: [String; 2],
}

/// An initial-value range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialRange {
    /// Inclusive `[low, high]` range.
    pub range: [f64; 2],
}

/// Maximum observation duration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurationSpec {
    /// Maximum duration expression.
    pub max: String,
}

/// One scenario in which a subagent runs the handed-over model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Stable identifier.
    pub id: String,
    /// Scenario setup.
    pub setup: Setup,
}
