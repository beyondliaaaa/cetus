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
        /// Hint-ladder i18n keys.
        #[serde(default)]
        hints: Vec<String>,
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
        /// Hint-ladder i18n keys.
        #[serde(default)]
        hints: Vec<String>,
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
        /// Hint-ladder i18n keys.
        #[serde(default)]
        hints: Vec<String>,
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
        /// Hint-ladder i18n keys.
        #[serde(default)]
        hints: Vec<String>,
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
        /// Hint-ladder i18n keys.
        #[serde(default)]
        hints: Vec<String>,
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
        /// Hint-ladder i18n keys.
        #[serde(default)]
        hints: Vec<String>,
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

    /// Whether this hole is an L1 causal arrow.
    #[must_use]
    pub fn is_causal_link(&self) -> bool {
        matches!(self, Self::CausalLink { .. })
    }
}
