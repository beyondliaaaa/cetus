//! wasm-bindgen bindings for the browser client.
//!
//! The surface is deliberately narrow and coarse-grained (§5.13). M1 exposes the
//! determinism witness and a one-shot model runner used by the cross-target CI
//! check; M2 replaces the runner with the full `compile`/`Sim` API.

#![forbid(unsafe_code)]

use std::sync::Arc;

use cetus_compile::compile_model;
use cetus_kernel::Sim;
use cetus_model::Model;
use cetus_sd::SdSubsystem;
use wasm_bindgen::prelude::*;

/// Hex digest of [`cetus_kernel::hello_digest`].
///
/// The native CLI prints the same value for `cetus hello`.
#[wasm_bindgen]
#[must_use]
pub fn hello_digest() -> String {
    cetus_kernel::hello_digest_hex()
}

/// Kernel crate version, useful for cache busting in the browser.
#[wasm_bindgen]
#[must_use]
pub fn kernel_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Compiles and runs a model to its declared horizon, returning the state hash.
///
/// `seed` is a `u32` because JS numbers cannot carry `u64` without `BigInt`; the
/// kernel widens it to `u64`.
///
/// # Errors
/// Returns a JS error string when the model fails to parse, compile or run.
#[wasm_bindgen]
pub fn run_model_digest(yaml: &str, seed: u32) -> Result<String, JsValue> {
    let model = Model::from_yaml(yaml).map_err(|error| JsValue::from_str(&error.to_string()))?;
    let horizon = model
        .time
        .horizon
        .ok_or_else(|| JsValue::from_str("model has no time.horizon"))?;
    let plan = Arc::new(compile_model(&model).map_err(|diagnostics| {
        let message = diagnostics
            .iter()
            .map(cetus_compile::Diagnostic::to_line)
            .collect::<Vec<_>>()
            .join("\n");
        JsValue::from_str(&message)
    })?);
    let subsystem = SdSubsystem::new(plan.clone())
        .with_integrator(model.time.integrator)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut sim = Sim::new(plan.clone(), u64::from(seed), Box::new(subsystem))
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let seconds = horizon * plan.time.unit_factor;
    let target = plan.time.tick_of(seconds);
    sim.run_until(target)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    Ok(sim.state_hash_hex())
}
