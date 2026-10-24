//! Teaching layer: the `cetus.lesson/0.1` schema.
//!
//! Slice 1 starts with the strict lesson schema; model patches and static
//! checks are added in follow-up commits. Observation generation, checkers,
//! mutation testing and the TypeScript runtime belong to later slices.

#![forbid(unsafe_code)]

mod error;
mod schema;

/// The lesson schema identifier this crate understands.
pub const SCHEMA_ID: &str = "cetus.lesson/0.1";

pub use error::LessonError;
pub use schema::*;
