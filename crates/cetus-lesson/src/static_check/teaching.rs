//! Teaching-task checks: runnable misconceptions, setup spaces and variants.

use std::collections::BTreeSet;

use cetus_compile::{CompiledPlan, Diagnostic, Diagnostics, parse_in_plan};
use cetus_model::{Model, ParameterRole};
use cetus_units::Dim;

use crate::patch::set_model_path;
use crate::schema::{Hole, InitialRange, Lesson, Scenario, SetupSpace, TeachingTask};

pub(super) fn check(
    lesson: &Lesson,
    truth_model: &Model,
    plan: &CompiledPlan,
    diagnostics: &mut Diagnostics,
) {
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
                plan,
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

#[allow(clippy::too_many_arguments)]
fn check_native(
    lesson: &Lesson,
    truth_model: &Model,
    plan: &CompiledPlan,
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
    if let Some(misconception) = lesson
        .misconceptions()
        .find(|misconception| misconception.id == holds)
        && misconception.source.is_hypothesized()
    {
        diagnostics.push(
            Diagnostic::warning(
                "W5202",
                format!(
                    "teaching task `{id}` holds hypothesized misconception `{holds}`; \
                     prefer a sourced misconception"
                ),
                Some(&format!("teaching.{id}.holds")),
            )
            .with_help("replace it once research-misconceptions finds a sourced entry"),
        );
    }
    check_setup_space(lesson, truth_model, plan, id, setup_space, diagnostics);
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
    plan: &CompiledPlan,
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
    for (name, range) in &setup_space.initial {
        if !truth_model.stocks.contains_key(name) {
            diagnostics.push(Diagnostic::error(
                "E5205",
                format!("teaching task `{id}` initial value `{name}` is not a stock"),
                Some(&format!("teaching.{id}.setup_space.initial.{name}")),
            ));
            continue;
        }
        check_initial_range(
            name,
            range,
            plan,
            &format!("teaching.{id}.setup_space.initial.{name}"),
            diagnostics,
        );
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

fn check_initial_range(
    name: &str,
    range: &InitialRange,
    plan: &CompiledPlan,
    path: &str,
    diagnostics: &mut Diagnostics,
) {
    let Some(stock) = plan.stocks.iter().find(|stock| stock.name == name) else {
        return;
    };
    let Some(slot) = plan.layout.slots.get(stock.slot) else {
        return;
    };
    for (index, bound) in range.range.iter().enumerate() {
        let bound_path = format!("{path}.range[{index}]");
        match parse_in_plan(bound, plan) {
            Ok(typed) => {
                let dim = typed.ty.dim();
                let matches =
                    dim == Some(slot.unit.dim()) || dim.is_some_and(Dim::is_dimensionless);
                if !matches {
                    let found = dim.map_or_else(|| "non-numeric".to_owned(), ToString::to_string);
                    diagnostics.push(Diagnostic::error(
                        "E1201",
                        format!(
                            "initial bound `{bound}` has dimension {found}, stock `{name}` has {}",
                            slot.unit.dim()
                        ),
                        Some(&bound_path),
                    ));
                }
            }
            Err(error) => {
                let span = error.span();
                diagnostics.push(
                    Diagnostic::error(error.code(), error.message(), Some(&bound_path))
                        .with_span((span.start, span.end)),
                );
            }
        }
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
