//! Module target verification: V0 schema, V1 compilation and V2 numerics.

use std::sync::Arc;

use cetus_compile::{Diagnostic, compile_model};
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
        match compile_model(&model) {
            Ok(plan) => {
                report.checks.push(CheckResult {
                    level: Level::V1,
                    name: "model compiles".to_owned(),
                    passed: true,
                    detail: format!("compiled {}@{}", plan.model_id, plan.model_version),
                });
                if level >= Level::V2 {
                    numeric::check(&model, &Arc::new(plan), &mut report);
                }
            }
            Err(diagnostics) => {
                let count = diagnostics.len();
                report.checks.push(CheckResult {
                    level: Level::V1,
                    name: "model compiles".to_owned(),
                    passed: false,
                    detail: format!("{count} diagnostic(s)"),
                });
                report.diagnostics.extend(diagnostics);
            }
        }
    }
    report
}
