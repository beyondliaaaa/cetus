//! V0/V1 verification of real content targets.

use std::path::{Path, PathBuf};

use cetus_compile::Severity;
use cetus_verify::{Level, VerifyError, verify_path};

fn content_root() -> PathBuf {
    cetus_testkit::workspace_root().join("content")
}

fn solar_module() -> PathBuf {
    module("energy", "solar-battery")
}

fn module(domain: &str, name: &str) -> PathBuf {
    content_root()
        .join("modules")
        .join(domain)
        .join(name)
        .join("module.yaml")
}

fn lesson(name: &str) -> PathBuf {
    content_root()
        .join("lessons")
        .join(name)
        .join("lesson.yaml")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("read fixture")
}

fn write_temp(name: &str, contents: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("cetus-verify-{name}-{}.yaml", std::process::id()));
    std::fs::write(&path, contents).expect("write temporary YAML");
    path
}

fn replace_module_line(source: &str, module: &str) -> String {
    source
        .lines()
        .map(|line| {
            if line.starts_with("module:") {
                format!("module: {module}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn given_solar_module_when_verified_at_v1_then_it_passes() {
    let report = verify_path(&solar_module(), Level::V1).expect("verify");
    assert!(report.passed(), "{report:?}");
    assert!(report.checks.iter().any(|check| check.level == Level::V0));
    assert!(report.checks.iter().any(|check| check.level == Level::V1));
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    );
}

#[test]
fn given_solar_module_when_verified_at_v0_then_only_v0_runs() {
    let report = verify_path(&solar_module(), Level::V0).expect("verify");
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].name, "model schema");
}

#[test]
fn given_positive_lesson_when_verified_at_v1_then_it_passes() {
    let report = verify_path(&lesson("energy.battery-night"), Level::V1).expect("verify");
    assert!(report.passed(), "{report:?}");
    assert!(
        report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error),
        "{:?}",
        report.diagnostics
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W5201"),
        "{:?}",
        report.diagnostics
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W5202"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn given_solar_module_when_verified_at_v2_then_it_passes() {
    let report = verify_path(&solar_module(), Level::V2).expect("verify");
    assert!(report.passed(), "{report:?}");
    assert!(report.checks.iter().any(|check| check.level == Level::V2));
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn given_positive_lesson_when_verified_at_v2_then_it_passes() {
    let report = verify_path(&lesson("energy.battery-night"), Level::V2).expect("verify");
    assert!(report.passed(), "{report:?}");
    assert!(report.checks.iter().any(|check| check.level == Level::V2));
}

#[test]
fn given_discrete_time_module_when_verified_at_v2_then_convergence_is_skipped() {
    let report = verify_path(&module("math", "logistic-map"), Level::V2).expect("verify");
    assert!(report.passed(), "{report:?}");
    assert!(
        report.checks.iter().any(|check| {
            check.name == "step-size convergence" && check.detail.contains("skipped")
        }),
        "{report:?}"
    );
}

#[test]
fn given_divergent_model_when_verified_at_v2_then_it_fails() {
    let source = r#"
schema: cetus.model/0.1
id: test.divergent
version: 1.0.0
title: { zh-CN: "发散模型" }
tier: empirical
paradigms: [sd]
time: { unit: h, dt: 0.5, horizon: 100, integrator: euler }
parameters:
  rate: { unit: "1/h", value: 1, source: "param:TODO-expert" }
stocks:
  x: { unit: "1", initial: "1" }
flows:
  growth: { to: x, unit: "1/h", eq: "rate * x * x" }
observables:
  x: { unit: "1", eq: "x" }
"#;
    let path = write_temp("divergent", source);
    let report = verify_path(&path, Level::V2).expect("verify");
    let _ = std::fs::remove_file(&path);
    assert!(!report.passed(), "{report:?}");
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E7201"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn given_content_lesson_when_loaded_then_t4_reads_the_observable() {
    let source = read(&lesson("energy.battery-night"));
    let lesson = cetus_lesson::Lesson::from_yaml(&source).expect("parse lesson");
    let t4 = lesson
        .tasks
        .iter()
        .find(|task| task.id == "t4")
        .expect("t4");
    assert_eq!(t4.answer.as_deref(), Some("first_time(charge_pct <= 0)"));
}

#[test]
fn given_broken_lesson_when_verified_at_v1_then_it_fails_with_e5203_and_e5401() {
    let report = verify_path(&lesson("energy.battery-night-broken"), Level::V1).expect("verify");
    assert!(!report.passed(), "{report:?}");
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E5203"),
        "{:?}",
        report.diagnostics
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E5401"),
        "{:?}",
        report.diagnostics
    );
    assert!(
        report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E5401"
                && diagnostic
                    .path
                    .as_deref()
                    .is_some_and(|path| path.starts_with("teaching."))
        }),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn given_broken_lesson_when_verified_at_v1_then_v0_passes_and_v1_fails() {
    // The schema is valid, so V0 passes and the requested V1 check runs.
    let report = verify_path(&lesson("energy.battery-night-broken"), Level::V1).expect("verify");
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.level == Level::V0 && check.passed)
    );
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.level == Level::V1 && !check.passed)
    );
}

