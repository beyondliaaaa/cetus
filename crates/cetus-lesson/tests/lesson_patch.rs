//! Model path and patch tests.

use cetus_compile::compile_model;
use cetus_lesson::{
    Lesson, ParameterOverride, PathError, apply_patch, apply_truth_overrides, set_model_path,
};
use cetus_model::Model;
use indexmap::IndexMap;

fn solar() -> Model {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/modules/energy/solar-battery/module.yaml"
    );
    let source = std::fs::read_to_string(path).expect("read solar module");
    Model::from_yaml(&source).expect("parse solar module")
}

fn positive_lesson() -> Lesson {
    let path = format!(
        "{}/tests/fixtures/positive.lesson.yaml",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(path).expect("read positive lesson");
    Lesson::from_yaml(&source).expect("parse positive lesson")
}

#[test]
fn given_parameter_path_when_set_then_the_value_changes() {
    let mut model = solar();
    set_model_path(&mut model, "parameters.efficiency.value", "0.183").expect("set parameter");
    assert!((model.parameters["efficiency"].value - 0.183).abs() < 1e-12);
}

#[test]
fn given_auxiliary_path_when_set_then_the_equation_changes() {
    let mut model = solar();
    let equation = "efficiency * area * irradiance";
    set_model_path(&mut model, "auxiliaries.pv_power.eq", equation).expect("set auxiliary");
    assert_eq!(model.auxiliaries["pv_power"].eq, equation);
}

#[test]
fn given_stock_path_when_set_then_the_initial_changes() {
    let mut model = solar();
    set_model_path(&mut model, "stocks.charge.initial", "0.25 * capacity").expect("set stock");
    assert_eq!(model.stocks["charge"].initial, "0.25 * capacity");
}

#[test]
fn given_flow_path_when_set_then_the_equation_changes() {
    let mut model = solar();
    set_model_path(&mut model, "flows.charging.eq", "0").expect("set flow");
    assert_eq!(model.flows["charging"].eq, "0");
}

#[test]
fn given_observable_path_when_set_then_the_equation_changes() {
    let mut model = solar();
    set_model_path(
        &mut model,
        "observables.charge_pct.eq",
        "1 - charge / capacity",
    )
    .expect("set observable");
    assert_eq!(model.observables["charge_pct"].eq, "1 - charge / capacity");
}

#[test]
fn given_unknown_path_when_set_then_not_found_is_returned() {
    let mut model = solar();
    let error =
        set_model_path(&mut model, "parameters.missing.value", "1").expect_err("unknown parameter");
    assert_eq!(error, PathError::NotFound);
    let error =
        set_model_path(&mut model, "parameters.efficiency.unit", "1").expect_err("unknown field");
    assert_eq!(error, PathError::NotFound);
    let error = set_model_path(&mut model, "efficiency", "1").expect_err("short path");
    assert_eq!(error, PathError::NotFound);
}

#[test]
fn given_non_numeric_parameter_value_when_set_then_bad_value_is_returned() {
    let mut model = solar();
    let error = set_model_path(&mut model, "parameters.efficiency.value", "not-a-number")
        .expect_err("non-numeric value");
    assert_eq!(error, PathError::BadValue);
}

#[test]
fn given_valid_patch_when_applied_then_the_patched_model_still_compiles() {
    let model = solar();
    let mut patch = IndexMap::new();
    patch.insert("parameters.efficiency.value".to_owned(), "0.183".to_owned());
    patch.insert(
        "auxiliaries.pv_power.eq".to_owned(),
        "efficiency * area * irradiance".to_owned(),
    );
    let patched = apply_patch(&model, &patch).expect("apply patch");
    assert!((patched.parameters["efficiency"].value - 0.183).abs() < 1e-12);
    assert!(compile_model(&patched).is_ok());
}

#[test]
fn given_patch_with_missing_target_when_applied_then_e5203_is_reported() {
    let model = solar();
    let mut patch = IndexMap::new();
    patch.insert("parameters.missing.value".to_owned(), "1".to_owned());
    let diagnostics = apply_patch(&model, &patch).expect_err("missing target");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E5203"
            && diagnostic.path.as_deref() == Some("patch.parameters.missing.value")
    }));
}

#[test]
fn given_truth_overrides_when_applied_then_efficiency_is_patched_and_unknowns_report_e2101() {
    let mut lesson = positive_lesson();
    let (model, diagnostics) = apply_truth_overrides(&lesson, &solar());
    assert_eq!(diagnostics, []);
    assert!((model.parameters["efficiency"].value - 0.183).abs() < 1e-12);

    let overrides = lesson.truth.overrides.as_mut().expect("overrides");
    overrides.parameters.insert(
        "does_not_exist".to_owned(),
        ParameterOverride { value: 1.0 },
    );
    let (_, diagnostics) = apply_truth_overrides(&lesson, &solar());
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E2101"
            && diagnostic.path.as_deref() == Some("truth.overrides.parameters.does_not_exist")
    }));
}
