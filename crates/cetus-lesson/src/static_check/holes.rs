//! Hole, task and misconception checks, for the base layer and L0-L3.

use std::collections::BTreeSet;

use cetus_compile::{Diagnostic, Diagnostics, compile_model};
use cetus_model::Model;

use super::layers;
use crate::patch::{PathError, set_model_path};
use crate::schema::{Hole, Lesson, Misconception, Task};

pub(super) fn check_base(
    lesson: &Lesson,
    truth_model: &Model,
    names: &BTreeSet<String>,
    diagnostics: &mut Diagnostics,
) {
    for hole in &lesson.player_model.holes {
        let prefix = format!("player_model.holes.{}", hole.id());
        check_hole(hole, truth_model, names, &prefix, diagnostics);
    }
    let all_holes: BTreeSet<&str> = lesson.holes().map(Hole::id).collect();
    check_tasks(&lesson.tasks, &all_holes, truth_model, "tasks", diagnostics);
    let task_ids: BTreeSet<&str> = lesson.tasks.iter().map(|task| task.id.as_str()).collect();
    check_misconceptions(
        &lesson.misconceptions,
        truth_model,
        "misconceptions",
        &BTreeSet::new(),
        &task_ids,
        diagnostics,
    );
}

pub(super) fn check_layers(
    lesson: &Lesson,
    truth_model: &Model,
    names: &BTreeSet<String>,
    diagnostics: &mut Diagnostics,
) {
    for (layer_id, layer) in layers(lesson) {
        let prefix = format!("layers.{}", layer_id.label());
        for hole in &layer.holes {
            let hole_prefix = format!("{prefix}.holes.{}", hole.id());
            check_hole(hole, truth_model, names, &hole_prefix, diagnostics);
        }
        let mut allowed: BTreeSet<&str> = lesson.player_model.holes.iter().map(Hole::id).collect();
        allowed.extend(layer.holes.iter().map(Hole::id));
        check_tasks(
            &layer.tasks,
            &allowed,
            truth_model,
            &format!("{prefix}.tasks"),
            diagnostics,
        );

        let mut causal: BTreeSet<&str> = lesson
            .player_model
            .holes
            .iter()
            .filter(|hole| hole.is_causal_link())
            .map(Hole::id)
            .collect();
        causal.extend(
            layer
                .holes
                .iter()
                .filter(|hole| hole.is_causal_link())
                .map(Hole::id),
        );
        let task_ids: BTreeSet<&str> = layer.tasks.iter().map(|task| task.id.as_str()).collect();
        check_misconceptions(
            &layer.misconceptions,
            truth_model,
            &format!("{prefix}.misconceptions"),
            &causal,
            &task_ids,
            diagnostics,
        );
    }
}

fn check_hole(
    hole: &Hole,
    truth_model: &Model,
    names: &BTreeSet<String>,
    prefix: &str,
    diagnostics: &mut Diagnostics,
) {
    if let Some(hints) = hole.hints() {
        check_hint(&hints.h1, "h1", prefix, diagnostics);
        check_hint(&hints.h2, "h2", prefix, diagnostics);
        check_hint(&hints.h3, "h3", prefix, diagnostics);
        if let Some(question) = &hints.q0 {
            check_hint(question, "q0", prefix, diagnostics);
        }
    }
    if let Some(target) = hole.target() {
        let mut model = truth_model.clone();
        if matches!(
            set_model_path(&mut model, target, "0"),
            Err(PathError::NotFound)
        ) {
            diagnostics.push(
                Diagnostic::error(
                    "E5203",
                    format!(
                        "hole `{}` targets `{target}`, which does not exist in the model",
                        hole.id()
                    ),
                    Some(&format!("{prefix}.target")),
                )
                .with_help(
                    "targets are parameters.<name>.value, stocks.<name>.initial, \
                     auxiliaries.<name>.eq, flows.<name>.eq or observables.<name>.eq",
                ),
            );
        }
    } else if let Hole::CausalLink { from, to, .. } = hole {
        if !names.contains(from) {
            diagnostics.push(Diagnostic::error(
                "E2101",
                format!(
                    "causal link `{}` names unknown quantity `{from}`",
                    hole.id()
                ),
                Some(&format!("{prefix}.from")),
            ));
        }
        if !names.contains(to) {
            diagnostics.push(Diagnostic::error(
                "E2101",
                format!("causal link `{}` names unknown quantity `{to}`", hole.id()),
                Some(&format!("{prefix}.to")),
            ));
        }
    }
}

fn check_hint(value: &str, field: &str, prefix: &str, diagnostics: &mut Diagnostics) {
    if value.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "E0003",
            format!("hint `{field}` must not be empty"),
            Some(&format!("{prefix}.hints.{field}")),
        ));
    }
}

