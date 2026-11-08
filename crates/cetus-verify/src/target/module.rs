//! Module target verification: V0 schema, V1 compilation and V2 numerics.

use std::sync::Arc;

use cetus_compile::{Diagnostic, Severity, compile_model_with_warnings};
use cetus_model::Model;

use crate::Level;
use crate::numeric;
use crate::report::{CheckResult, Report};

pub(super) fn verify(target: &str, source: &str, level: Level) -> Report {
    let mut report = Report {
        target: target.to_owned(),
        level,
        checks: Vec::new(),
        diagnostics: Vec::new(),
    };
    let model = match Model::from_yaml(source) {
        Ok(model) => {
            report.checks.push(CheckResult {
                level: Level::V0,
                name: "model schema".to_owned(),
                passed: true,
                detail: format!("{} loaded", cetus_model::SCHEMA_ID),
            });
            model
        }
        Err(error) => {
            report.checks.push(CheckResult {
                level: Level::V0,
                name: "model schema".to_owned(),
                passed: false,
                detail: error.to_string(),
            });
            report
                .diagnostics
                .push(Diagnostic::error("E0003", error.to_string(), None));
            return report;
        }
    };
    if level >= Level::V1 {
        let (plan, diagnostics) = compile_model_with_warnings(&model);
        let errors = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        let warnings = diagnostics.len() - errors;
        report.diagnostics.extend(diagnostics);
        match plan {
            Some(plan) => {
                report.checks.push(CheckResult {
                    level: Level::V1,
                    name: "model compiles".to_owned(),
                    passed: true,
                    detail: if warnings == 0 {
                        format!("compiled {}@{}", plan.model_id, plan.model_version)
                    } else {
                        format!(
                            "compiled {}@{}; {warnings} warning(s)",
                            plan.model_id, plan.model_version
                        )
                    },
                });
                if level >= Level::V2 {
                    numeric::check(&model, &Arc::new(plan), &mut report);
                }
            }
            None => {
                report.checks.push(CheckResult {
                    level: Level::V1,
                    name: "model compiles".to_owned(),
                    passed: false,
                    detail: format!("{errors} diagnostic(s)"),
                });
            }
        }
    }
    report
}