#[test]
fn given_lesson_with_unknown_schema_when_verified_then_v0_fails_and_v1_does_not_run() {
    let path = write_temp("unknown-schema", "schema: cetus.lesson/9.9\n");
    let report = verify_path(&path, Level::V1).expect("verify");
    assert_eq!(report.checks.len(), 1);
    assert!(!report.checks[0].passed);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0003"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn given_lesson_with_missing_module_when_verified_at_v1_then_e2101_is_reported() {
    let source = read(&lesson("energy.battery-night"));
    let source = replace_module_line(&source, "missing.module@^1");
    let root = std::env::temp_dir().join(format!(
        "cetus-verify-missing-module-{}",
        std::process::id()
    ));
    let lesson_dir = root.join("content").join("lessons").join("missing-module");
    std::fs::create_dir_all(&lesson_dir).expect("create lesson dir");
    std::fs::create_dir_all(root.join("content").join("modules")).expect("create modules dir");
    let path = lesson_dir.join("lesson.yaml");
    std::fs::write(&path, source).expect("write lesson");
    let report = verify_path(&path, Level::V1).expect("verify");
    let _ = std::fs::remove_dir_all(&root);
    assert!(!report.passed(), "{report:?}");
    assert!(
        report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E2101" && diagnostic.message.contains("cannot load module")
        }),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn given_v3_level_when_parsed_then_unsupported() {
    assert!(matches!(
        Level::parse("V3"),
        Err(VerifyError::UnsupportedLevel(level)) if level == "V3"
    ));
    assert_eq!(Level::parse("v0").expect("parse v0"), Level::V0);
    assert_eq!(Level::parse("v2").expect("parse v2"), Level::V2);
}

#[test]
fn given_report_when_rendered_then_json_has_ok_and_checks_root() {
    let report = verify_path(&solar_module(), Level::V1).expect("verify");
    let json = report.to_json();
    assert_eq!(json["ok"], true);
    assert!(json["checks"].is_array());
    let markdown = report.to_markdown();
    assert!(markdown.starts_with("# Cetus verification report"));
    assert!(markdown.contains("PASS"), "{markdown}");

    let broken = verify_path(&lesson("energy.battery-night-broken"), Level::V1).expect("verify");
    assert!(broken.to_markdown().contains("FAIL"));
}

#[test]
fn given_module_with_compile_error_when_verified_then_v1_fails() {
    let source = read(&solar_module());
    let broken = source.replace(
        "  charging:\n    to: charge\n    unit: W",
        "  charging:\n    to: charge\n    unit: \"kW*h\"",
    );
    assert_ne!(broken, source, "replacement did not apply");
    let path = write_temp("bad-module-unit", &broken);
    let report = verify_path(&path, Level::V1).expect("verify");
    assert!(!report.passed(), "{report:?}");
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.level == Level::V1 && !check.passed),
        "{:?}",
        report.checks
    );
    assert!(!report.diagnostics.is_empty(), "{report:?}");
}
