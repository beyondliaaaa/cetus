//! Cumulative verification levels.

use crate::VerifyError;

/// A cumulative verification level: requesting [`Level::V1`] also runs V0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Schema validation.
    V0,
    /// Static checks on top of V0.
    V1,
    /// Numerical checks on top of V1.
    V2,
}

impl Level {
    /// Parses `V0`, `V1` or `V2`, case-insensitively.
    ///
    /// # Errors
    /// Returns [`VerifyError::UnsupportedLevel`] for every other input.
    pub fn parse(input: &str) -> Result<Self, VerifyError> {
        if input.eq_ignore_ascii_case("V0") {
            Ok(Self::V0)
        } else if input.eq_ignore_ascii_case("V1") {
            Ok(Self::V1)
        } else if input.eq_ignore_ascii_case("V2") {
            Ok(Self::V2)
        } else {
            Err(VerifyError::UnsupportedLevel(input.to_owned()))
        }
    }

    /// The canonical spelling of this level.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V0 => "V0",
            Self::V1 => "V1",
            Self::V2 => "V2",
        }
    }
}
