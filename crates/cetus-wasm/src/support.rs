//! Shared wire-format helpers.

use cetus_compile::{Diagnostic, Severity};
use serde::Serialize;

#[derive(Serialize)]
struct DiagnosticJson<'a> {
    severity: &'a str,
    code: &'a str,
    message: &'a str,
    path: Option<&'a str>,
    start: Option<usize>,
    end: Option<usize>,
    help: Option<&'a str>,
}

#[derive(Serialize)]
struct DiagnosticsJson<'a> {
    diagnostics: Vec<DiagnosticJson<'a>>,
}

/// Serialises compiler diagnostics as the `{"diagnostics":[...]}` payload.
pub(crate) fn diagnostics_json(diagnostics: &[Diagnostic]) -> String {
    let items = diagnostics
        .iter()
        .map(|diagnostic| {
            let (start, end) = diagnostic
                .span
                .map_or((None, None), |(start, end)| (Some(start), Some(end)));
            DiagnosticJson {
                severity: match diagnostic.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                },
                code: &diagnostic.code,
                message: &diagnostic.message,
                path: diagnostic.path.as_deref(),
                start,
                end,
                help: diagnostic.help.as_deref(),
            }
        })
        .collect();
    json(
        &DiagnosticsJson { diagnostics: items },
        "{\"diagnostics\":[]}",
    )
}

/// Lowercase hexadecimal rendering without an allocation per byte.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Serialises a value, falling back to a caller-supplied JSON document.
pub(crate) fn json<T: Serialize>(value: &T, fallback: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| fallback.to_owned())
}
