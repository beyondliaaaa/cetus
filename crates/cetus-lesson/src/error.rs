//! Lesson loading errors.

use thiserror::Error;

/// An error while loading a `cetus.lesson/0.1` document.
#[derive(Debug, Error)]
pub enum LessonError {
    /// YAML syntax error (including an unknown or mistyped field).
    #[error("YAML error at {location}: {message}")]
    Yaml {
        /// Human-readable location (`line:column`) when available.
        location: String,
        /// Parser message.
        message: String,
    },
    /// The schema field is missing or unsupported.
    #[error("unsupported schema `{0}`; expected `cetus.lesson/0.1`")]
    UnsupportedSchema(String),
}

impl LessonError {
    /// The diagnostic code for lesson loading errors.
    #[must_use]
    pub fn code(&self) -> &'static str {
        "E0003"
    }
}
