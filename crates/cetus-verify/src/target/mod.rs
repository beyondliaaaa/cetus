//! Target detection and per-target verification dispatch.

mod lesson;
mod module;

use std::path::Path;

use cetus_compile::Diagnostic;

use crate::report::{CheckResult, Report};
use crate::{Level, VerifyError};

/// Verifies `path` up to the requested cumulative `level`.
///
/// # Errors
/// Returns [`VerifyError::Io`] when the target cannot be read. Unsupported
/// levels are rejected by [`Level::parse`] before this function is reached.
pub fn verify_path(path: &Path, level: Level) -> Result<Report, VerifyError> {
    let source = std::fs::read_to_string(path).map_err(|source| VerifyError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let target = path.display().to_string();
    let report = match schema_of(&source) {
        Ok(schema) if schema == cetus_model::SCHEMA_ID => module::verify(&target, &source, level),
        Ok(schema) if schema == cetus_lesson::SCHEMA_ID => {
            lesson::verify(&target, path, &source, level)
        }
        Ok(schema) => unknown_schema(&target, level, &format!("unknown schema `{schema}`")),
        Err(message) => unknown_schema(&target, level, &message),
    };
    Ok(report)
}

fn schema_of(source: &str) -> Result<String, String> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(source).map_err(|error| error.to_string())?;
    value
        .get("schema")
        .and_then(serde_yaml_ng::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "missing `schema` field".to_owned())
}

fn unknown_schema(target: &str, level: Level, message: &str) -> Report {
    Report {
        target: target.to_owned(),
        level,
        checks: vec![CheckResult {
            level: Level::V0,
            name: "schema".to_owned(),
            passed: false,
            detail: message.to_owned(),
        }],
        diagnostics: vec![Diagnostic::error(
            "E0003",
            format!("cannot determine content schema: {message}"),
            None,
        )],
    }
}
