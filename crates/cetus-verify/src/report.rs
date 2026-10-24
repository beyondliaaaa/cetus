//! Verification reports and their Markdown/JSON renderings.

use std::fmt::Write as _;

use cetus_compile::{Diagnostics, Severity};

use crate::Level;

/// The outcome of one check at one level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// Level that produced this check.
    pub level: Level,
    /// Stable check name.
    pub name: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Short human-readable outcome.
    pub detail: String,
}

/// A complete verification report for one target.
#[derive(Debug, Clone)]
pub struct Report {
    /// Target path, as displayed to the user.
    pub target: String,
    /// Requested level.
    pub level: Level,
    /// Checks that ran, in order.
    pub checks: Vec<CheckResult>,
    /// Diagnostics produced by the checks.
    pub diagnostics: Diagnostics,
}

impl Report {
    /// Whether every check passed and no error-severity diagnostic exists.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|check| check.passed)
            && !self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error)
    }

    /// Renders the report as the stable JSON shape.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "target": self.target,
            "level": self.level.as_str(),
            "ok": self.passed(),
            "checks": self.checks.iter().map(|check| {
                serde_json::json!({
                    "level": check.level.as_str(),
                    "name": check.name,
                    "ok": check.passed,
                    "detail": check.detail,
                })
            }).collect::<Vec<_>>(),
            "diagnostics": self.diagnostics.iter().map(|diagnostic| {
                serde_json::json!({
                    "code": diagnostic.code,
                    "severity": severity_name(diagnostic.severity),
                    "path": diagnostic.path,
                    "message": diagnostic.message,
                    "span": diagnostic.span,
                    "help": diagnostic.help,
                })
            }).collect::<Vec<_>>(),
        })
    }

    /// Renders the report as Markdown.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("# Cetus verification report\n\n");
        let _ = writeln!(out, "- Target: `{}`", self.target);
        let _ = writeln!(out, "- Level: `{}`", self.level.as_str());
        let result = if self.passed() { "PASS" } else { "FAIL" };
        let _ = writeln!(out, "- Result: **{result}**\n");
        out.push_str("## Checks\n\n");
        out.push_str("| Level | Check | Result | Detail |\n|---|---|---|---|\n");
        for check in &self.checks {
            let status = if check.passed { "PASS" } else { "FAIL" };
            let _ = writeln!(
                out,
                "| {} | {} | {status} | {} |",
                check.level.as_str(),
                escape_cell(&check.name),
                escape_cell(&check.detail),
            );
        }
        out.push_str("\n## Diagnostics\n\n");
        if self.diagnostics.is_empty() {
            out.push_str("_No diagnostics._");
        } else {
            out.push_str(cetus_compile::diagnostics_to_markdown(&self.diagnostics).trim_end());
        }
        out
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn escape_cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}
