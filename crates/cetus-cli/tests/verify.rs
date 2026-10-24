//! CLI behaviour for `cetus verify`.

use std::process::{Command, Output};

fn lesson(name: &str) -> String {
    format!(
        "{}/../../content/lessons/{name}/lesson.yaml",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cetus"))
        .args(args)
        .output()
        .expect("run cetus")
}

fn parsed_stdout(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

#[test]
fn given_positive_lesson_when_verified_at_v1_json_then_exit_zero_and_ok() {
    let output = run(&[
        "verify",
        &lesson("energy.battery-night"),
        "--level",
        "V1",
        "--report",
        "json",
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(parsed_stdout(&output)["ok"], true);
}

#[test]
fn given_broken_lesson_when_verified_at_v1_json_then_exit_one_with_codes() {
    let output = run(&[
        "verify",
        &lesson("energy.battery-night-broken"),
        "--level",
        "V1",
        "--report",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let json = parsed_stdout(&output);
    assert_eq!(json["ok"], false);
    let codes: Vec<&str> = json["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect();
    assert!(codes.contains(&"E5203"), "{codes:?}");
    assert!(codes.contains(&"E5401"), "{codes:?}");
}

#[test]
fn given_v2_level_when_verified_then_exit_two() {
    let output = run(&[
        "verify",
        &lesson("energy.battery-night"),
        "--level",
        "V2",
        "--report",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unsupported verification level"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn given_positive_lesson_when_verified_at_v0_md_then_only_v0_runs() {
    let output = run(&[
        "verify",
        &lesson("energy.battery-night"),
        "--level",
        "V0",
        "--report",
        "md",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(
        stdout.starts_with("# Cetus verification report"),
        "{stdout}"
    );
    assert!(stdout.contains("PASS"), "{stdout}");
    assert!(!stdout.contains("V1"), "{stdout}");
}

#[test]
fn given_unknown_report_format_when_verified_then_exit_one() {
    let output = run(&[
        "verify",
        &lesson("energy.battery-night"),
        "--level",
        "V1",
        "--report",
        "xyz",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown report format"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
