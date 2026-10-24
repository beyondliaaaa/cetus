//! Teaching-task checks: runnable misconceptions, setup spaces and variants.

use std::collections::BTreeSet;

use cetus_compile::{Diagnostic, Diagnostics};
use cetus_model::{Model, ParameterRole};

use crate::patch::set_model_path;
use crate::schema::{Hole, Lesson, Scenario, SetupSpace, TeachingTask};

pub(super) fn check(lesson: &Lesson, truth_model: &Model, diagnostics: &mut Diagnostics) {
    for task in &lesson.teaching {
        match task {
            TeachingTask::Native {
                id,
                holds,
                setup_space,
                variants,
                ..
            } => check_native(
                lesson,
                truth_model,
                id,
                holds,
                setup_space,
                variants,
                diagnostics,
            ),
            TeachingTask::Subagent {
                id,
                gives,
                scenarios,
                ..
            } => check_subagent(lesson, truth_model, id, gives, scenarios, diagnostics),
        }
    }
}

fn check_native(
    lesson: &Lesson,
    truth_model: &Model,
    id: &str,
    holds: &str,
    setup_space: &SetupSpace,
    variants: &[String],
    diagnostics: &mut Diagnostics,
) {
    if !holds_applies(lesson, truth_model, holds) {
        diagnostics.push(
            Diagnostic::error(
                "E5401",
                format!(
                    "teaching task `{id}` holds `{holds}`, which is not a runnable model misconception"
                ),
                Some(&format!("teaching.{id}.holds")),
            )
            .with_help(
                "holds must reference a misconception whose patch applies to the model, not \
                 an L1 causal-arrow patch",
            ),
        );
    }
    check_setup_space(lesson, truth_model, id, setup_space, diagnostics);
    check_variants(lesson, id, holds, variants, diagnostics);
}

fn holds_applies(lesson: &Lesson, truth_model: &Model, holds: &str) -> bool {
    let Some(misconception) = lesson
        .misconceptions()
        .find(|misconception| misconception.id == holds)
    else {
        return false;
    };
    if misconception.patch.is_empty() {
        return false;
    }
    let mut model = truth_model.clone();
    misconception
        .patch
        .iter()
        .all(|(path, value)| set_model_path(&mut model, path, value).is_ok())
}

fn check_setup_space(
    lesson: &Lesson,
    truth_model: &Model,
    id: &str,
    setup_space: &SetupSpace,
    diagnostics: &mut Diagnostics,
) {
    if let Some(inherit) = &setup_space.inherit
        && lesson.teaching_task(inherit).is_none()
    {
        diagnostics.push(Diagnostic::error(
            "E5401",
            format!("teaching task `{id}` inherits unknown teaching task `{inherit}`"),
            Some(&format!("teaching.{id}.setup_space.inherit")),
        ));
    }
    for instrument in &setup_space.instruments {
        if !lesson
            .instruments
            .iter()
            .any(|declared| declared.id == *instrument)
        {
            diagnostics.push(Diagnostic::error(
                "E5401",
                format!("teaching task `{id}` uses undeclared instrument `{instrument}`"),
                Some(&format!("teaching.{id}.setup_space.instruments")),
            ));
        }
    }
    for name in setup_space.controls.keys() {
        let valid = truth_model
            .parameters
            .get(name)
            .is_some_and(|spec| matches!(spec.role, ParameterRole::Control | ParameterRole::Input));
        if !valid {
            diagnostics.push(Diagnostic::error(
                "E5401",
                format!(
                    "teaching task `{id}` control `{name}` is not a control or input parameter"
                ),
                Some(&format!("teaching.{id}.setup_space.controls.{name}")),
            ));
        }
    }
    for name in setup_space.initial.keys() {
        let known =
            truth_model.stocks.contains_key(name) || truth_model.observables.contains_key(name);
        if !known {
            diagnostics.push(Diagnostic::error(
                "E5401",
                format!("teaching task `{id}` initial value `{name}` is not a stock or observable"),
                Some(&format!("teaching.{id}.setup_space.initial.{name}")),
            ));
        }
    }
    if setup_space.duration.is_none() && setup_space.inherit.is_none() {
        diagnostics.push(
            Diagnostic::error(
                "E5401",
                format!("teaching task `{id}` declares neither a duration nor an inherited setup"),
                Some(&format!("teaching.{id}.setup_space.duration")),
            )
            .with_help("set setup_space.duration.max or inherit another teaching task's setup"),
        );
    }
}

fn check_variants(
    lesson: &Lesson,
    id: &str,
    holds: &str,
    variants: &[String],
    diagnostics: &mut Diagnostics,
) {
    for variant in variants {
        let valid = lesson.teaching_task(variant).and_then(TeachingTask::holds) == Some(holds);
        if !valid {
            diagnostics.push(Diagnostic::error(
                "E5401",
                format!("teaching task `{id}` variant `{variant}` must exist and hold `{holds}`"),
                Some(&format!("teaching.{id}.variants")),
            ));
        }
    }
}

fn check_subagent(
    lesson: &Lesson,
    truth_model: &Model,
    id: &str,
    gives: &[String],
    scenarios: &[Scenario],
    diagnostics: &mut Diagnostics,
) {
    let holes: BTreeSet<&str> = lesson.holes().map(Hole::id).collect();
    for give in gives {
        if !holes.contains(give.as_str()) {
            diagnostics.push(Diagnostic::error(
                "E5401",
                format!("subagent teaching task `{id}` gives unknown hole `{give}`"),
                Some(&format!("teaching.{id}.gives")),
            ));
        }
    }
    for scenario in scenarios {
        check_scenario(truth_model, id, scenario, diagnostics);
    }
}

fn check_scenario(
    truth_model: &Model,
    id: &str,
    scenario: &Scenario,
    diagnostics: &mut Diagnostics,
) {
    for name in scenario.setup.params.keys() {
        if !truth_model.parameters.contains_key(name) {
            diagnostics.push(Diagnostic::error(
                "E5401",
                format!(
                    "teaching task `{id}` scenario `{}` sets unknown parameter `{name}`",
                    scenario.id
                ),
                Some(&format!(
                    "teaching.{id}.scenarios.{}.setup.params.{name}",
                    scenario.id
                )),
            ));
        }
    }
    for name in scenario.setup.initial.keys() {
        if !truth_model.stocks.contains_key(name) {
            diagnostics.push(Diagnostic::error(
                "E5401",
                format!(
                    "teaching task `{id}` scenario `{}` sets unknown stock `{name}`",
                    scenario.id
                ),
                Some(&format!(
                    "teaching.{id}.scenarios.{}.setup.initial.{name}",
                    scenario.id
                )),
            ));
        }
    }
}
