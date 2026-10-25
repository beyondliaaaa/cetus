//! Schema parsing and strictness tests.

use cetus_lesson::{Check, Hole, Lesson, LessonError, MisconceptionSourceKind};

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
fn given_positive_lesson_when_parsed_then_misconception_sources_are_exposed() {
    let lesson = positive_lesson();
    let square = lesson
        .misconceptions()
        .find(|misconception| misconception.id == "m_square")
        .expect("m_square");
    assert_eq!(square.source.kind, MisconceptionSourceKind::Hypothesized);
    assert_eq!(square.source.reference, None);
    assert!(square.source.is_hypothesized());
    assert!(square.source.note.is_some());
}

#[test]
fn given_exam_item_source_when_parsed_then_it_is_exposed() {
    let lesson = Lesson::from_yaml(&fixture("positive.lesson.yaml").replace(
        "source: { kind: hypothesized, ref: null, note: \"待 research-misconceptions 调研\" }",
        "source: { kind: exam_item, ref: \"src:exam.2019.q3\", note: \"example\" }",
    ))
    .expect("parse");
    let square = lesson
        .misconceptions()
        .find(|misconception| misconception.id == "m_square")
        .expect("m_square");
    assert_eq!(square.source.kind, MisconceptionSourceKind::ExamItem);
    assert_eq!(square.source.reference.as_deref(), Some("src:exam.2019.q3"));
}

#[test]
fn given_missing_misconception_source_when_parsed_then_it_is_rejected() {
    let broken = fixture("positive.lesson.yaml").replace(
        "    source: { kind: hypothesized, ref: null, note: \"待 research-misconceptions 调研\" }\n",
        "",
    );
    let error = Lesson::from_yaml(&broken).expect_err("source is required");
    assert!(matches!(error, LessonError::Yaml { .. }));
}

#[test]
fn given_hole_hints_when_parsed_then_the_ladder_is_exposed() {
    let lesson = positive_lesson();
    let pv = lesson
        .holes()
        .find(|hole| hole.id() == "h_pv")
        .expect("h_pv");
    let hints = pv.hints().expect("h_pv hints");
    assert_eq!(hints.q0.as_deref(), Some("hint.pv.0"));
    assert_eq!(hints.h1, "hint.pv.1");
    assert_eq!(hints.h2, "hint.pv.2");
    assert_eq!(hints.h3, "hint.pv.3");

    let efficiency = lesson
        .holes()
        .find(|hole| hole.id() == "h_eff")
        .expect("h_eff");
    assert_eq!(efficiency.hints().expect("h_eff hints").q0, None);
}

#[test]
fn given_unknown_hint_field_when_parsed_then_it_is_rejected() {
    let broken = fixture("positive.lesson.yaml").replace(
        "hints: { q0: hint.pv.0, h1: hint.pv.1, h2: hint.pv.2, h3: hint.pv.3 }",
        "hints: { q0: hint.pv.0, h1: hint.pv.1, h2: hint.pv.2, h3: hint.pv.3, h4: nope }",
    );
    let error = Lesson::from_yaml(&broken).expect_err("unknown hint field must be rejected");
    assert!(matches!(error, LessonError::Yaml { .. }));
}

#[test]
fn given_rel_tol_check_when_parsed_then_tolerance_is_a_number() {
    let lesson = Lesson::from_yaml(&fixture("positive.lesson.yaml").replace(
        "check: { kind: param_within, rel_tol: 0.08 }",
        "check: { kind: rel_tol, tol: 0.1 }",
    ))
    .expect("parse");
    match &lesson.tasks[2].check {
        Some(Check::RelTol { tol }) => assert_eq!(*tol, 0.1),
        other => panic!("expected RelTol, got {other:?}"),
    }
}

#[test]
fn given_positive_lesson_when_parsed_then_t4_reads_the_observable() {
    let lesson = positive_lesson();
    let t4 = lesson
        .tasks
        .iter()
        .find(|task| task.id == "t4")
        .expect("t4");
    assert_eq!(t4.answer.as_deref(), Some("first_time(charge_pct <= 0)"));
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
