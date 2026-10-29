//! V1 static check tests.

use cetus_compile::Severity;
use cetus_lesson::{
    Check, Hole, InitialRange, Lesson, MisconceptionSourceKind, TeachingTask, check_static,
};
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
fn given_positive_lesson_when_checked_statically_then_only_source_warnings_remain() {
    let diagnostics = check_static(&lesson_fixture("positive.lesson.yaml"), &solar());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == Severity::Warning),
        "{diagnostics:?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "W5201"
            && diagnostic.path.as_deref() == Some("misconceptions.m_square.source.kind")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "W5202"
            && diagnostic.path.as_deref() == Some("teaching.teach-natives-1.holds")
    }));
}

#[test]
fn given_documented_source_without_ref_when_checked_then_e5204_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let misconception = lesson
        .misconceptions
        .iter_mut()
        .find(|misconception| misconception.id == "m_square")
        .expect("m_square");
    misconception.source.kind = MisconceptionSourceKind::ResearchDocumented;
    misconception.source.reference = None;
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E5204"
            && diagnostic.path.as_deref() == Some("misconceptions.m_square.source.ref")
    }));
}

#[test]
fn given_documented_source_with_valid_ref_when_checked_then_no_source_error_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let misconception = lesson
        .misconceptions
        .iter_mut()
        .find(|misconception| misconception.id == "m_square")
        .expect("m_square");
    misconception.source.kind = MisconceptionSourceKind::ExamItem;
    misconception.source.reference = Some("src:exam.2019.q3".to_owned());
    let diagnostics = check_static(&lesson, &solar());
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E5204"
            && diagnostic
                .path
                .as_deref()
                .is_some_and(|path| path.starts_with("misconceptions.m_square.source"))
    }));
}

#[test]
fn given_mc_source_ref_when_checked_then_no_source_error_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let misconception = lesson
        .misconceptions
        .iter_mut()
        .find(|misconception| misconception.id == "m_square")
        .expect("m_square");
    misconception.source.kind = MisconceptionSourceKind::ResearchDocumented;
    misconception.source.reference = Some("mc:meta.modeling.stock-follows-inflow".to_owned());
    let diagnostics = check_static(&lesson, &solar());
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E5204"
            && diagnostic
                .path
                .as_deref()
                .is_some_and(|path| path.starts_with("misconceptions.m_square.source"))
    }));
}

#[test]
fn given_mc_source_ref_with_empty_id_when_checked_then_e5204_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let misconception = lesson
        .misconceptions
        .iter_mut()
        .find(|misconception| misconception.id == "m_square")
        .expect("m_square");
    misconception.source.kind = MisconceptionSourceKind::ResearchDocumented;
    misconception.source.reference = Some("mc:".to_owned());
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E5204"
            && diagnostic.path.as_deref() == Some("misconceptions.m_square.source.ref")
    }));
}

#[test]
fn given_bad_source_ref_when_checked_then_e5204_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let misconception = lesson
        .misconceptions
        .iter_mut()
        .find(|misconception| misconception.id == "m_square")
        .expect("m_square");
    misconception.source.kind = MisconceptionSourceKind::ResearchDocumented;
    misconception.source.reference = Some("not-a-src".to_owned());
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E5204"
            && diagnostic.path.as_deref() == Some("misconceptions.m_square.source.ref")
    }));
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
fn given_empty_h1_when_checked_then_e0003_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let hole = lesson
        .player_model
        .holes
        .iter_mut()
        .find(|hole| hole.id() == "h_pv")
        .expect("h_pv");
    if let Hole::Equation { hints, .. } = hole {
        hints.h1.clear();
    }
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E0003"
            && diagnostic
                .path
                .as_deref()
                .is_some_and(|path| path == "player_model.holes.h_pv.hints.h1")
    }));
}

#[test]
fn given_empty_q0_when_checked_then_e0003_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let hole = lesson
        .player_model
        .holes
        .iter_mut()
        .find(|hole| hole.id() == "h_pv")
        .expect("h_pv");
    if let Hole::Equation { hints, .. } = hole {
        hints.q0 = Some(String::new());
    }
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E0003"
            && diagnostic
                .path
                .as_deref()
                .is_some_and(|path| path == "player_model.holes.h_pv.hints.q0")
    }));
}

#[test]
fn given_rel_tol_at_one_when_checked_then_e0003_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    lesson.tasks[1].check = Some(Check::RelTol { tol: 1.0 });
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E0003" && diagnostic.path.as_deref() == Some("tasks.t2.check.tol")
    }));
}

#[test]
fn given_param_within_zero_when_checked_then_e0003_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    lesson.tasks[2].check = Some(Check::ParamWithin { rel_tol: 0.0 });
    let diagnostics = check_static(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E0003" && diagnostic.path.as_deref() == Some("tasks.t3.check.rel_tol")
    }));
}

#[test]
fn given_valid_rel_tol_when_checked_then_no_tolerance_error_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    lesson.tasks[1].check = Some(Check::RelTol { tol: 0.1 });
    let diagnostics = check_static(&lesson, &solar());
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E0003"
            && diagnostic
                .path
                .as_deref()
                .is_some_and(|path| path.starts_with("tasks.t2.check.tol"))
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

#[test]
fn given_task_initial_on_observable_when_checked_then_e5205_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let t4 = lesson
        .tasks
        .iter_mut()
        .find(|task| task.id == "t4")
        .expect("t4");
    let setup = t4.setup.as_mut().expect("t4 setup");
    setup.initial.clear();
    setup
        .initial
        .insert("charge_pct".to_owned(), "0.3".to_owned());
    let diagnostics = check_static(&lesson, &solar());
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E5205"
                && diagnostic.path.as_deref() == Some("tasks.t4.setup.initial.charge_pct")
        }),
        "{diagnostics:?}"
    );
}

#[test]
fn given_setup_space_initial_on_observable_when_checked_then_e5205_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let task = lesson
        .teaching
        .iter_mut()
        .find(|task| task.id() == "teach-natives-1")
        .expect("teach-natives-1");
    let TeachingTask::Native { setup_space, .. } = task else {
        panic!("teach-natives-1 must be a native task");
    };
    setup_space.initial.clear();
    setup_space.initial.insert(
        "charge_pct".to_owned(),
        InitialRange {
            range: ["0.2".to_owned(), "1.0".to_owned()],
        },
    );
    let diagnostics = check_static(&lesson, &solar());
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E5205"
                && diagnostic.path.as_deref()
                    == Some("teaching.teach-natives-1.setup_space.initial.charge_pct")
        }),
        "{diagnostics:?}"
    );
}

#[test]
fn given_setup_space_initial_with_wrong_dimension_when_checked_then_e1201_is_reported() {
    let mut lesson = lesson_fixture("positive.lesson.yaml");
    let task = lesson
        .teaching
        .iter_mut()
        .find(|task| task.id() == "teach-natives-1")
        .expect("teach-natives-1");
    let TeachingTask::Native { setup_space, .. } = task else {
        panic!("teach-natives-1 must be a native task");
    };
    setup_space.initial.clear();
    setup_space.initial.insert(
        "charge".to_owned(),
        InitialRange {
            range: ["0.2 * area".to_owned(), "capacity".to_owned()],
        },
    );
    let diagnostics = check_static(&lesson, &solar());
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E1201"
                && diagnostic.path.as_deref()
                    == Some("teaching.teach-natives-1.setup_space.initial.charge.range[0]")
        }),
        "{diagnostics:?}"
    );
}
