//! Teaching layer: `cetus.lesson/0.1` schema, model patches and static checks.
//!
//! Slice 1 implements the lesson schema, model-path patch application and the
//! V1 static checks (`E2101`, `E5203`, `E5401`). Observation generation,
//! checkers, mutation testing and the TypeScript runtime belong to later
//! slices.

#![forbid(unsafe_code)]

mod error;
mod patch;
mod schema;
mod static_check;

/// The lesson schema identifier this crate understands.
pub const SCHEMA_ID: &str = "cetus.lesson/0.1";

pub use error::LessonError;
pub use patch::{PathError, apply_patch, apply_truth_overrides, set_model_path};
pub use schema::*;
pub use static_check::check_static;
