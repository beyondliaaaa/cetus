//! V1 static check tests.

use cetus_lesson::{Hole, Lesson, check_static};
use cetus_model::Model;

fn solar() -> Model {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/modules/energy/solar-battery/module.yaml"
    );
    let source = std::fs::read_to_string(path).expect("read solar module");
    Model::from_yaml(&source).expect("parse solar module")
}

fn lesson_fixture(name: &str) -> Lesson {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(path).expect("read lesson fixture");
    Lesson::from_yaml(&source).expect("parse lesson fixture")
}

#[test]
fn given_positive_lesson_when_checked_statically_then_no_diagnostics_are_reported() {
    let diagnostics = check_static(&lesson_fixture("positive.lesson.yaml"), &solar());
    assert_eq!(diagnostics, [], "{diagnostics:?}");
}

#[test]
fn given_hole_with_missing_target_when_checked_statically_then_e5203_is_reported() {
    let diagnostics = check_static(&lesson_fixture("broken.lesson.yaml"), &solar());
    let hit = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E5203"
                && diagnostic.path.as_deref() == Some("player_model.holes.h_pv.target")
        })
        .unwrap_or_else(|| panic!("missing E5203: {diagnostics:?}"));
    assert!(hit.help.is_some());
}

#[test]
fn given_native_holds_causal_patch_when_checked_statically_then_e5401_is_reported() {
    let diagnostics = check_static(&lesson_fixture("broken.lesson.yaml"), &solar());
    let hit = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E5401"
                && diagnostic.path.as_deref() == Some("teaching.teach-natives-1.holds")
        })
        .unwrap_or_else(|| panic!("missing E5401: {diagnostics:?}"));
    assert!(
        hit.help
            .as_deref()
            .is_some_and(|help| help.contains("causal-arrow"))
    );
}

#[test]
fn given_unknown_instrument_measure_when_checked_statically_then_e2101_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    lesson.instruments[0].measures = "does_not_exist".to_owned();
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E2101"
            && diagnostic.path.as_deref() == Some("instruments.lux_meter.measures")
    }));
}

#[test]
fn given_unknown_task_hole_when_checked_statically_then_e2101_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    lesson.tasks[1].hole = Some("missing_hole".to_owned());
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E2101" && diagnostic.path.as_deref() == Some("tasks.t2.hole")
    }));
}

#[test]
fn given_unknown_task_control_when_checked_statically_then_e2101_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    lesson.tasks[0].controls = vec!["missing_control".to_owned()];
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E2101" && diagnostic.path.as_deref() == Some("tasks.t1.controls")
    }));
}

#[test]
fn given_unknown_causal_from_when_checked_statically_then_e2101_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let layer = lesson
        .layers
        .as_mut()
        .and_then(|layers| layers.l1.as_mut())
        .expect("L1 layer");
    match layer.holes.first_mut() {
        Some(Hole::CausalLink { from, .. }) => *from = "does_not_exist".to_owned(),
        _ => panic!("first L1 hole must be a causal link"),
    }
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E2101" && diagnostic.path.as_deref() == Some("layers.L1.holes.c1.from")
    }));
}
