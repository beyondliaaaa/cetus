//! Model metadata serialisation.

use cetus_model::{Integrator, InvariantKind, ParadigmId, ParameterRole, Tier, TimeKind, ViewKind};
use cetus_units::UnitRegistry;
use serde::Serialize;

use crate::CompiledModel;
use crate::support::{hex, json};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetaJson<'a> {
    id: &'a str,
    version: &'a str,
    title: serde_json::Value,
    tier: Tier,
    paradigms: &'a [ParadigmId],
    views: Vec<ViewKind>,
    content_hash: String,
    time: TimeJson,
    parameters: Vec<ParameterJson>,
    stocks: Vec<StockJson>,
    auxiliaries: Vec<AuxiliaryJson>,
    flows: Vec<FlowJson>,
    observables: Vec<ObservableJson>,
    invariants: Vec<InvariantJson>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeJson {
    unit: String,
    unit_factor: f64,
    dt: f64,
    horizon: Option<f64>,
    integrator: Integrator,
    kind: TimeKind,
    start: f64,
    start_tick: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ParameterJson {
    name: String,
    unit: String,
    unit_factor: f64,
    offset: f64,
    value: f64,
    si_value: f64,
    range: Option<[f64; 2]>,
    si_range: Option<[f64; 2]>,
    role: ParameterRole,
    source: Option<String>,
    dim: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StockJson {
    name: String,
    unit: String,
    unit_factor: f64,
    initial: f64,
    si_initial: f64,
    bounds: BoundsJson,
    dim: String,
}

#[derive(Serialize)]
struct BoundsJson {
    min: Option<f64>,
    max: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuxiliaryJson {
    name: String,
    unit: String,
    unit_factor: f64,
    eq: String,
    dim: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FlowJson {
    name: String,
    from: Option<String>,
    to: Option<String>,
    unit: String,
    unit_factor: f64,
    eq: String,
    dim: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ObservableJson {
    name: String,
    unit: String,
    unit_factor: f64,
    display: Option<String>,
    dim: String,
}

#[derive(Serialize)]
struct InvariantJson {
    id: String,
    kind: InvariantKind,
}

/// Renders the model metadata document.
pub(crate) fn meta_json(compiled: &CompiledModel) -> String {
    let model = &compiled.model;
    let plan = compiled.plan.as_ref();
    let registry = UnitRegistry::new();

    let time = TimeJson {
        unit: model.time.unit.clone(),
        unit_factor: plan.time.unit_factor,
        dt: plan.time.quantum / plan.time.unit_factor,
        horizon: model.time.horizon,
        integrator: model.time.integrator,
        kind: model.time.kind,
        start: model.time.start.unwrap_or(0.0),
        start_tick: plan.time.start_tick as f64,
    };

    let mut parameters = Vec::with_capacity(model.parameters.len());
    for (name, spec) in &model.parameters {
        let Some(parameter) = plan
            .parameters
            .iter()
            .find(|parameter| parameter.name.as_str() == name.as_str())
        else {
            continue;
        };
        let Some(info) = plan
            .layout
            .slot_of(name)
            .and_then(|slot| plan.layout.slots.get(slot))
        else {
            continue;
        };
        parameters.push(ParameterJson {
            name: name.clone(),
            unit: spec.unit.clone(),
            unit_factor: info.unit.factor(),
            offset: info.unit.offset(),
            value: info.unit.from_si(parameter.value),
            si_value: parameter.value,
            range: parameter
                .range
                .map(|(min, max)| [info.unit.from_si(min), info.unit.from_si(max)]),
            si_range: parameter.range.map(|(min, max)| [min, max]),
            role: parameter.role,
            source: spec.source.clone(),
            dim: info.dim.to_string(),
        });
    }

    let mut stocks = Vec::with_capacity(model.stocks.len());
    for (name, spec) in &model.stocks {
        let Some(stock) = plan
            .stocks
            .iter()
            .find(|stock| stock.name.as_str() == name.as_str())
        else {
            continue;
        };
        let Some(info) = plan.layout.slots.get(stock.slot) else {
            continue;
        };
        let (min, max) = stock.bounds.map_or((None, None), |(min, max)| {
            (
                min.map(|value| info.unit.from_si(value)),
                max.map(|value| info.unit.from_si(value)),
            )
        });
        stocks.push(StockJson {
            name: name.clone(),
            unit: spec.unit.clone(),
            unit_factor: info.unit.factor(),
            initial: info.unit.from_si(stock.initial),
            si_initial: stock.initial,
            bounds: BoundsJson { min, max },
            dim: info.dim.to_string(),
        });
    }

    let mut auxiliaries = Vec::with_capacity(model.auxiliaries.len());
    for (name, spec) in &model.auxiliaries {
        let Some(auxiliary) = plan
            .auxiliaries
            .iter()
            .find(|auxiliary| auxiliary.name.as_str() == name.as_str())
        else {
            continue;
        };
        let Some(info) = plan.layout.slots.get(auxiliary.slot) else {
            continue;
        };
        auxiliaries.push(AuxiliaryJson {
            name: name.clone(),
            unit: spec.unit.clone(),
            unit_factor: info.unit.factor(),
            eq: spec.eq.clone(),
            dim: info.dim.to_string(),
        });
    }

    let mut flows = Vec::with_capacity(model.flows.len());
    for (name, spec) in &model.flows {
        let (unit_factor, dim) = if let Ok(unit) = registry.parse(&spec.unit) {
            (unit.factor(), unit.dim().to_string())
        } else {
            let dim = plan
                .flows
                .iter()
                .find(|flow| flow.name.as_str() == name.as_str())
                .and_then(|flow| flow.rate.ty().dim())
                .map_or_else(|| "1".to_owned(), ToString::to_string);
            (1.0, dim)
        };
        flows.push(FlowJson {
            name: name.clone(),
            from: spec.from.clone(),
            to: spec.to.clone(),
            unit: spec.unit.clone(),
            unit_factor,
            eq: spec.eq.clone(),
            dim,
        });
    }

    let mut observables = Vec::with_capacity(model.observables.len());
    for (name, spec) in &model.observables {
        let Some(observable) = plan
            .observables
            .iter()
            .find(|observable| observable.name.as_str() == name.as_str())
        else {
            continue;
        };
        observables.push(ObservableJson {
            name: name.clone(),
            unit: spec.unit.clone(),
            unit_factor: observable.unit.factor(),
            display: observable.display.clone(),
            dim: observable.unit.dim().to_string(),
        });
    }

    let invariants = plan
        .invariants
        .iter()
        .map(|invariant| InvariantJson {
            id: invariant.id.clone(),
            kind: invariant.kind,
        })
        .collect();

    let meta = MetaJson {
        id: &plan.model_id,
        version: &plan.model_version,
        title: serde_json::to_value(&model.title).unwrap_or(serde_json::Value::Null),
        tier: model.tier,
        paradigms: &plan.paradigms,
        views: crate::model::views_of(plan),
        content_hash: hex(&plan.content_hash),
        time,
        parameters,
        stocks,
        auxiliaries,
        flows,
        observables,
        invariants,
    };
    json(&meta, "{}")
}

pub(crate) fn content_hash(compiled: &CompiledModel) -> String {
    hex(&compiled.plan.content_hash)
}
