//! Lesson target verification: V0 schema, V1 static checks and V2 numerics.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cetus_compile::{Diagnostic, Diagnostics, Severity, compile_model};
use cetus_lesson::{Lesson, apply_truth_overrides, check_static};
use cetus_model::Model;

use crate::Level;
use crate::numeric;
use crate::report::{CheckResult, Report};

pub(super) fn verify(target: &str, path: &Path, source: &str, level: Level) -> Report {
    let mut report = Report {
        target: target.to_owned(),
        level,
        checks: Vec::new(),
        diagnostics: Vec::new(),
    };
    let lesson = match Lesson::from_yaml(source) {
        Ok(lesson) => {
            report.checks.push(CheckResult {
                level: Level::V0,
                name: "lesson schema".to_owned(),
                passed: true,
                detail: format!("{} loaded", cetus_lesson::SCHEMA_ID),
            });
            lesson
        }
        Err(error) => {
            report.checks.push(CheckResult {
                level: Level::V0,
                name: "lesson schema".to_owned(),
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
        match static_checks(path, &lesson) {
            Ok(diagnostics) => {
                let errors = diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.severity == Severity::Error)
                    .count();
                let warnings = diagnostics.len() - errors;
                report.checks.push(CheckResult {
                    level: Level::V1,
                    name: "lesson static checks".to_owned(),
                    passed: errors == 0,
                    detail: if diagnostics.is_empty() {
                        "no diagnostics".to_owned()
                    } else {
                        format!("{errors} error(s), {warnings} warning(s)")
                    },
                });
                report.diagnostics.extend(diagnostics);
            }
            Err(diagnostic) => {
                report.checks.push(CheckResult {
                    level: Level::V1,
                    name: "lesson static checks".to_owned(),
                    passed: false,
                    detail: diagnostic.message.clone(),
                });
                report.diagnostics.push(*diagnostic);
            }
        }
    }
    if level >= Level::V2 {
        check_numeric(path, &lesson, &mut report);
    }
    report
}

fn check_numeric(lesson_path: &Path, lesson: &Lesson, report: &mut Report) {
    let model = match load_module(lesson_path, lesson) {
        Ok(model) => model,
        Err(diagnostic) => {
            report.checks.push(CheckResult {
                level: Level::V2,
                name: "lesson numerics".to_owned(),
                passed: false,
                detail: diagnostic.message.clone(),
            });
            return;
        }
    };
    let (truth, _overrides) = apply_truth_overrides(lesson, &model);
    match compile_model(&truth) {
        Ok(plan) => numeric::check(&truth, &Arc::new(plan), report),
        Err(diagnostics) => {
            report.checks.push(CheckResult {
                level: Level::V2,
                name: "lesson numerics".to_owned(),
                passed: false,
                detail: format!(
                    "truth model failed to compile: {} diagnostic(s)",
                    diagnostics.len()
                ),
            });
        }
    }
}

fn static_checks(lesson_path: &Path, lesson: &Lesson) -> Result<Diagnostics, Box<Diagnostic>> {
    let model = load_module(lesson_path, lesson)?;
    Ok(check_static(lesson, &model))
}

fn load_module(lesson_path: &Path, lesson: &Lesson) -> Result<Model, Box<Diagnostic>> {
    let requirement = lesson.module_requirement();
    let Some((domain, name)) = requirement.id.split_once('.') else {
        return Err(Box::new(Diagnostic::error(
            "E2101",
            format!(
                "module reference `{}` is not `<domain>.<name>`",
                requirement.id
            ),
            Some("module"),
        )));
    };
    let root = content_root(lesson_path).ok_or_else(|| {
        Box::new(
            Diagnostic::error(
                "E2101",
                "cannot locate the content root containing `modules/` and `lessons/`",
                Some("module"),
            )
            .with_help("run cetus verify from the repository root or from inside content/"),
        )
    })?;
    let module_path = root
        .join("modules")
        .join(domain)
        .join(name)
        .join("module.yaml");
    let source = std::fs::read_to_string(&module_path).map_err(|error| {
        Box::new(Diagnostic::error(
            "E2101",
            format!(
                "cannot load module `{}` from {}: {error}",
                requirement.id,
                module_path.display()
            ),
            Some("module"),
        ))
    })?;
    let model = Model::from_yaml(&source).map_err(|error| {
        Box::new(Diagnostic::error(
            "E0003",
            format!("module `{}` failed to load: {error}", requirement.id),
            Some("module"),
        ))
    })?;
    check_major(&requirement.id, requirement.requirement.as_deref(), &model)?;
    Ok(model)
}

fn check_major(id: &str, requirement: Option<&str>, model: &Model) -> Result<(), Box<Diagnostic>> {
    let Some(required) = requirement.and_then(requirement_major) else {
        return Ok(());
    };
    let Some(actual) = leading_major(&model.version) else {
        return Ok(());
    };
    if required == actual {
        return Ok(());
    }
    Err(Box::new(Diagnostic::error(
        "E2101",
        format!(
            "lesson requires module `{id}` major version {required}, but the module is version {}",
            model.version
        ),
        Some("module"),
    )))
}

fn requirement_major(requirement: &str) -> Option<u64> {
    leading_major(requirement.strip_prefix('^')?)
}

fn leading_major(version: &str) -> Option<u64> {
    version.split('.').next()?.parse().ok()
}

fn content_root(lesson_path: &Path) -> Option<PathBuf> {
    let parent = lesson_path.parent()?;
    for ancestor in parent.ancestors() {
        if is_content_root(ancestor) {
            return Some(ancestor.to_path_buf());
        }
    }
    let cwd = std::env::current_dir().ok()?;
    let candidate = cwd.join("content");
    is_content_root(&candidate).then_some(candidate)
}

fn is_content_root(path: &Path) -> bool {
    path.join("modules").is_dir() && path.join("lessons").is_dir()
}
