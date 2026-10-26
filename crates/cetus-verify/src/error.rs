//! Errors that prevent a verification report from being produced.

use thiserror::Error;

/// A failure before verification can report on the target's content.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// The target file could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// Path that could not be read.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// The requested verification level is not implemented by this build.
    #[error("unsupported verification level `{0}`; this build implements V0, V1 and V2")]
    UnsupportedLevel(String),
}
