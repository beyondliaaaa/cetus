//! V1 static checks for lessons.
//!
//! The checks validate a lesson against the module it references without
//! running anything: instruments, hole targets, task references, model
//! misconception patches and teaching-task references.

mod holes;
mod teaching;

use std::collections::BTreeSet;

use cetus_compile::{CompiledPlan, Diagnostic, Diagnostics, compile_model_with_warnings};
use cetus_model::Model;

use crate::patch::apply_truth_overrides;
use crate::schema::{Layer, LayerId, Lesson};

/// Runs every V1 static check for `lesson` against `base_model`.
///
/// Codes emitted by this crate are `E2101`, `E5203`, `E5401` and `E0003`; any
/// other diagnostic comes from the compiler.
#[must_use]
pub fn check_static(lesson: &Lesson, base_model: &Model) -> Diagnostics {
    let (truth_model, mut diagnostics) = apply_truth_overrides(lesson, base_model);
    let (plan, compile_diagnostics) = compile_model_with_warnings(&truth_model);
    diagnostics.extend(compile_diagnostics);
    let Some(plan) = plan else {
        return diagnostics;
    };
    let names = model_names(&plan);
    check_instruments(lesson, &names, &mut diagnostics);
    holes::check_base(lesson, &truth_model, &names, &mut diagnostics);
    holes::check_layers(lesson, &truth_model, &names, &mut diagnostics);
    teaching::check(lesson, &truth_model, &mut diagnostics);
    diagnostics
}

fn check_instruments(lesson: &Lesson, names: &BTreeSet<String>, diagnostics: &mut Diagnostics) {
    for instrument in &lesson.instruments {
        if !names.contains(&instrument.measures) {
            diagnostics.push(Diagnostic::error(
                "E2101",
                format!(
                    "instrument `{}` measures `{}`, which is not in the model",
                    instrument.id, instrument.measures
                ),
                Some(&format!("instruments.{}.measures", instrument.id)),
            ));
        }
    }
}

fn model_names(plan: &CompiledPlan) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = plan
        .layout
        .slots
        .iter()
        .map(|slot| slot.name.clone())
        .collect();
    names.extend(plan.flows.iter().map(|flow| flow.name.clone()));
    names.extend(plan.flows.iter().map(|flow| flow.name.clone()));
    names.extend(
        plan.observables
            .iter()
            .map(|observable| observable.name.clone()),
    );
    names
}

pub(super) fn layers(lesson: &Lesson) -> impl Iterator<Item = (LayerId, &Layer)> {
    let store = lesson.layers.as_ref();
    [
        (LayerId::L0, store.and_then(|store| store.l0.as_ref())),
        (LayerId::L1, store.and_then(|store| store.l1.as_ref())),
        (LayerId::L2, store.and_then(|store| store.l2.as_ref())),
        (LayerId::L3, store.and_then(|store| store.l3.as_ref())),
    ]
    .into_iter()
    .filter_map(|(id, layer)| layer.map(|layer| (id, layer)))
}
