//! Applying model paths and patches.
//!
//! Only five path forms are meaningful to a lesson author:
//! `parameters.<name>.value`, `stocks.<name>.initial`, `auxiliaries.<name>.eq`,
//! `flows.<name>.eq` and `observables.<name>.eq`.

use cetus_compile::{Diagnostic, Diagnostics};
use cetus_model::Model;
use indexmap::IndexMap;

use crate::schema::Lesson;

/// Why a model path could not be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    /// The path names no supported model field.
    NotFound,
    /// The path exists but the value cannot be parsed for it.
    BadValue,
}

/// Sets one supported model path from its string form.
///
/// # Errors
/// Returns [`PathError::NotFound`] for an unsupported path or an unknown name,
/// and [`PathError::BadValue`] when a parameter value is not numeric.
pub fn set_model_path(model: &mut Model, path: &str, value: &str) -> Result<(), PathError> {
    let mut parts = path.split('.');
    let (Some(group), Some(name), Some(field), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(PathError::NotFound);
    };
    match (group, field) {
        ("parameters", "value") => {
            let parsed = value.parse::<f64>().map_err(|_| PathError::BadValue)?;
            let spec = model.parameters.get_mut(name).ok_or(PathError::NotFound)?;
            spec.value = parsed;
            Ok(())
        }
        ("stocks", "initial") => {
            let spec = model.stocks.get_mut(name).ok_or(PathError::NotFound)?;
            value.clone_into(&mut spec.initial);
            Ok(())
        }
        ("auxiliaries", "eq") => {
            let spec = model.auxiliaries.get_mut(name).ok_or(PathError::NotFound)?;
            value.clone_into(&mut spec.eq);
            Ok(())
        }
        ("flows", "eq") => {
            let spec = model.flows.get_mut(name).ok_or(PathError::NotFound)?;
            value.clone_into(&mut spec.eq);
            Ok(())
        }
        ("observables", "eq") => {
            let spec = model.observables.get_mut(name).ok_or(PathError::NotFound)?;
            value.clone_into(&mut spec.eq);
            Ok(())
        }
        _ => Err(PathError::NotFound),
    }
}

/// Applies an ordered map of model paths to a clone of `base`.
///
/// # Errors
/// Returns `E5203` for a target that does not exist and `E0003` for a
/// non-numeric parameter value.
pub fn apply_patch(base: &Model, patch: &IndexMap<String, String>) -> Result<Model, Diagnostics> {
    let mut model = base.clone();
    let mut diagnostics = Diagnostics::new();
    for (path, value) in patch {
        match set_model_path(&mut model, path, value) {
            Ok(()) => {}
            Err(PathError::NotFound) => diagnostics.push(Diagnostic::error(
                "E5203",
                format!("patch target `{path}` does not exist in the model"),
                Some(&format!("patch.{path}")),
            )),
            Err(PathError::BadValue) => diagnostics.push(Diagnostic::error(
                "E0003",
                format!("patch value `{value}` for `{path}` is not a number"),
                Some(&format!("patch.{path}")),
            )),
        }
    }
    if diagnostics.is_empty() {
        Ok(model)
    } else {
        Err(diagnostics)
    }
}

/// Applies `truth.overrides.parameters` to a clone of `base`.
///
/// Unknown parameters produce `E2101`; the model is returned either way.
#[must_use]
pub fn apply_truth_overrides(lesson: &Lesson, base: &Model) -> (Model, Diagnostics) {
    let mut model = base.clone();
    let mut diagnostics = Diagnostics::new();
    if let Some(overrides) = &lesson.truth.overrides {
        for (name, parameter) in &overrides.parameters {
            if let Some(spec) = model.parameters.get_mut(name) {
                spec.value = parameter.value;
            } else {
                diagnostics.push(Diagnostic::error(
                    "E2101",
                    format!("truth override parameter `{name}` is not declared by the module"),
                    Some(&format!("truth.overrides.parameters.{name}")),
                ));
            }
        }
    }
    (model, diagnostics)
}
