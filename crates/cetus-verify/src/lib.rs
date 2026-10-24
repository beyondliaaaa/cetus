//! V0–V5 automated verification framework.
//!
//! Slice 1 implements the cumulative V0 (schema) and V1 (static) checks for
//! modules and lessons, with Markdown and JSON reports. V2–V5 belong to later
//! slices.

#![forbid(unsafe_code)]

mod error;
mod level;
mod report;
mod target;

pub use error::VerifyError;
pub use level::Level;
pub use report::{CheckResult, Report};
pub use target::verify_path;

/// The verification levels this build implements.
pub const IMPLEMENTED_LEVELS: &str = "V0, V1";