fn check_tasks(
    tasks: &[Task],
    allowed_holes: &BTreeSet<&str>,
    truth_model: &Model,
    prefix: &str,
    diagnostics: &mut Diagnostics,
) {
    for task in tasks {
        if let Some(hole) = &task.hole
            && !allowed_holes.contains(hole.as_str())
        {
            diagnostics.push(Diagnostic::error(
                "E2101",
                format!("task `{}` references unknown hole `{hole}`", task.id),
                Some(&format!("{prefix}.{}.hole", task.id)),
            ));
        }
        for hole in &task.holes {
            if !allowed_holes.contains(hole.as_str()) {
                diagnostics.push(Diagnostic::error(
                    "E2101",
                    format!("task `{}` references unknown hole `{hole}`", task.id),
                    Some(&format!("{prefix}.{}.holes", task.id)),
                ));
            }
        }
        for control in &task.controls {
            if !truth_model.parameters.contains_key(control) {
                diagnostics.push(Diagnostic::error(
                    "E2101",
                    format!("task `{}` references unknown control `{control}`", task.id),
                    Some(&format!("{prefix}.{}.controls", task.id)),
                ));
            }
        }
    }
}

fn check_misconceptions(
    misconceptions: &[Misconception],
    truth_model: &Model,
    prefix: &str,
    causal_holes: &BTreeSet<&str>,
    task_ids: &BTreeSet<&str>,
    diagnostics: &mut Diagnostics,
) {
    for misconception in misconceptions {
        check_source(misconception, prefix, diagnostics);
        for task_id in &misconception.must_fail {
            if !task_ids.contains(task_id.as_str()) {
                diagnostics.push(Diagnostic::error(
                    "E2101",
                    format!(
                        "misconception `{}` must_fail references unknown task `{task_id}`",
                        misconception.id
                    ),
                    Some(&format!("{prefix}.{}.must_fail", misconception.id)),
                ));
            }
        }
        check_patch(
            misconception,
            truth_model,
            prefix,
            causal_holes,
            diagnostics,
        );
    }
}

fn check_source(misconception: &Misconception, prefix: &str, diagnostics: &mut Diagnostics) {
    let source = &misconception.source;
    let path = format!("{prefix}.{}.source", misconception.id);
    if source.is_hypothesized() {
        diagnostics.push(
            Diagnostic::warning(
                "W5201",
                format!(
                    "misconception `{}` is marked hypothesized; cite a source before expert_reviewed",
                    misconception.id
                ),
                Some(&format!("{path}.kind")),
            )
            .with_help(
                "replace kind with research_documented, exam_item, historical_theory or \
                 folk_belief and add ref: \"src:<id>\" once research lands",
            ),
        );
    } else if source.reference.is_none() {
        diagnostics.push(Diagnostic::error(
            "E5204",
            format!(
                "misconception `{}` needs source.ref unless kind is hypothesized",
                misconception.id
            ),
            Some(&format!("{path}.ref")),
        ));
        return;
    }
    if let Some(reference) = &source.reference
        && !valid_source_ref(reference)
    {
        diagnostics.push(Diagnostic::error(
            "E5204",
            format!(
                "misconception `{}` source.ref `{reference}` is not `src:<id>`",
                misconception.id
            ),
            Some(&format!("{path}.ref")),
        ));
    }
}

fn valid_source_ref(reference: &str) -> bool {
    reference
        .strip_prefix("src:")
        .is_some_and(|id| !id.is_empty() && !id.chars().any(char::is_whitespace))
}

fn check_patch(
    misconception: &Misconception,
    truth_model: &Model,
    prefix: &str,
    causal_holes: &BTreeSet<&str>,
    diagnostics: &mut Diagnostics,
) {
    let mut model = truth_model.clone();
    let mut applied = false;
    let mut all_applied = true;
    for (path, value) in &misconception.patch {
        if causal_holes.contains(path.as_str()) {
            continue;
        }
        match set_model_path(&mut model, path, value) {
            Ok(()) => applied = true,
            Err(PathError::NotFound) => {
                all_applied = false;
                diagnostics.push(
                    Diagnostic::error(
                        "E5203",
                        format!(
                            "misconception `{}` patches `{path}`, which does not exist in the model",
                            misconception.id
                        ),
                        Some(&format!("{prefix}.{}.patch.{path}", misconception.id)),
                    )
                    .with_help(
                        "L1 causal patches may only name a causal_link hole id declared in \
                         this layer or the base player model",
                    ),
                );
            }
            Err(PathError::BadValue) => {
                all_applied = false;
                diagnostics.push(Diagnostic::error(
                    "E0003",
                    format!(
                        "misconception `{}` value `{value}` for `{path}` is not a number",
                        misconception.id
                    ),
                    Some(&format!("{prefix}.{}.patch.{path}", misconception.id)),
                ));
            }
        }
    }
    if !(all_applied && applied) {
        return;
    }
    if let Err(errors) = compile_model(&model) {
        let path_prefix = format!("{prefix}.{}.patch.", misconception.id);
        for mut diagnostic in errors {
            if let Some(path) = diagnostic.path.take() {
                diagnostic.path = Some(format!("{path_prefix}{path}"));
            }
            diagnostics.push(diagnostic);
        }
    }
}
