//! The `cetus.lesson/0.1` schema.
//!
//! Every struct rejects unknown fields so that a typo in a lesson file fails
//! loudly instead of being silently ignored.

mod hole;
mod layers;
mod task;
mod teaching;

pub use hole::Hole;
pub use layers::{Layer, LayerBase, LayerId, Layers};
pub use task::{Check, Setup, Task, TaskKind};
pub use teaching::{ControlRange, DurationSpec, InitialRange, Scenario, SetupSpace, TeachingTask};

use cetus_model::LocalizedText;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::SCHEMA_ID;
use crate::error::LessonError;

/// A complete lesson document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    /// Schema identifier, must be `cetus.lesson/0.1`.
    pub schema: String,
    /// Stable lesson id.
    pub id: String,
    /// Semantic version of this lesson.
    pub version: String,
    /// Knowledge-network node this lesson teaches.
    pub node: String,
    /// Module reference (`energy.solar-battery@^1`).
    pub module: String,
    /// Learning objectives.
    pub objectives: Vec<Objective>,
    /// Truth model overrides and dataset source.
    pub truth: Truth,
    /// Instruments the player may observe with.
    #[serde(default)]
    pub instruments: Vec<Instrument>,
    /// The base player model.
    pub player_model: PlayerModel,
    /// Base tasks.
    #[serde(default)]
    pub tasks: Vec<Task>,
    /// Base misconceptions.
    #[serde(default)]
    pub misconceptions: Vec<Misconception>,
    /// Mastery settings.
    pub mastery: Mastery,
    /// Narrative i18n keys.
    pub narrative: Narrative,
    /// Optional per-layer additions.
    #[serde(default)]
    pub layers: Option<Layers>,
    /// Teaching tasks.
    #[serde(default)]
    pub teaching: Vec<TeachingTask>,
}

/// One learning objective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    /// Stable identifier.
    pub id: String,
    /// Objective text keyed by language tag.
    pub text: LocalizedText,
}

/// The truth side of a lesson.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Truth {
    /// Parameter overrides applied before anything else.
    #[serde(default)]
    pub overrides: Option<TruthOverrides>,
    /// Dataset reference instead of (or alongside) the model.
    #[serde(default)]
    pub dataset: Option<String>,
}

/// Truth parameter overrides.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TruthOverrides {
    /// Overridden parameter values keyed by parameter name.
    #[serde(default)]
    pub parameters: IndexMap<String, ParameterOverride>,
}

/// A replacement value for one parameter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterOverride {
    /// New value in the parameter's declared unit.
    pub value: f64,
}

/// An instrument the player may observe with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    /// Stable identifier.
    pub id: String,
    /// Quantity observed.
    pub measures: String,
    /// Sampling interval expression.
    pub every: String,
    /// Optional noise model.
    #[serde(default)]
    pub noise: Option<Noise>,
    /// Optional display resolution.
    #[serde(default)]
    pub resolution: Option<f64>,
}

/// Instrument noise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Noise {
    /// Noise kind.
    pub kind: NoiseKind,
    /// Relative standard deviation.
    pub sigma: f64,
}

/// Supported noise kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoiseKind {
    /// Gaussian noise proportional to the reading.
    GaussianRel,
}

/// The player's model: the truth model plus explicit holes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerModel {
    /// Base model the player starts from.
    pub base: PlayerBase,
    /// Hidden pieces.
    #[serde(default)]
    pub holes: Vec<Hole>,
}

/// The base of a player model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerBase {
    /// The lesson's truth model.
    Truth,
}

/// Where a misconception comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MisconceptionSource {
    /// Source category.
    pub kind: MisconceptionSourceKind,
    /// Source reference (`src:<id>`); required unless `kind` is `hypothesized`.
    #[serde(rename = "ref", default)]
    pub reference: Option<String>,
    /// Optional authoring note.
    #[serde(default)]
    pub note: Option<String>,
}

/// The category of a misconception source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MisconceptionSourceKind {
    /// Recorded in discipline-education research.
    ResearchDocumented,
    /// A wrong approach that recurs in exam items.
    ExamItem,
    /// A theory held historically.
    HistoricalTheory,
    /// A documented folk belief.
    FolkBelief,
    /// A plausible guess with no source yet.
    Hypothesized,
}

