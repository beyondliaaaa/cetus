//! V0–V5 automated verification framework.
//!
//! This build implements the cumulative V0 (schema), V1 (static) and V2
//! (numerical) checks for modules and lessons, with Markdown and JSON reports.
//! V3–V5 belong to later slices.

#![forbid(unsafe_code)]

mod error;
mod level;
mod numeric;
mod report;
mod target;

pub use error::VerifyError;
pub use level::Level;
pub use report::{CheckResult, Report};
pub use target::verify_path;

/// The verification levels this build implements.
pub const IMPLEMENTED_LEVELS: &str = "V0, V1, V2";
