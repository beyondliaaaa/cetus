//! Schema parsing and strictness tests.

use cetus_lesson::{Hole, Lesson, LessonError};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).expect("read fixture")
}

fn positive_lesson() -> Lesson {
    Lesson::from_yaml(&fixture("positive.lesson.yaml")).expect("parse positive lesson")
}

#[test]
fn given_positive_lesson_when_parsed_then_layers_and_teaching_are_exposed() {
    let lesson = positive_lesson();
    assert_eq!(lesson.id, "lesson.energy.battery-night");
    assert_eq!(lesson.holes().count(), 5);
    assert_eq!(lesson.tasks().count(), 8);
    assert_eq!(lesson.misconceptions().count(), 3);

    let native = lesson
        .teaching_task("teach-natives-1")
        .expect("native teaching task");
    assert_eq!(native.holds(), Some("m_constant_sun"));
    assert_eq!(native.variants().len(), 1);
    assert_eq!(native.variants()[0], "teach-natives-1b");
    assert!(native.setup_space().is_some());

    let subagent = lesson
        .teaching_task("teach-subagent-1")
        .expect("subagent teaching task");
    assert_eq!(subagent.holds(), None);
    assert!(subagent.setup_space().is_none());
    assert!(lesson.teaching_task("missing").is_none());

    let causal = lesson
        .holes()
        .find(|hole| hole.id() == "c1")
        .expect("causal hole");
    assert!(causal.is_causal_link());
    assert_eq!(causal.target(), None);
    assert!(matches!(causal, Hole::CausalLink { .. }));
}

#[test]
fn given_unknown_check_field_when_parsed_then_it_is_rejected() {
    let broken = fixture("positive.lesson.yaml").replace(
        "check: { kind: expr_equiv }",
        "check: { kind: expr_equiv, bogus: 1 }",
    );
    let error = Lesson::from_yaml(&broken).expect_err("unknown check field must be rejected");
    assert!(matches!(error, LessonError::Yaml { .. }));
}

#[test]
fn given_unknown_top_level_field_when_parsed_then_it_is_rejected() {
    let broken = fixture("positive.lesson.yaml").replace(
        "schema: cetus.lesson/0.1\n",
        "schema: cetus.lesson/0.1\ntitle: nope\n",
    );
    let error = Lesson::from_yaml(&broken).expect_err("unknown field must be rejected");
    assert!(matches!(error, LessonError::Yaml { .. }));
}

#[test]
fn given_unknown_task_field_when_parsed_then_it_is_rejected() {
    let broken =
        fixture("positive.lesson.yaml").replace("kind: measure", "kind: measure\n    title: nope");
    let error = Lesson::from_yaml(&broken).expect_err("unknown task field must be rejected");
    assert!(matches!(error, LessonError::Yaml { .. }));
}

#[test]
fn given_unknown_hole_field_when_parsed_then_it_is_rejected() {
    let broken = fixture("positive.lesson.yaml").replace(
        "target: auxiliaries.pv_power.eq",
        "target: auxiliaries.pv_power.eq\n      surprise: nope",
    );
    let error = Lesson::from_yaml(&broken).expect_err("unknown hole field must be rejected");
    assert!(matches!(error, LessonError::Yaml { .. }));
}

#[test]
fn given_unsupported_schema_when_parsed_then_it_is_rejected() {
    let broken = fixture("positive.lesson.yaml").replace("cetus.lesson/0.1", "cetus.lesson/9.9");
    let error = Lesson::from_yaml(&broken).expect_err("unsupported schema must be rejected");
    assert!(matches!(error, LessonError::UnsupportedSchema(_)));
    assert_eq!(error.code(), "E0003");
}

#[test]
fn given_module_with_requirement_when_split_then_id_and_caret_range_are_returned() {
    let mut lesson = positive_lesson();
    let requirement = lesson.module_requirement();
    assert_eq!(requirement.id, "energy.solar-battery");
    assert_eq!(requirement.requirement.as_deref(), Some("^1"));

    lesson.module = "energy.solar-battery".to_owned();
    let requirement = lesson.module_requirement();
    assert_eq!(requirement.id, "energy.solar-battery");
    assert_eq!(requirement.requirement, None);
}

#[test]
fn given_positive_lesson_when_serialised_then_it_roundtrips() {
    let lesson = positive_lesson();
    let yaml = lesson.to_yaml().expect("serialise lesson");
    let again = Lesson::from_yaml(&yaml).expect("reparse lesson");
    assert_eq!(lesson, again);
}
