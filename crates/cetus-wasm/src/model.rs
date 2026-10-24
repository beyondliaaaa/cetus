//! Compilation and simulation lifecycle behind [`crate::CompiledModel`].

use std::sync::Arc;

use cetus_compile::{CompiledPlan, Diagnostic, compile_model};
use cetus_kernel::Sim;
use cetus_model::{Model, ViewKind};
use cetus_sd::SdSubsystem;

use crate::support::diagnostics_json;
use crate::{CompiledModel, SimHandle};

pub(crate) fn compile(yaml: &str) -> Result<CompiledModel, String> {
    let model = Model::from_yaml(yaml).map_err(|error| {
        diagnostics_json(&[Diagnostic::error("E0003", error.to_string(), None)])
    })?;
    let plan = compile_model(&model).map_err(|diagnostics| diagnostics_json(&diagnostics))?;
    Ok(CompiledModel {
        model,
        plan: Arc::new(plan),
    })
}

pub(crate) fn start(compiled: &CompiledModel, seed: u32) -> Result<SimHandle, String> {
    let plan = compiled.plan.clone();
    let subsystem = SdSubsystem::new(plan.clone()).with_integrator(compiled.model.time.integrator);
    let subsystem = subsystem.map_err(|error| error.to_string())?;
    let sim = Sim::new(plan.clone(), u64::from(seed), Box::new(subsystem))
        .map_err(|error| error.to_string())?;
    Ok(SimHandle {
        sim,
        horizon_tick: horizon_tick(&compiled.model, &plan),
    })
}

/// Restores a simulation from a snapshot produced by the same plan.
pub(crate) fn restore(compiled: &CompiledModel, bytes: &[u8]) -> Result<SimHandle, String> {
    let plan = compiled.plan.clone();
    let subsystem = SdSubsystem::new(plan.clone()).with_integrator(compiled.model.time.integrator);
    let subsystem = subsystem.map_err(|error| error.to_string())?;
    let sim = Sim::restore(plan.clone(), Box::new(subsystem), bytes)
        .map_err(|error| error.to_string())?;
    Ok(SimHandle {
        sim,
        horizon_tick: horizon_tick(&compiled.model, &plan),
    })
}

fn horizon_tick(model: &Model, plan: &CompiledPlan) -> Option<u64> {
    model
        .time
        .horizon
        .map(|horizon| plan.time.tick_of(horizon * plan.time.unit_factor))
}

pub(crate) fn views_of(plan: &CompiledPlan) -> Vec<ViewKind> {
    let mut views = Vec::new();
    for paradigm in &plan.paradigms {
        for view in paradigm.views() {
            if !views.contains(view) {
                views.push(*view);
            }
        }
    }
    views
}
