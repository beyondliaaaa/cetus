//! Simulation operations behind [`crate::SimHandle`].

use cetus_compile::{parse_in_plan, suggest_unit};
use cetus_expr::compile;
use cetus_kernel::eval_expr;
use cetus_units::UnitRegistry;
use serde::ser::SerializeMap as _;
use serde::{Serialize, Serializer};

use crate::SimHandle;
use crate::support::json;

struct OrderedValues(Vec<(String, f64)>);

impl Serialize for OrderedValues {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusJson {
    tick: f64,
    time: f64,
    display_time: f64,
    done: bool,
    hash: String,
    observables: OrderedValues,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryOk {
    ok: bool,
    si_value: f64,
    value: f64,
    unit: String,
    dim: String,
}

#[derive(Serialize)]
struct QueryError<'a> {
    ok: bool,
    code: &'a str,
    message: &'a str,
    start: usize,
    end: usize,
}

/// Renders the current simulation status.
pub(crate) fn status_json(handle: &SimHandle) -> String {
    let sim = &handle.sim;
    let plan = sim.plan();
    let mut observables = Vec::with_capacity(plan.observables.len());
    for observable in &plan.observables {
        let value = sim.observable(&observable.name).unwrap_or(0.0);
        observables.push((observable.name.clone(), value));
    }
    let done = handle
        .horizon_tick
        .is_some_and(|horizon| sim.tick() >= horizon);
    let status = StatusJson {
        tick: sim.tick() as f64,
        time: sim.time(),
        display_time: sim.display_time(),
        done,
        hash: sim.state_hash_hex(),
        observables: OrderedValues(observables),
    };
    json(&status, "{}")
}

/// Advances at most `n` macro steps, never past the horizon.
pub(crate) fn step(handle: &mut SimHandle, n: u32) -> Result<String, String> {
    let target = handle.sim.tick().saturating_add(u64::from(n));
    let target = handle
        .horizon_tick
        .map_or(target, |horizon| target.min(horizon));
    handle
        .sim
        .run_until(target)
        .map_err(|error| error.to_string())?;
    Ok(status_json(handle))
}

/// Advances to the tick of `display_time`, clamped to `[current, horizon]`.
pub(crate) fn run_until(handle: &mut SimHandle, display_time: f64) -> Result<String, String> {
    if !display_time.is_finite() || display_time < 0.0 {
        return Err("display time must be a finite, non-negative number".to_owned());
    }
    let plan = handle.sim.plan();
    let target = plan.time.tick_of(display_time * plan.time.unit_factor);
    let target = handle
        .horizon_tick
        .map_or(target, |horizon| target.min(horizon));
    handle
        .sim
        .run_until(target)
        .map_err(|error| error.to_string())?;
    Ok(status_json(handle))
}

/// Sets a parameter, converting the value from the requested unit to SI.
pub(crate) fn set_param(
    handle: &mut SimHandle,
    name: &str,
    value: f64,
    unit: Option<&str>,
) -> Result<String, String> {
    if !value.is_finite() {
        return Err(format!("parameter `{name}` value must be finite"));
    }
    let plan = handle.sim.plan();
    let parameter = plan
        .parameters
        .iter()
        .find(|parameter| parameter.name.as_str() == name)
        .ok_or_else(|| format!("unknown parameter `{name}`"))?;
    let declared = plan
        .layout
        .slots
        .get(parameter.slot)
        .ok_or_else(|| format!("parameter `{name}` has no layout slot"))?
        .unit
        .clone();
    if declared.is_affine() {
        return Err(format!(
            "parameter `{name}` cannot be set through the affine unit `{}`",
            declared.symbol()
        ));
    }
    let si_value = match unit {
        None => declared.to_si(value),
        Some(text) => {
            let requested = UnitRegistry::new()
                .parse(text)
                .map_err(|error| format!("invalid unit `{text}`: {error}"))?;
            if requested.is_affine() {
                return Err(format!(
                    "unit `{text}` is affine; set an absolute value instead"
                ));
            }
            if requested.dim() != declared.dim() {
                return Err(format!(
                    "unit `{text}` is {}, but parameter `{name}` declares {}",
                    requested.dim(),
                    declared.dim()
                ));
            }
            requested.to_si(value)
        }
    };
    handle
        .sim
        .set_param(name, si_value)
        .map_err(|error| error.to_string())?;
    Ok(status_json(handle))
}

/// Registers an observable probe.
pub(crate) fn add_probe(handle: &mut SimHandle, name: &str) -> Result<(), String> {
    handle
        .sim
        .add_probe(name)
        .map_err(|error| error.to_string())
}

/// Reads `[tick, value, ...]` pairs recorded after `since_tick`.
pub(crate) fn read_probe(
    handle: &SimHandle,
    name: &str,
    since_tick: f64,
) -> Result<Vec<f64>, String> {
    if !since_tick.is_finite() || since_tick < 0.0 {
        return Err("since_tick must be a finite, non-negative number".to_owned());
    }
    let probe = handle
        .sim
        .probes()
        .iter()
        .find(|probe| probe.name.as_str() == name)
        .ok_or_else(|| format!("unknown probe `{name}`"))?;
    let mut out = Vec::with_capacity(probe.ticks.len() * 2);
    for (tick, value) in probe.ticks.iter().zip(&probe.values) {
        if *tick as f64 > since_tick {
            out.push(*tick as f64);
            out.push(*value);
        }
    }
    Ok(out)
}

/// Evaluates an ad-hoc expression against the current state.
pub(crate) fn query(handle: &SimHandle, source: &str) -> String {
    let sim = &handle.sim;
    let plan = sim.plan();
    match parse_in_plan(source, plan) {
        Ok(typed) => {
            if !typed.ty.is_m1_supported() || !typed.ty.is_numeric() {
                return query_error(
                    "E1901",
                    &format!("type {} is not supported by query", typed.ty.describe()),
                    0,
                    source.len(),
                );
            }
            let compiled = compile(&typed);
            let si_value = eval_expr(&compiled, sim.state(), sim.time(), &plan.tables);
            let dim = typed
                .ty
                .dim()
                .map_or_else(|| "1".to_owned(), ToString::to_string);
            let (unit, value) = match typed.ty.dim().and_then(|dim| suggest_unit(plan, dim)) {
                Some(symbol) => {
                    let factor = UnitRegistry::new()
                        .parse(&symbol)
                        .map_or(1.0, |unit| unit.factor());
                    (symbol, si_value / factor)
                }
                None => (dim.clone(), si_value),
            };
            json(
                &QueryOk {
                    ok: true,
                    si_value,
                    value,
                    unit,
                    dim,
                },
                "{\"ok\":false,\"code\":\"E1901\",\"message\":\"could not serialise the result\",\"start\":0,\"end\":0}",
            )
        }
        Err(error) => {
            let span = error.span();
            query_error(error.code(), error.message(), span.start, span.end)
        }
    }
}

fn query_error(code: &str, message: &str, start: usize, end: usize) -> String {
    json(
        &QueryError {
            ok: false,
            code,
            message,
            start,
            end,
        },
        "{\"ok\":false,\"code\":\"E0003\",\"message\":\"could not serialise the error\",\"start\":0,\"end\":0}",
    )
}

/// Encodes a snapshot of the current simulation.
pub(crate) fn snapshot(handle: &SimHandle) -> Result<Vec<u8>, String> {
    handle.sim.snapshot().map_err(|error| error.to_string())
}

/// Canonical state hash in lowercase hex.
pub(crate) fn state_hash(handle: &SimHandle) -> String {
    handle.sim.state_hash_hex()
}

/// Current tick as a JS-safe number.
pub(crate) fn tick(handle: &SimHandle) -> f64 {
    handle.sim.tick() as f64
}

/// Current time in the model's display unit.
pub(crate) fn display_time(handle: &SimHandle) -> f64 {
    handle.sim.display_time()
}
