//! Model edit patches.

use serde::Deserialize;

use crate::CompiledModel;

#[derive(Deserialize)]
#[serde(tag = "op")]
enum Patch {
    #[serde(rename = "set_eq")]
    Equation {
        section: EqSection,
        name: String,
        eq: String,
    },
    #[serde(rename = "set_initial")]
    Initial { name: String, expr: String },
    #[serde(rename = "set_value")]
    Value { name: String, value: f64 },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum EqSection {
    Flows,
    Auxiliaries,
    Observables,
}

/// Applies an edit patch and returns the serialised model.
pub(crate) fn edit_model(compiled: &CompiledModel, patch_json: &str) -> Result<String, String> {
    let patch: Patch =
        serde_json::from_str(patch_json).map_err(|error| format!("invalid patch JSON: {error}"))?;
    let mut model = compiled.model.clone();
    match patch {
        Patch::Equation { section, name, eq } => match section {
            EqSection::Flows => {
                let flow = model
                    .flows
                    .get_mut(&name)
                    .ok_or_else(|| format!("unknown flow `{name}`"))?;
                flow.eq = eq;
            }
            EqSection::Auxiliaries => {
                let auxiliary = model
                    .auxiliaries
                    .get_mut(&name)
                    .ok_or_else(|| format!("unknown auxiliary `{name}`"))?;
                auxiliary.eq = eq;
            }
            EqSection::Observables => {
                let observable = model
                    .observables
                    .get_mut(&name)
                    .ok_or_else(|| format!("unknown observable `{name}`"))?;
                observable.eq = eq;
            }
        },
        Patch::Initial { name, expr } => {
            let stock = model
                .stocks
                .get_mut(&name)
                .ok_or_else(|| format!("unknown stock `{name}`"))?;
            stock.initial = expr;
        }
        Patch::Value { name, value } => {
            let parameter = model
                .parameters
                .get_mut(&name)
                .ok_or_else(|| format!("unknown parameter `{name}`"))?;
            parameter.value = value;
        }
    }
    model.to_yaml().map_err(|error| error.to_string())
}
