//! Representation layers (L0-L3) of a lesson.

use serde::{Deserialize, Serialize};

use super::{Hole, Misconception, Task};

/// Which representation layer a block belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerId {
    /// Direct manipulation of world objects.
    #[serde(rename = "L0")]
    L0,
    /// Causal arrows.
    #[serde(rename = "L1")]
    L1,
    /// Stocks, flows, parameters and equations.
    #[serde(rename = "L2")]
    L2,
    /// Code and algorithms.
    #[serde(rename = "L3")]
    L3,
}

impl LayerId {
    /// The canonical `L0`-`L3` spelling.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::L0 => "L0",
            Self::L1 => "L1",
            Self::L2 => "L2",
            Self::L3 => "L3",
        }
    }
}

/// The base layer may be inherited wholesale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerBase {
    /// The lesson's base `player_model`, `tasks` and `misconceptions`.
    Base,
}

/// Per-layer additions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    /// Inherit the base layer.
    #[serde(default)]
    pub inherit: Option<LayerBase>,
    /// Tasks added at this layer.
    #[serde(default)]
    pub tasks: Vec<Task>,
    /// Holes added at this layer.
    #[serde(default)]
    pub holes: Vec<Hole>,
    /// Misconceptions added at this layer.
    #[serde(default)]
    pub misconceptions: Vec<Misconception>,
}

/// The four optional representation layers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layers {
    /// Layer 0: direct manipulation.
    #[serde(rename = "L0", default)]
    pub l0: Option<Layer>,
    /// Layer 1: causal links.
    #[serde(rename = "L1", default)]
    pub l1: Option<Layer>,
    /// Layer 2: equations and parameters.
    #[serde(rename = "L2", default)]
    pub l2: Option<Layer>,
    /// Layer 3: code.
    #[serde(rename = "L3", default)]
    pub l3: Option<Layer>,
}

impl Layers {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Layer> {
        [
            self.l0.as_ref(),
            self.l1.as_ref(),
            self.l2.as_ref(),
            self.l3.as_ref(),
        ]
        .into_iter()
        .flatten()
    }
}
