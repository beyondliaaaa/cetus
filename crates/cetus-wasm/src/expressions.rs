//! Formula-input expression checking.

use cetus_compile::{parse_in_plan, suggest_unit};
use cetus_expr::Type;
use cetus_units::UnitRegistry;
use serde::Serialize;

use crate::CompiledModel;
use crate::support::json;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ParseOkJson<'a> {
    ok: bool,
    dim: Option<String>,
    unit: Option<String>,
    #[serde(rename = "type")]
    ty: &'a str,
    matches_expected: Option<bool>,
    expected_unit: Option<String>,
}

#[derive(Serialize)]
struct ParseErrorJson<'a> {
    ok: bool,
    code: &'a str,
    message: &'a str,
    start: usize,
    end: usize,
}

/// Parses and type-checks an expression for the formula input box.
pub(crate) fn parse_expression(
    compiled: &CompiledModel,
    source: &str,
    expected_unit: Option<&str>,
) -> String {
    let plan = compiled.plan.as_ref();
    match parse_in_plan(source, plan) {
        Ok(typed) => {
            let is_bool = typed.ty.is_bool();
            let type_name = match &typed.ty {
                Type::Scalar(_) => "scalar",
                Type::Bool => "bool",
                _ => "other",
            };
            let dim = if is_bool {
                None
            } else {
                typed.ty.dim().map(ToString::to_string)
            };
            let unit = if is_bool {
                None
            } else {
                typed
                    .ty
                    .dim()
                    .and_then(|dim| suggest_unit(plan, dim))
                    .or_else(|| dim.clone())
            };
            let (matches_expected, expected_echo) = match expected_unit {
                Some(text) => {
                    let matches = if is_bool {
                        Some(false)
                    } else {
                        UnitRegistry::new()
                            .parse(text)
                            .ok()
                            .map(|unit| typed.ty.dim() == Some(unit.dim()))
                    };
                    (matches, Some(text.to_owned()))
                }
                None => (None, None),
            };
            json(
                &ParseOkJson {
                    ok: true,
                    dim,
                    unit,
                    ty: type_name,
                    matches_expected,
                    expected_unit: expected_echo,
                },
                "{\"ok\":false,\"code\":\"E1901\",\"message\":\"could not serialise the result\",\"start\":0,\"end\":0}",
            )
        }
        Err(error) => {
            let span = error.span();
            json(
                &ParseErrorJson {
                    ok: false,
                    code: error.code(),
                    message: error.message(),
                    start: span.start,
                    end: span.end,
                },
                "{\"ok\":false,\"code\":\"E0003\",\"message\":\"could not serialise the error\",\"start\":0,\"end\":0}",
            )
        }
    }
}
