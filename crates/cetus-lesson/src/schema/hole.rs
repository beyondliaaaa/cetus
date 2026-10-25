//! Holes in the player model: the pieces the learner has to reconstruct.

use serde::{Deserialize, Serialize};

/// One hole the player must fill.
///
/// Six variants are model-path holes: their `target` is a path into the truth
/// model. `causal_link` holes instead describe an arrow between two named
/// quantities and carry no model path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Hole {
    /// An equation the player must rebuild from palette blocks.
    Equation {
        /// Stable identifier.
        id: String,
        /// Model path the hole replaces.
        target: String,
        /// Blocks offered by the visual builder.
        #[serde(default)]
        palette: Vec<String>,
        /// Hint ladder; `q0` is optional, `h1`-`h3` are required.
        hints: HintLadder,
    },
    /// A parameter value the player must estimate or fit.
    Parameter {
        /// Stable identifier.
        id: String,
        /// Model path the hole replaces.
        target: String,
        /// Blocks offered by the visual builder.
        #[serde(default)]
        palette: Vec<String>,
        /// Hint ladder; `q0` is optional, `h1`-`h3` are required.
        hints: HintLadder,
    },
    /// A missing connection in a stock-flow graph.
    Link {
        /// Stable identifier.
        id: String,
        /// Model path the hole replaces.
        target: String,
        /// Blocks offered by the visual builder.
        #[serde(default)]
        palette: Vec<String>,
        /// Hint ladder; `q0` is optional, `h1`-`h3` are required.
        hints: HintLadder,
    },
    /// A missing rule.
    Rule {
        /// Stable identifier.
        id: String,
        /// Model path the hole replaces.
        target: String,
        /// Blocks offered by the visual builder.
        #[serde(default)]
        palette: Vec<String>,
        /// Hint ladder; `q0` is optional, `h1`-`h3` are required.
        hints: HintLadder,
    },
    /// A code hole (a property-checked implementation).
    Code {
        /// Stable identifier.
        id: String,
        /// Model path the hole replaces.
        target: String,
        /// Blocks offered by the visual builder.
        #[serde(default)]
        palette: Vec<String>,
        /// Hint ladder; `q0` is optional, `h1`-`h3` are required.
        hints: HintLadder,
    },
    /// A derivation step the player must justify.
    Derivation {
        /// Stable identifier.
        id: String,
        /// Model path the hole replaces.
        target: String,
        /// Blocks offered by the visual builder.
        #[serde(default)]
        palette: Vec<String>,
        /// Hint ladder; `q0` is optional, `h1`-`h3` are required.
        hints: HintLadder,
    },
    /// An L1 causal arrow: increasing `from` does what to `to`?
    CausalLink {
        /// Stable identifier.
        id: String,
        /// Cause quantity name.
        from: String,
        /// Effect quantity name.
        to: String,
    },
}

/// The four-rung hint ladder for a hole (§6.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HintLadder {
    /// Level-0 question; when absent the checker generates one deterministically.
    #[serde(default)]
    pub q0: Option<String>,
    /// Level-1 direction hint.
    pub h1: String,
    /// Level-2 key-relationship hint.
    pub h2: String,
    /// Level-3 nearly complete answer.
    pub h3: String,
}

impl Hole {
    /// The stable identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Equation { id, .. }
            | Self::Parameter { id, .. }
            | Self::Link { id, .. }
            | Self::Rule { id, .. }
            | Self::Code { id, .. }
            | Self::Derivation { id, .. }
            | Self::CausalLink { id, .. } => id,
        }
    }

    /// The model path this hole replaces, if it is a model-path hole.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::Equation { target, .. }
            | Self::Parameter { target, .. }
            | Self::Link { target, .. }
            | Self::Rule { target, .. }
            | Self::Code { target, .. }
            | Self::Derivation { target, .. } => Some(target),
            Self::CausalLink { .. } => None,
        }
    }

    /// The hint ladder, if this hole has one.
    #[must_use]
    pub fn hints(&self) -> Option<&HintLadder> {
        match self {
            Self::Equation { hints, .. }
            | Self::Parameter { hints, .. }
            | Self::Link { hints, .. }
            | Self::Rule { hints, .. }
            | Self::Code { hints, .. }
            | Self::Derivation { hints, .. } => Some(hints),
            Self::CausalLink { .. } => None,
        }
    }

    /// Whether this hole is an L1 causal arrow.
    #[must_use]
    pub fn is_causal_link(&self) -> bool {
        matches!(self, Self::CausalLink { .. })
    }
}
