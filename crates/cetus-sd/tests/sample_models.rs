//! Reference solutions intentionally use `std` math so that they are independent
//! of the engine's `cetus-math` wrappers.
#![allow(clippy::disallowed_methods)]

//! M1 exit criteria: the eight sample models run and match their reference
//! solutions (§12 M1). Reference values come from closed-form solutions or
//! conservation laws, never from the simulator itself.

use std::path::PathBuf;
use std::sync::Arc;

use cetus_compile::{CompiledPlan, compile_model};
use cetus_kernel::Sim;
use cetus_model::Model;
use cetus_sd::SdSubsystem;

fn model_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/modules")
        .join(relative)
}

fn load(relative: &str) -> (Model, Arc<CompiledPlan>) {
    let path = model_path(relative);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let model = Model::from_yaml(&source).expect("model parses");
    let plan = Arc::new(compile_model(&model).expect("model compiles"));
    (model, plan)
}

fn run(relative: &str, probe: &str, seconds: f64) -> (Vec<f64>, Vec<f64>, Arc<CompiledPlan>) {
    let (model, plan) = load(relative);
    let subsystem = SdSubsystem::new(plan.clone())
        .with_integrator(model.time.integrator)
        .expect("integrator");
    let mut sim = Sim::new(plan.clone(), 1, Box::new(subsystem)).expect("sim");
    sim.add_probe(probe).expect("probe");
    let target = plan.time.tick_of(seconds);
    sim.run_until(target).expect("run");
    let probe_data = &sim.probes()[0];
    let times: Vec<f64> = probe_data
        .ticks
        .iter()
        .map(|tick| plan.time.time_of(*tick))
        .collect();
    (times, probe_data.values.clone(), plan)
}

fn nrmse(observed: &[f64], expected: &[f64], scale: f64) -> f64 {
    assert_eq!(observed.len(), expected.len());
    let mut sum = 0.0;
    for (left, right) in observed.iter().zip(expected) {
        let error = left - right;
        sum += error * error;
    }
    (sum / observed.len() as f64).sqrt() / scale
}

#[test]
fn decay_matches_the_analytic_solution() {
    let (times, values, _) = run("math/decay/module.yaml", "amount", 24.0 * 3600.0);
    let expected: Vec<f64> = times.iter().map(|t| (-0.1 * t / 3600.0).exp()).collect();
    let error = nrmse(&values, &expected, 1.0);
    println!("decay nrmse = {error:.3e}");
    assert!(error < 1e-4, "nrmse = {error}");
}

#[test]
fn logistic_matches_the_analytic_solution() {
    let (times, values, _) = run(
        "bio/population-logistic/module.yaml",
        "population",
        40.0 * 3600.0,
    );
    let expected: Vec<f64> = times
        .iter()
        .map(|t| {
            let hours = t / 3600.0;
            1000.0 / (1.0 + ((1000.0 - 10.0) / 10.0) * (-0.3 * hours).exp())
        })
        .collect();
    let error = nrmse(&values, &expected, 1000.0);
    println!("logistic nrmse = {error:.3e}");
    assert!(error < 1e-4, "nrmse = {error}");
}

#[test]
fn rc_matches_the_analytic_solution() {
    let (times, values, _) = run("phys/rc-circuit/module.yaml", "capacitor_voltage", 10.0);
    let expected: Vec<f64> = times.iter().map(|t| 5.0 * (1.0 - (-t).exp())).collect();
    let error = nrmse(&values, &expected, 5.0);
    println!("rc nrmse = {error:.3e}");
    assert!(error < 1e-4, "nrmse = {error}");
}

#[test]
fn sir_conserves_the_population() {
    let (_, values, plan) = run("bio/epidemic-sir/module.yaml", "total", 160.0 * 86_400.0);
    for value in &values {
        assert!((value - 1000.0).abs() < 1e-6, "total = {value}");
    }
    let _ = plan;
}

#[test]
fn sir_peak_is_smaller_than_the_population() {
    let (_, infectious, _) = run(
        "bio/epidemic-sir/module.yaml",
        "infectious",
        160.0 * 86_400.0,
    );
    let peak = infectious.iter().copied().fold(0.0_f64, f64::max);
    assert!(peak > 10.0, "epidemic never grew: {peak}");
    assert!(peak < 1000.0, "epidemic infected everyone: {peak}");
}

#[test]
fn predator_prey_conserves_the_lotka_volterra_invariant() {
    let (_, prey, _) = run(
        "bio/predator-prey/module.yaml",
        "prey",
        120.0 * 30.4375 * 86_400.0,
    );
    let (_, predator, _) = run(
        "bio/predator-prey/module.yaml",
        "predator",
        120.0 * 30.4375 * 86_400.0,
    );
    let invariant = |p: f64, q: f64| 0.005 * p - 0.5 * p.ln() + 0.01 * q - 1.0 * q.ln();
    let initial = invariant(prey[0], predator[0]);
    let final_value = invariant(*prey.last().unwrap(), *predator.last().unwrap());
    let drift = (final_value - initial).abs();
    println!("lotka-volterra invariant drift = {drift:.6}");
    assert!(drift < 0.05, "invariant drifted by {drift}");
}

#[test]
fn inventory_never_goes_negative() {
    let (_, values, _) = run(
        "eng/inventory-oscillation/module.yaml",
        "inventory",
        60.0 * 7.0 * 86_400.0,
    );
    for value in &values {
        assert!(*value >= 0.0, "negative inventory: {value}");
    }
}

#[test]
fn logistic_map_follows_the_recurrence() {
    let (_, values, _) = run("math/logistic-map/module.yaml", "x", 80.0);
    let mut x = 0.2_f64;
    for (index, value) in values.iter().enumerate() {
        x = 3.2 * x * (1.0 - x);
        assert!((value - x).abs() < 1e-9, "step {index}: {value} vs {x}");
    }
}

#[test]
fn solar_battery_keeps_the_lights_on_for_one_night() {
    let (_, values, _) = run(
        "energy/solar-battery/module.yaml",
        "charge_pct",
        72.0 * 3600.0,
    );
    for value in &values {
        assert!(
            *value >= 0.0 && *value <= 1.0,
            "charge out of bounds: {value}"
        );
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(min < 0.5, "battery never discharged: {min}");
}