impl MisconceptionSource {
    /// Whether this source is still a hypothesis awaiting research.
    #[must_use]
    pub fn is_hypothesized(&self) -> bool {
        self.kind == MisconceptionSourceKind::Hypothesized
    }
}

/// A misconception model and the tasks it must fail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Misconception {
    /// Stable identifier.
    pub id: String,
    /// Human-readable note.
    pub note: String,
    /// Where this misconception comes from.
    pub source: MisconceptionSource,
    /// Model patches (or L1 causal patches for layer misconceptions).
    #[serde(default)]
    pub patch: IndexMap<String, String>,
    /// Task ids this misconception must fail.
    #[serde(default)]
    pub must_fail: Vec<String>,
}

/// Mastery settings for the lesson's node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mastery {
    /// Knowledge-network node.
    pub node: String,
    /// Review events that keep this node alive.
    #[serde(default)]
    pub maintenance_events: Vec<String>,
}

/// Narrative framing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Narrative {
    /// Intro dialog key.
    pub intro: String,
    /// Outro dialog key.
    pub outro: String,
}

/// A module reference split into id and requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleRequirement {
    /// Module id before the last `@`.
    pub id: String,
    /// Version requirement after the last `@`, when present.
    pub requirement: Option<String>,
}

impl Lesson {
    /// Parses a lesson from YAML.
    ///
    /// # Errors
    /// Returns [`LessonError`] on syntax errors, unknown fields or an
    /// unsupported schema id.
    pub fn from_yaml(source: &str) -> Result<Self, LessonError> {
        let lesson: Self = serde_yaml_ng::from_str(source).map_err(|error| {
            let location = error.location().map_or_else(
                || "unknown".to_owned(),
                |loc| format!("{}:{}", loc.line(), loc.column()),
            );
            LessonError::Yaml {
                location,
                message: error.to_string(),
            }
        })?;
        if lesson.schema != SCHEMA_ID {
            return Err(LessonError::UnsupportedSchema(lesson.schema));
        }
        Ok(lesson)
    }

    /// Serialises the lesson back to YAML.
    ///
    /// # Errors
    /// Returns a YAML error.
    pub fn to_yaml(&self) -> Result<String, LessonError> {
        serde_yaml_ng::to_string(self).map_err(|error| LessonError::Yaml {
            location: "unknown".to_owned(),
            message: error.to_string(),
        })
    }

    /// Splits the module reference on its last `@`.
    #[must_use]
    pub fn module_requirement(&self) -> ModuleRequirement {
        self.module.rsplit_once('@').map_or_else(
            || ModuleRequirement {
                id: self.module.clone(),
                requirement: None,
            },
            |(id, requirement)| ModuleRequirement {
                id: id.to_owned(),
                requirement: Some(requirement.to_owned()),
            },
        )
    }

    /// Base holes followed by layer holes in L0-L3 order.
    pub fn holes(&self) -> impl Iterator<Item = &Hole> {
        let base = self.player_model.holes.iter();
        let layers = self
            .layers
            .iter()
            .flat_map(Layers::iter)
            .flat_map(|layer| layer.holes.iter());
        base.chain(layers)
    }

    /// Base tasks followed by layer tasks in L0-L3 order.
    pub fn tasks(&self) -> impl Iterator<Item = &Task> {
        let base = self.tasks.iter();
        let layers = self
            .layers
            .iter()
            .flat_map(Layers::iter)
            .flat_map(|layer| layer.tasks.iter());
        base.chain(layers)
    }

    /// Base misconceptions followed by layer misconceptions in L0-L3 order.
    pub fn misconceptions(&self) -> impl Iterator<Item = &Misconception> {
        let base = self.misconceptions.iter();
        let layers = self
            .layers
            .iter()
            .flat_map(Layers::iter)
            .flat_map(|layer| layer.misconceptions.iter());
        base.chain(layers)
    }

    /// Finds a teaching task by id.
    #[must_use]
    pub fn teaching_task(&self, id: &str) -> Option<&TeachingTask> {
        self.teaching.iter().find(|task| task.id() == id)
    }
}
