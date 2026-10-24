//! Teaching layer: the `cetus.lesson/0.1` schema and model patches.
//!
//! Slice 1 starts with the strict lesson schema and model-path patches; static
//! checks are added in a follow-up commit. Observation generation, checkers,
//! mutation testing and the TypeScript runtime belong to later slices.

#![forbid(unsafe_code)]

mod error;
mod patch;
mod schema;

/// The lesson schema identifier this crate understands.
pub const SCHEMA_ID: &str = "cetus.lesson/0.1";

pub use error::LessonError;
pub use patch::{PathError, apply_patch, apply_truth_overrides, set_model_path};
pub use schema::*;
