//! wasm-bindgen bindings for the browser client.
//!
//! The surface is deliberately narrow and coarse-grained (§5.13). M2 exposes
//! model compilation, a stepping simulation, expression checking and model
//! editing; the determinism witnesses from M0/M1 are unchanged.

#![forbid(unsafe_code)]

mod edit;
mod expressions;
mod meta;
mod model;
mod sim;
mod support;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use cetus_compile::CompiledPlan;
use cetus_kernel::Sim;
use cetus_model::Model;
use wasm_bindgen::prelude::*;

/// A compiled model ready to start simulations.
#[wasm_bindgen(js_name = CompiledModel)]
#[derive(Debug)]
pub struct CompiledModel {
    pub(crate) model: Model,
    pub(crate) plan: Arc<CompiledPlan>,
}

/// A running simulation.
#[wasm_bindgen(js_name = Sim)]
#[derive(Debug)]
pub struct SimHandle {
    pub(crate) sim: Sim,
    pub(crate) horizon_tick: Option<u64>,
}

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
    let plan = Arc::new(cetus_compile::compile_model(&model).map_err(|diagnostics| {
        let message = diagnostics
            .iter()
            .map(cetus_compile::Diagnostic::to_line)
            .collect::<Vec<_>>()
            .join("\n");
        JsValue::from_str(&message)
    })?);
    let subsystem = cetus_sd::SdSubsystem::new(plan.clone())
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

/// Compiles a model document.
///
/// # Errors
/// Returns a JSON string `{"diagnostics":[...]}` when the YAML cannot be
/// parsed or the model fails static checks.
#[wasm_bindgen(js_name = compileModel)]
pub fn compile_model(yaml: &str) -> Result<CompiledModel, JsValue> {
    model::compile(yaml).map_err(|message| JsValue::from_str(&message))
}

#[wasm_bindgen]
impl CompiledModel {
    /// Model metadata and display-unit conversions as JSON.
    #[wasm_bindgen(js_name = metaJson)]
    pub fn meta_json(&self) -> String {
        meta::meta_json(self)
    }

    /// Lowercase BLAKE3 hex of the normalised model content.
    #[wasm_bindgen(js_name = contentHash)]
    pub fn content_hash(&self) -> String {
        meta::content_hash(self)
    }

    /// Starts a fresh simulation at the plan's start tick.
    ///
    /// # Errors
    /// Returns a readable message when the subsystem or kernel rejects the
    /// model.
    #[wasm_bindgen(js_name = start)]
    pub fn start(&self, seed: u32) -> Result<SimHandle, JsValue> {
        model::start(self, seed).map_err(|message| JsValue::from_str(&message))
    }

    /// Restores a simulation from a snapshot.
    ///
    /// # Errors
    /// Returns a readable message when the snapshot is corrupt or belongs to a
    /// different model.
    #[wasm_bindgen(js_name = restore)]
    pub fn restore(&self, bytes: &[u8]) -> Result<SimHandle, JsValue> {
        model::restore(self, bytes).map_err(|message| JsValue::from_str(&message))
    }

    /// Parses and type-checks an expression for the formula input box.
    // wasm-bindgen's optional-string ABI needs an owned `Option<String>` here.
    #[allow(clippy::needless_pass_by_value)]
    #[wasm_bindgen(js_name = parseExpression)]
    pub fn parse_expression(&self, source: &str, expected_unit: Option<String>) -> String {
        expressions::parse_expression(self, source, expected_unit.as_deref())
    }

    /// Applies an edit patch and returns the new model YAML.
    ///
    /// # Errors
    /// Returns a readable message for invalid patches or unknown names.
    #[wasm_bindgen(js_name = editModel)]
    pub fn edit_model(&self, patch_json: &str) -> Result<String, JsValue> {
        edit::edit_model(self, patch_json).map_err(|message| JsValue::from_str(&message))
    }
}

#[wasm_bindgen(js_class = Sim)]
impl SimHandle {
    /// Current tick, time, completion flag, state hash and observables.
    #[wasm_bindgen(js_name = statusJson)]
    pub fn status_json(&self) -> String {
        sim::status_json(self)
    }

    /// Advances at most `n` macro steps, never past the horizon.
    ///
    /// # Errors
    /// Returns a readable message when a kernel step fails.
    #[wasm_bindgen(js_name = step)]
    pub fn step(&mut self, n: u32) -> Result<String, JsValue> {
        sim::step(self, n).map_err(|message| JsValue::from_str(&message))
    }

    /// Advances to `display_time`, clamped to `[current, horizon]`.
    ///
    /// # Errors
    /// Returns a readable message for non-finite or negative input, or when a
    /// kernel step fails.
    #[wasm_bindgen(js_name = runUntil)]
    pub fn run_until(&mut self, display_time: f64) -> Result<String, JsValue> {
        sim::run_until(self, display_time).map_err(|message| JsValue::from_str(&message))
    }

    /// Sets a parameter, converting from the declared or requested unit to SI.
    ///
    /// # Errors
    /// Returns a readable message for unknown names, incompatible or affine
    /// units, and values outside the declared range.
    // wasm-bindgen's optional-string ABI needs an owned `Option<String>` here.
    #[allow(clippy::needless_pass_by_value)]
    #[wasm_bindgen(js_name = setParam)]
    pub fn set_param(
        &mut self,
        name: &str,
        value: f64,
        unit: Option<String>,
    ) -> Result<String, JsValue> {
        sim::set_param(self, name, value, unit.as_deref())
            .map_err(|message| JsValue::from_str(&message))
    }

    /// Registers an observable probe.
    ///
    /// # Errors
    /// Returns a readable message for an unknown observable.
    #[wasm_bindgen(js_name = addProbe)]
    pub fn add_probe(&mut self, name: &str) -> Result<(), JsValue> {
        sim::add_probe(self, name).map_err(|message| JsValue::from_str(&message))
    }

    /// Reads interleaved `[tick, value, ...]` pairs recorded after `since_tick`.
    ///
    /// # Errors
    /// Returns a readable message for an unknown probe or invalid `since_tick`.
    #[wasm_bindgen(js_name = readProbe)]
    pub fn read_probe(&self, name: &str, since_tick: f64) -> Result<Vec<f64>, JsValue> {
        sim::read_probe(self, name, since_tick).map_err(|message| JsValue::from_str(&message))
    }

    /// Evaluates an expression against the current state.
    #[wasm_bindgen(js_name = query)]
    pub fn query(&self, expr: &str) -> String {
        sim::query(self, expr)
    }

    /// Encodes a snapshot of the current simulation.
    ///
    /// # Errors
    /// Returns a readable message when encoding fails.
    #[wasm_bindgen(js_name = snapshot)]
    pub fn snapshot(&self) -> Result<Vec<u8>, JsValue> {
        sim::snapshot(self).map_err(|message| JsValue::from_str(&message))
    }

    /// Canonical state hash in lowercase hex.
    #[wasm_bindgen(js_name = stateHash)]
    pub fn state_hash(&self) -> String {
        sim::state_hash(self)
    }

    /// Current tick as a JS-safe number.
    #[wasm_bindgen(js_name = tick)]
    pub fn tick(&self) -> f64 {
        sim::tick(self)
    }

    /// Current time in the model's display unit.
    #[wasm_bindgen(js_name = displayTime)]
    pub fn display_time(&self) -> f64 {
        sim::display_time(self)
    }
}
