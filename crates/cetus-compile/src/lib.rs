//! Model → IR compiler with static checks and diagnostics (§5.4).
//!
//! Pipeline:
//!
//! ```text
//! Model ─resolve─▶ names ─units─▶ dimension checks ─structure─▶ IR
//! ```
//!
//! The IR is a flat, index-addressed plan. Runtime evaluation never looks up a
//! string (determinism + performance).

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use cetus_expr::{
    CompiledExpr, DomainVar, EquivOptions, EquivOutcome, Instr, NameResolver, Resolved,
    TableSignature, Type, compile, equivalent, parse_and_check,
};
use cetus_model::{InvariantKind, Model, ParadigmId, ParameterRole, TimeKind as ModelTimeKind};
use cetus_units::{BaseDim, Dim, Unit, UnitRegistry};

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The model cannot be compiled.
    Error,
    /// The model compiles but something is suspicious.
    Warning,
}

/// A compiler diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Diagnostic code (`E1203`, `W2001`, ...).
    pub code: String,
    /// Severity.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Model path such as `flows.charging.eq`.
    pub path: Option<String>,
    /// Byte span inside the offending expression.
    pub span: Option<(usize, usize)>,
    /// Suggested fix.
    pub help: Option<String>,
}

impl Diagnostic {
    /// Builds an error diagnostic.
    #[must_use]
    pub fn error(code: &str, message: impl Into<String>, path: Option<&str>) -> Self {
        Self {
            code: code.to_owned(),
            severity: Severity::Error,
            message: message.into(),
            path: path.map(ToOwned::to_owned),
            span: None,
            help: None,
        }
    }

    /// Builds a warning diagnostic.
    #[must_use]
    pub fn warning(code: &str, message: impl Into<String>, path: Option<&str>) -> Self {
        Self {
            code: code.to_owned(),
            severity: Severity::Warning,
            message: message.into(),
            path: path.map(ToOwned::to_owned),
            span: None,
            help: None,
        }
    }

    /// Attaches a byte span.
    #[must_use]
    pub fn with_span(mut self, span: (usize, usize)) -> Self {
        self.span = Some(span);
        self
    }

    /// Attaches a suggested fix.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Renders the diagnostic as a single line.
    #[must_use]
    pub fn to_line(&self) -> String {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let path = self.path.as_deref().unwrap_or("-");
        format!("{severity}[{}] {path}: {}", self.code, self.message)
    }
}

/// A list of diagnostics.
pub type Diagnostics = Vec<Diagnostic>;

/// Time base (K2).
#[derive(Debug, Clone)]
pub struct TimeBase {
    /// Seconds per tick.
    pub quantum: f64,
    /// Seconds per display unit.
    pub unit_factor: f64,
    /// Display unit symbol.
    pub unit_symbol: String,
    /// Time kind.
    pub kind: ModelTimeKind,
    /// First tick.
    pub start_tick: u64,
}

impl TimeBase {
    /// Absolute time in seconds at `tick`.
    #[must_use]
    pub fn time_of(&self, tick: u64) -> f64 {
        tick as f64 * self.quantum
    }

    /// Tick at or below `seconds`.
    #[must_use]
    pub fn tick_of(&self, seconds: f64) -> u64 {
        if self.quantum <= 0.0 {
            return 0;
        }
        let tick = (seconds / self.quantum).floor();
        if tick <= 0.0 { 0 } else { tick as u64 }
    }

    /// Converts seconds to the display unit.
    #[must_use]
    pub fn display(&self, seconds: f64) -> f64 {
        seconds / self.unit_factor
    }
}

/// What a state slot holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    /// Stock (integrated).
    Stock,
    /// Parameter (constant or externally driven).
    Parameter,
    /// Auxiliary (recomputed each step).
    Auxiliary,
}

/// One slot in the flat state vector.
#[derive(Debug, Clone)]
pub struct SlotInfo {
    /// Name as written in the model.
    pub name: String,
    /// Slot kind.
    pub kind: SlotKind,
    /// Unit.
    pub unit: Unit,
    /// Dimension (cached from `unit`).
    pub dim: Dim,
    /// Typical magnitude in SI units (K13).
    pub scale: f64,
}

/// State layout: stocks first, then parameters, then auxiliaries.
#[derive(Debug, Clone, Default)]
pub struct StateLayout {
    /// All slots in index order.
    pub slots: Vec<SlotInfo>,
    /// Number of stock slots (they occupy `0..stock_count`).
    pub stock_count: usize,
    /// Number of parameter slots.
    pub parameter_count: usize,
    /// Number of auxiliary slots.
    pub auxiliary_count: usize,
}

impl StateLayout {
    /// Total slot count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the layout is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Finds a slot by name.
    #[must_use]
    pub fn slot_of(&self, name: &str) -> Option<usize> {
        self.slots.iter().position(|slot| slot.name == name)
    }
}

/// A compiled parameter.
#[derive(Debug, Clone)]
pub struct CompiledParameter {
    /// Name.
    pub name: String,
    /// Slot index.
    pub slot: usize,
    /// Value in SI units.
    pub value: f64,
    /// Allowed range in SI units.
    pub range: Option<(f64, f64)>,
    /// Role.
    pub role: ParameterRole,
}

/// A compiled stock.
#[derive(Debug, Clone)]
pub struct CompiledStock {
    /// Name.
    pub name: String,
    /// Slot index.
    pub slot: usize,
    /// Initial value in SI units.
    pub initial: f64,
    /// Optional bounds in SI units.
    pub bounds: Option<(Option<f64>, Option<f64>)>,
    /// Flows that fill this stock.
    pub inflows: Vec<usize>,
    /// Flows that drain this stock.
    pub outflows: Vec<usize>,
}

/// A compiled auxiliary.
#[derive(Debug, Clone)]
pub struct CompiledAuxiliary {
    /// Name.
    pub name: String,
    /// Slot index.
    pub slot: usize,
    /// Defining expression.
    pub expr: CompiledExpr,
}

/// A compiled flow.
#[derive(Debug, Clone)]
pub struct CompiledFlow {
    /// Name.
    pub name: String,
    /// Rate expression (SI units per second).
    pub rate: CompiledExpr,
    /// Target stock slots and signs (`+1` fills, `-1` drains). A conversion
    /// flow (S -> I in an epidemic) has two targets with opposite signs.
    pub targets: Vec<(usize, i32)>,
}

/// A compiled observable.
#[derive(Debug, Clone)]
pub struct CompiledObservable {
    /// Name.
    pub name: String,
    /// Unit.
    pub unit: Unit,
    /// Expression in SI units.
    pub expr: CompiledExpr,
    /// Display hint.
    pub display: Option<String>,
}

/// A compiled invariant.
#[derive(Debug, Clone)]
pub struct CompiledInvariant {
    /// Identifier.
    pub id: String,
    /// Kind.
    pub kind: InvariantKind,
    /// Boolean expression (for `assert` / `bounds`).
    pub expr: Option<CompiledExpr>,
}

/// A compiled lookup table.
#[derive(Debug, Clone)]
pub struct CompiledTable {
    /// Name.
    pub name: String,
    /// X points in SI units.
    pub x: Vec<f64>,
    /// Y points in SI units.
    pub y: Vec<f64>,
}

impl CompiledTable {
    /// Piecewise-linear interpolation (clamped at the ends).
    #[must_use]
    pub fn lookup(&self, x: f64) -> f64 {
        if self.x.is_empty() {
            return 0.0;
        }
        if x <= self.x[0] {
            return self.y[0];
        }
        let last = self.x.len() - 1;
        if x >= self.x[last] {
            return self.y[last];
        }
        for index in 0..last {
            if x <= self.x[index + 1] {
                let x0 = self.x[index];
                let x1 = self.x[index + 1];
                if x1 <= x0 {
                    return self.y[index];
                }
                let t = (x - x0) / (x1 - x0);
                return self.y[index] + t * (self.y[index + 1] - self.y[index]);
            }
        }
        self.y[last]
    }
}

/// The compiled, index-addressed model plan.
#[derive(Debug, Clone)]
pub struct CompiledPlan {
    /// Module id.
    pub model_id: String,
    /// Module version.
    pub model_version: String,
    /// BLAKE3 hash of the normalised model YAML.
    pub content_hash: [u8; 32],
    /// Time base.
    pub time: TimeBase,
    /// State layout.
    pub layout: StateLayout,
    /// Parameters in declaration order.
    pub parameters: Vec<CompiledParameter>,
    /// Stocks in declaration order.
    pub stocks: Vec<CompiledStock>,
    /// Auxiliaries in evaluation order.
    pub auxiliaries: Vec<CompiledAuxiliary>,
    /// Flows in declaration order.
    pub flows: Vec<CompiledFlow>,
    /// Observables.
    pub observables: Vec<CompiledObservable>,
    /// Invariants.
    pub invariants: Vec<CompiledInvariant>,
    /// Tables.
    pub tables: Vec<CompiledTable>,
    /// Paradigms used.
    pub paradigms: Vec<ParadigmId>,
}

impl CompiledPlan {
    /// Number of stock slots.
    #[must_use]
    pub fn stock_count(&self) -> usize {
        self.layout.stock_count
    }

    /// Index of a stock by name.
    #[must_use]
    pub fn stock_index(&self, name: &str) -> Option<usize> {
        self.stocks.iter().position(|stock| stock.name == name)
    }

    /// Index of an observable by name.
    #[must_use]
    pub fn observable_index(&self, name: &str) -> Option<usize> {
        self.observables.iter().position(|obs| obs.name == name)
    }

    /// Finds a table by name.
    #[must_use]
    pub fn table(&self, name: &str) -> Option<&CompiledTable> {
        self.tables.iter().find(|table| table.name == name)
    }
}

/// Compiles a model into a [`CompiledPlan`].
///
/// # Errors
/// Returns every diagnostic found; the plan is only produced when there are no
/// errors.
#[allow(clippy::too_many_lines)]
pub fn compile_model(model: &Model) -> Result<CompiledPlan, Diagnostics> {
    let mut diagnostics = Diagnostics::new();
    let registry = UnitRegistry::new();

    let Some(time) = compile_time(model, &registry, &mut diagnostics) else {
        return Err(diagnostics);
    };

    let (tables, table_signatures) = compile_tables(model, &registry, &mut diagnostics);

    let mut names: indexmap::IndexMap<String, (usize, Type)> = indexmap::IndexMap::new();
    let mut slots: Vec<SlotInfo> = Vec::new();

    for (name, spec) in &model.stocks {
        let Ok(unit) = registry.parse(&spec.unit) else {
            diagnostics.push(Diagnostic::error(
                "E1205",
                format!("unknown unit `{}`", spec.unit),
                Some(&format!("stocks.{name}.unit")),
            ));
            continue;
        };
        let slot = slots.len();
        names.insert(name.clone(), (slot, Type::scalar(unit.dim().clone())));
        slots.push(SlotInfo {
            name: name.clone(),
            kind: SlotKind::Stock,
            dim: unit.dim().clone(),
            unit,
            scale: spec.scale.unwrap_or(1.0),
        });
    }
    let stock_count = slots.len();

    for (name, spec) in &model.parameters {
        let Ok(unit) = registry.parse(&spec.unit) else {
            diagnostics.push(Diagnostic::error(
                "E1205",
                format!("unknown unit `{}`", spec.unit),
                Some(&format!("parameters.{name}.unit")),
            ));
            continue;
        };
        if unit.is_affine() {
            diagnostics.push(Diagnostic::error(
                "E1206",
                "parameters cannot use affine units (°C, °F); use K",
                Some(&format!("parameters.{name}.unit")),
            ));
        }
        let slot = slots.len();
        let ty = match spec.kind.as_deref() {
            None | Some("scalar") => Type::scalar(unit.dim().clone()),
            Some("complex") => Type::Complex(unit.dim().clone()),
            Some(other) => {
                diagnostics.push(Diagnostic::error(
                    "E1901",
                    format!("unknown value kind `{other}`; expected `scalar` or `complex`"),
                    Some(&format!("parameters.{name}.kind")),
                ));
                Type::scalar(unit.dim().clone())
            }
        };
        names.insert(name.clone(), (slot, ty));
        slots.push(SlotInfo {
            name: name.clone(),
            kind: SlotKind::Parameter,
            dim: unit.dim().clone(),
            unit,
            scale: spec.scale.unwrap_or(1.0),
        });
    }
    let parameter_count = slots.len() - stock_count;

    for (name, spec) in &model.auxiliaries {
        let Ok(unit) = registry.parse(&spec.unit) else {
            diagnostics.push(Diagnostic::error(
                "E1205",
                format!("unknown unit `{}`", spec.unit),
                Some(&format!("auxiliaries.{name}.unit")),
            ));
            continue;
        };
        let slot = slots.len();
        names.insert(name.clone(), (slot, Type::scalar(unit.dim().clone())));
        slots.push(SlotInfo {
            name: name.clone(),
            kind: SlotKind::Auxiliary,
            dim: unit.dim().clone(),
            unit,
            scale: spec.scale.unwrap_or(1.0),
        });
    }
    let auxiliary_count = slots.len() - stock_count - parameter_count;

    let resolver = ModelResolver {
        names: &names,
        tables: &table_signatures,
    };

    let mut parameters = Vec::new();
    for (name, spec) in &model.parameters {
        let Some((slot, _)) = names.get(name) else {
            continue;
        };
        let unit = registry.parse(&spec.unit).expect("checked above");
        if spec.role == ParameterRole::Fixed && spec.source.is_none() {
            diagnostics.push(Diagnostic::error(
                "E2304",
                format!("fixed parameter `{name}` must declare a source (P6)"),
                Some(&format!("parameters.{name}.source")),
            ));
        }
        if let Some(range) = spec.range
            && range[0] > range[1]
        {
            diagnostics.push(Diagnostic::error(
                "E1209",
                format!("range of `{name}` is inverted"),
                Some(&format!("parameters.{name}.range")),
            ));
        }
        parameters.push(CompiledParameter {
            name: name.clone(),
            slot: *slot,
            value: unit.to_si(spec.value),
            range: spec
                .range
                .map(|range| (unit.to_si(range[0]), unit.to_si(range[1]))),
            role: spec.role,
        });
    }

    let mut stocks: Vec<CompiledStock> = Vec::new();
    for (name, spec) in &model.stocks {
        let Some((slot, ty)) = names.get(name) else {
            continue;
        };
        let path = format!("stocks.{name}.initial");
        let initial =
            match parse_checked(&spec.initial, &resolver, &registry, &path, &mut diagnostics) {
                Some(typed) => {
                    if contains_stock_load(&typed, stock_count) {
                        diagnostics.push(Diagnostic::error(
                            "E1210",
                            format!("initial value of `{name}` cannot depend on a stock"),
                            Some(&path),
                        ));
                        0.0
                    } else {
                        let compiled = compile(&typed);
                        let raw = evaluate_with_parameters(&compiled, &parameters, slots.len());
                        let declared = registry.parse(&spec.unit).expect("checked above");
                        if compiled.ty().dim().is_some_and(Dim::is_dimensionless)
                            && !declared.dim().is_dimensionless()
                        {
                            declared.to_si(raw)
                        } else {
                            if compiled.ty().dim() != ty.dim() {
                                diagnostics.push(Diagnostic::error(
                                    "E1201",
                                    format!(
                                        "initial value has dimension {}, stock declares {}",
                                        describe_dim(compiled.ty().dim()),
                                        describe_dim(ty.dim())
                                    ),
                                    Some(&path),
                                ));
                            }
                            raw
                        }
                    }
                }
                None => 0.0,
            };
        let declared = registry.parse(&spec.unit).expect("checked above");
        let bounds = spec.bounds.as_ref().map(|bounds| {
            let min = bounds.min.map(|value| declared.to_si(value));
            let max = bounds.max.as_ref().and_then(|expr| {
                parse_checked(
                    expr,
                    &resolver,
                    &registry,
                    &format!("stocks.{name}.bounds.max"),
                    &mut diagnostics,
                )
                .map(|typed| {
                    let compiled = compile(&typed);
                    let raw = evaluate_with_parameters(&compiled, &parameters, slots.len());
                    if compiled.ty().dim().is_some_and(Dim::is_dimensionless)
                        && !declared.dim().is_dimensionless()
                    {
                        declared.to_si(raw)
                    } else {
                        raw
                    }
                })
            });
            (min, max)
        });
        stocks.push(CompiledStock {
            name: name.clone(),
            slot: *slot,
            initial,
            bounds,
            inflows: Vec::new(),
            outflows: Vec::new(),
        });
    }

    let auxiliaries = compile_auxiliaries(model, &names, &resolver, &registry, &mut diagnostics);

    let mut flows: Vec<CompiledFlow> = Vec::new();
    let time_dim = Dim::base(BaseDim::Time);
    for (name, spec) in &model.flows {
        let path = format!("flows.{name}");
        let mut targets: Vec<(usize, i32)> = Vec::new();
        let mut declared_dims: Vec<&Dim> = Vec::new();
        for (role_name, sign) in [(&spec.from, -1), (&spec.to, 1)] {
            let Some(stock_name) = role_name else {
                continue;
            };
            let Some((target_slot, target_ty)) = names.get(stock_name) else {
                diagnostics.push(Diagnostic::error(
                    "E2101",
                    format!("flow `{name}` references unknown stock `{stock_name}`"),
                    Some(&path),
                ));
                continue;
            };
            let Some(dim) = target_ty.dim() else {
                continue;
            };
            declared_dims.push(dim);
            targets.push((*target_slot, sign));
        }
        if targets.is_empty() {
            diagnostics.push(Diagnostic::error(
                "E3401",
                format!("flow `{name}` needs one of `from` or `to`"),
                Some(&path),
            ));
            continue;
        }
        if declared_dims.len() == 2 && declared_dims[0] != declared_dims[1] {
            diagnostics.push(Diagnostic::error(
                "E1203",
                format!(
                    "conversion flow `{name}` moves between different dimensions: {} and {}",
                    declared_dims[0], declared_dims[1]
                ),
                Some(&path),
            ));
            continue;
        }
        let Some(stock_dim) = declared_dims.first().map(|dim| (*dim).clone()) else {
            continue;
        };
        let Ok(flow_unit) = registry.parse(&spec.unit) else {
            diagnostics.push(Diagnostic::error(
                "E1205",
                format!("unknown unit `{}`", spec.unit),
                Some(&format!("{path}.unit")),
            ));
            continue;
        };
        let expected = if model.time.kind == ModelTimeKind::Discrete {
            stock_dim.clone()
        } else {
            &stock_dim / &time_dim
        };
        if flow_unit.dim() != &expected {
            diagnostics.push(
                Diagnostic::error(
                    "E1203",
                    format!(
                        "flow unit {} is {}, but {} per {} is {}",
                        spec.unit,
                        flow_unit.dim(),
                        stock_dim,
                        model.time.unit,
                        expected
                    ),
                    Some(&format!("{path}.unit")),
                )
                .with_help(if model.time.kind == ModelTimeKind::Discrete {
                    "in a discrete model a flow is the increment per step, in the stock unit"
                } else {
                    "a flow must be a stock unit divided by the time unit"
                }),
            );
        }
        let Some(typed) = parse_checked(
            &spec.eq,
            &resolver,
            &registry,
            &format!("{path}.eq"),
            &mut diagnostics,
        ) else {
            continue;
        };
        if typed.ty.dim() != Some(&expected) {
            diagnostics.push(Diagnostic::error(
                "E1203",
                format!(
                    "flow rate has dimension {}, expected {}",
                    describe_dim(typed.ty.dim()),
                    expected
                ),
                Some(&format!("{path}.eq")),
            ));
            continue;
        }
        let index = flows.len();
        flows.push(CompiledFlow {
            name: name.clone(),
            rate: compile(&typed),
            targets: targets.clone(),
        });
        for (slot, sign) in &targets {
            if let Some(stock) = stocks.iter_mut().find(|stock| stock.slot == *slot) {
                if *sign > 0 {
                    stock.inflows.push(index);
                } else {
                    stock.outflows.push(index);
                }
            }
        }
    }

    let mut observables = Vec::new();
    for (name, spec) in &model.observables {
        let path = format!("observables.{name}");
        let Ok(unit) = registry.parse(&spec.unit) else {
            diagnostics.push(Diagnostic::error(
                "E1205",
                format!("unknown unit `{}`", spec.unit),
                Some(&format!("{path}.unit")),
            ));
            continue;
        };
        let Some(typed) = parse_checked(
            &spec.eq,
            &resolver,
            &registry,
            &format!("{path}.eq"),
            &mut diagnostics,
        ) else {
            continue;
        };
        if typed.ty.dim() != Some(unit.dim()) {
            diagnostics.push(Diagnostic::error(
                "E1208",
                format!(
                    "observable `{name}` declares {} but its expression has {}",
                    unit.dim(),
                    describe_dim(typed.ty.dim())
                ),
                Some(&format!("{path}.eq")),
            ));
        }
        observables.push(CompiledObservable {
            name: name.clone(),
            unit,
            expr: compile(&typed),
            display: spec.display.clone(),
        });
    }

    let mut invariants = Vec::new();
    for spec in &model.invariants {
        let expr = spec.expr.as_ref().and_then(|expr| {
            let typed = parse_checked(
                expr,
                &resolver,
                &registry,
                &format!("invariants.{}", spec.id),
                &mut diagnostics,
            )?;
            if !typed.ty.is_bool() {
                diagnostics.push(Diagnostic::error(
                    "E1202",
                    format!("invariant `{}` must be a boolean expression", spec.id),
                    Some(&format!("invariants.{}", spec.id)),
                ));
            }
            Some(compile(&typed))
        });
        invariants.push(CompiledInvariant {
            id: spec.id.clone(),
            kind: spec.kind,
            expr,
        });
    }

    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(diagnostics);
    }

    let normalized = model.to_yaml().unwrap_or_default();
    let content_hash = *blake3::hash(normalized.as_bytes()).as_bytes();

    Ok(CompiledPlan {
        model_id: model.id.clone(),
        model_version: model.version.clone(),
        content_hash,
        time,
        layout: StateLayout {
            slots,
            stock_count,
            parameter_count,
            auxiliary_count,
        },
        parameters,
        stocks,
        auxiliaries,
        flows,
        observables,
        invariants,
        tables,
        paradigms: model.paradigms.clone(),
    })
}

fn compile_time(
    model: &Model,
    registry: &UnitRegistry,
    diagnostics: &mut Diagnostics,
) -> Option<TimeBase> {
    let unit = match registry.parse(&model.time.unit) {
        Ok(unit) => unit,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "E1205",
                format!("unknown time unit: {error}"),
                Some("time.unit"),
            ));
            return None;
        }
    };
    let discrete = model.time.kind == ModelTimeKind::Discrete;
    if !discrete && unit.dim() != &Dim::base(BaseDim::Time) {
        diagnostics.push(Diagnostic::error(
            "E1201",
            format!("time unit `{}` is not a time", model.time.unit),
            Some("time.unit"),
        ));
        return None;
    }
    if !(model.time.dt > 0.0 && model.time.dt.is_finite()) {
        diagnostics.push(Diagnostic::error(
            "E3411",
            "time.dt must be a positive finite number",
            Some("time.dt"),
        ));
        return None;
    }
    let start = model.time.start.unwrap_or(0.0);
    if start < 0.0 {
        diagnostics.push(Diagnostic::error(
            "E3411",
            "time.start must be non-negative",
            Some("time.start"),
        ));
        return None;
    }
    Some(TimeBase {
        quantum: if discrete {
            model.time.dt
        } else {
            model.time.dt * unit.factor()
        },
        unit_factor: if discrete { 1.0 } else { unit.factor() },
        unit_symbol: model.time.unit.clone(),
        kind: model.time.kind,
        start_tick: (start / model.time.dt).round() as u64,
    })
}

fn compile_tables(
    model: &Model,
    registry: &UnitRegistry,
    diagnostics: &mut Diagnostics,
) -> (Vec<CompiledTable>, Vec<TableSignature>) {
    let mut tables = Vec::new();
    let mut signatures = Vec::new();
    for (name, spec) in &model.tables {
        let Ok(x_unit) = registry.parse(&spec.x.unit) else {
            diagnostics.push(Diagnostic::error(
                "E1205",
                format!("unknown unit `{}`", spec.x.unit),
                Some(&format!("tables.{name}.x.unit")),
            ));
            continue;
        };
        let Ok(y_unit) = registry.parse(&spec.y.unit) else {
            diagnostics.push(Diagnostic::error(
                "E1205",
                format!("unknown unit `{}`", spec.y.unit),
                Some(&format!("tables.{name}.y.unit")),
            ));
            continue;
        };
        if spec.x.points.len() != spec.y.points.len() {
            diagnostics.push(Diagnostic::error(
                "E3410",
                format!(
                    "table `{name}` has {} x points and {} y points",
                    spec.x.points.len(),
                    spec.y.points.len()
                ),
                Some(&format!("tables.{name}")),
            ));
            continue;
        }
        if spec.x.points.windows(2).any(|pair| pair[0] >= pair[1]) {
            diagnostics.push(Diagnostic::error(
                "E3410",
                format!("table `{name}` x points must be strictly increasing"),
                Some(&format!("tables.{name}.x.points")),
            ));
        }
        let id = u32::try_from(tables.len()).unwrap_or(u32::MAX);
        tables.push(CompiledTable {
            name: name.clone(),
            x: spec
                .x
                .points
                .iter()
                .map(|point| x_unit.to_si(*point))
                .collect(),
            y: spec
                .y
                .points
                .iter()
                .map(|point| y_unit.to_si(*point))
                .collect(),
        });
        signatures.push(TableSignature {
            name: name.clone(),
            id,
            input: x_unit.dim().clone(),
            output: y_unit.dim().clone(),
        });
    }
    (tables, signatures)
}

struct ModelResolver<'a> {
    names: &'a indexmap::IndexMap<String, (usize, Type)>,
    tables: &'a [TableSignature],
}

impl NameResolver for ModelResolver<'_> {
    fn resolve(&self, name: &str) -> Option<Resolved> {
        self.names.get(name).map(|(slot, ty)| Resolved::Variable {
            id: u32::try_from(*slot).unwrap_or(u32::MAX),
            ty: ty.clone(),
        })
    }

    fn resolve_table(&self, name: &str) -> Option<TableSignature> {
        self.tables.iter().find(|table| table.name == name).cloned()
    }
}

fn parse_checked(
    source: &str,
    resolver: &ModelResolver<'_>,
    registry: &UnitRegistry,
    path: &str,
    diagnostics: &mut Diagnostics,
) -> Option<cetus_expr::TypedExpr> {
    match parse_and_check(source, resolver, registry) {
        Ok(typed) => {
            if !typed.ty.is_m1_supported() {
                diagnostics.push(Diagnostic::error(
                    "E1901",
                    format!(
                        "type {} is not supported by this paradigm yet",
                        typed.ty.describe()
                    ),
                    Some(path),
                ));
                return None;
            }
            Some(typed)
        }
        Err(error) => {
            let span = error.span();
            diagnostics.push(
                Diagnostic::error(error.code(), error.message(), Some(path))
                    .with_span((span.start, span.end)),
            );
            None
        }
    }
}

fn contains_stock_load(typed: &cetus_expr::TypedExpr, stock_count: usize) -> bool {
    match &typed.kind {
        cetus_expr::TypedKind::Load(id) => (*id as usize) < stock_count,
        cetus_expr::TypedKind::Unary(_, inner) => contains_stock_load(inner, stock_count),
        cetus_expr::TypedKind::Binary(_, lhs, rhs) => {
            contains_stock_load(lhs, stock_count) || contains_stock_load(rhs, stock_count)
        }
        cetus_expr::TypedKind::If(cond, then, otherwise) => {
            contains_stock_load(cond, stock_count)
                || contains_stock_load(then, stock_count)
                || contains_stock_load(otherwise, stock_count)
        }
        cetus_expr::TypedKind::Call(_, args) => {
            args.iter().any(|arg| contains_stock_load(arg, stock_count))
        }
        cetus_expr::TypedKind::Lookup(_, x) => contains_stock_load(x, stock_count),
        _ => false,
    }
}

fn evaluate_with_parameters(
    expr: &CompiledExpr,
    parameters: &[CompiledParameter],
    slot_count: usize,
) -> f64 {
    let mut ctx = cetus_expr::SlotContext::new(slot_count);
    for parameter in parameters {
        ctx.set(
            u32::try_from(parameter.slot).unwrap_or(u32::MAX),
            parameter.value,
        );
    }
    expr.eval(&ctx)
}

fn compile_auxiliaries(
    model: &Model,
    names: &indexmap::IndexMap<String, (usize, Type)>,
    resolver: &ModelResolver<'_>,
    registry: &UnitRegistry,
    diagnostics: &mut Diagnostics,
) -> Vec<CompiledAuxiliary> {
    let mut typed: indexmap::IndexMap<String, cetus_expr::TypedExpr> = indexmap::IndexMap::new();
    for (name, spec) in &model.auxiliaries {
        let path = format!("auxiliaries.{name}.eq");
        if let Some(expr) = parse_checked(&spec.eq, resolver, registry, &path, diagnostics) {
            if let Some((_, declared)) = names.get(name)
                && expr.ty.dim() != declared.dim()
            {
                diagnostics.push(Diagnostic::error(
                    "E1208",
                    format!(
                        "auxiliary `{name}` declares {} but its equation has {}",
                        describe_dim(declared.dim()),
                        describe_dim(expr.ty.dim())
                    ),
                    Some(&path),
                ));
            }
            typed.insert(name.clone(), expr);
        }
    }

    let aux_slots: BTreeMap<u32, String> = names
        .iter()
        .filter(|(name, _)| model.auxiliaries.contains_key(*name))
        .map(|(name, (slot, _))| (u32::try_from(*slot).unwrap_or(u32::MAX), name.clone()))
        .collect();

    let mut order: Vec<String> = Vec::new();
    let mut visiting: Vec<String> = Vec::new();
    let mut done: BTreeSet<String> = BTreeSet::new();
    for name in model.auxiliaries.keys() {
        if !typed.contains_key(name) {
            continue;
        }
        visit_auxiliary(
            name,
            &typed,
            &aux_slots,
            &mut visiting,
            &mut done,
            &mut order,
            diagnostics,
        );
    }

    let mut out = Vec::new();
    for name in order {
        let Some(expr) = typed.get(&name) else {
            continue;
        };
        let Some((slot, _)) = names.get(&name) else {
            continue;
        };
        out.push(CompiledAuxiliary {
            name: name.clone(),
            slot: *slot,
            expr: compile(expr),
        });
    }
    out
}

fn visit_auxiliary(
    name: &str,
    typed: &indexmap::IndexMap<String, cetus_expr::TypedExpr>,
    aux_slots: &BTreeMap<u32, String>,
    visiting: &mut Vec<String>,
    done: &mut BTreeSet<String>,
    order: &mut Vec<String>,
    diagnostics: &mut Diagnostics,
) {
    if done.contains(name) {
        return;
    }
    if visiting.iter().any(|entry| entry == name) {
        let cycle = visiting
            .iter()
            .skip_while(|entry| entry.as_str() != name)
            .cloned()
            .collect::<Vec<_>>();
        let mut message = String::new();
        for (index, entry) in cycle.iter().enumerate() {
            if index > 0 {
                message.push_str(" -> ");
            }
            let _ = write!(message, "{entry}");
        }
        let _ = write!(message, " -> {name}");
        diagnostics.push(
            Diagnostic::error(
                "E3101",
                format!("algebraic cycle between auxiliaries: {message}"),
                Some(&format!("auxiliaries.{name}.eq")),
            )
            .with_help("break the cycle with `smooth` or `delay1`, or move it to a `solve:` block"),
        );
        return;
    }
    let Some(expr) = typed.get(name) else {
        return;
    };
    visiting.push(name.to_owned());
    let mut loads = Vec::new();
    collect_loads(expr, &mut loads);
    for slot in loads {
        if let Some(dependency) = aux_slots.get(&slot) {
            visit_auxiliary(
                dependency,
                typed,
                aux_slots,
                visiting,
                done,
                order,
                diagnostics,
            );
        }
    }
    visiting.pop();
    done.insert(name.to_owned());
    order.push(name.to_owned());
}

fn collect_loads(expr: &cetus_expr::TypedExpr, out: &mut Vec<u32>) {
    match &expr.kind {
        cetus_expr::TypedKind::Load(id) => out.push(*id),
        cetus_expr::TypedKind::Unary(_, inner) => collect_loads(inner, out),
        cetus_expr::TypedKind::Binary(_, lhs, rhs) => {
            collect_loads(lhs, out);
            collect_loads(rhs, out);
        }
        cetus_expr::TypedKind::If(cond, then, otherwise) => {
            collect_loads(cond, out);
            collect_loads(then, out);
            collect_loads(otherwise, out);
        }
        cetus_expr::TypedKind::Call(_, args) => {
            for arg in args {
                collect_loads(arg, out);
            }
        }
        cetus_expr::TypedKind::Lookup(_, x) => collect_loads(x, out),
        _ => {}
    }
}

fn describe_dim(dim: Option<&Dim>) -> String {
    dim.map_or_else(|| "non-numeric".to_owned(), ToString::to_string)
}

/// Formats diagnostics as Markdown.
#[must_use]
pub fn diagnostics_to_markdown(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::from("| Severity | Code | Path | Message |\n|---|---|---|---|\n");
    for diagnostic in diagnostics {
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let _ = writeln!(
            out,
            "| {severity} | `{}` | `{}` | {} |",
            diagnostic.code,
            diagnostic.path.as_deref().unwrap_or("-"),
            diagnostic.message.replace('|', "\\|")
        );
    }
    out
}

/// Checks whether two model expressions are numerically equivalent over a
/// declared domain (§5.2).
#[must_use]
pub fn expressions_equivalent(
    lhs: &CompiledExpr,
    rhs: &CompiledExpr,
    domain: &[DomainVar],
    options: &EquivOptions,
) -> EquivOutcome {
    equivalent(lhs, rhs, domain, options)
}

/// Returns every variable load in an expression (used by the verifier).
#[must_use]
pub fn loads_of(expr: &CompiledExpr) -> Vec<u32> {
    expr.code()
        .iter()
        .filter_map(|instruction| match instruction {
            Instr::Load(id) => Some(*id),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
pub(crate) const SOLAR_YAML: &str = r#"
schema: cetus.model/0.1
id: energy.solar-battery
version: 1.0.0
title: { zh-CN: "太阳能板与蓄电池", en: "Solar panel and battery" }
tier: empirical
paradigms: [sd]
time:
  unit: h
  dt: 0.05
  horizon: 72
  integrator: rk4
parameters:
  area: { unit: "m^2", value: 12, range: [1, 100], role: control }
  efficiency: { unit: "1", value: 0.20, range: [0.05, 0.47], source: "param:pv.module-efficiency" }
  peak_irradiance: { unit: "W/m^2", value: 1000, range: [0, 1400], source: "param:solar.peak-irradiance.earthlike" }
  day_length: { unit: h, value: 26, source: "world:planet.day_length" }
  capacity: { unit: "kW*h", value: 20, range: [1, 200], role: control }
  load: { unit: W, value: 800, range: [0, 5000], role: input }
stocks:
  charge:
    unit: "kW*h"
    initial: "0.5 * capacity"
    bounds: { min: 0, max: capacity }
auxiliaries:
  irradiance: { unit: "W/m^2", eq: "peak_irradiance * max(0, sin(2*pi*time/day_length))" }
  pv_power: { unit: W, eq: "efficiency * area * irradiance" }
flows:
  charging: { to: charge, unit: W, eq: "if charge < capacity then pv_power else 0" }
  discharging: { from: charge, unit: W, eq: "if charge > 0 then load else 0" }
observables:
  charge_pct: { unit: "1", eq: "charge / capacity", display: percent }
invariants:
  - { id: charge-bounds, kind: bounds, expr: "charge >= 0 and charge <= capacity" }
meta:
  authors: ["@handle"]
  sources: ["src:iea-pv-2024"]
  assumptions:
    - "辐照度用正弦半波近似日变化"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn solar_model() -> Model {
        Model::from_yaml(crate::SOLAR_YAML).expect("parse")
    }

    #[test]
    fn compiles_the_solar_model() {
        let model = solar_model();
        let plan = compile_model(&model).expect("compile");
        assert_eq!(plan.stock_count(), 1);
        assert_eq!(plan.parameters.len(), 6);
        assert_eq!(plan.flows.len(), 2);
        assert_eq!(plan.observables.len(), 1);
        assert!(plan.time.quantum > 0.0);
        let charge = plan.stock_index("charge").expect("charge");
        assert_eq!(plan.stocks[charge].inflows.len(), 1);
        assert_eq!(plan.stocks[charge].outflows.len(), 1);
    }

    #[test]
    fn rejects_a_flow_with_the_wrong_dimension() {
        let yaml = crate::SOLAR_YAML.replace(
            "charging: { to: charge, unit: W, eq: \"if charge < capacity then pv_power else 0\" }",
            "charging: { to: charge, unit: \"m\", eq: \"pv_power\" }",
        );
        let model = Model::from_yaml(&yaml).expect("parse");
        let diagnostics = compile_model(&model).expect_err("must fail");
        let flow = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "E1203")
            .expect("E1203");
        assert_eq!(flow.path.as_deref(), Some("flows.charging.unit"));
    }

    #[test]
    fn rejects_unknown_identifier() {
        let yaml = crate::SOLAR_YAML.replace(
            "efficiency * area * irradiance",
            "efficiency * area * sunshine",
        );
        let model = Model::from_yaml(&yaml).expect("parse");
        let diagnostics = compile_model(&model).expect_err("must fail");
        assert!(diagnostics.iter().any(|d| d.code == "E2101"));
    }

    #[test]
    fn detects_algebraic_cycles() {
        let yaml = crate::SOLAR_YAML.replace(
            "irradiance: { unit: \"W/m^2\", eq: \"peak_irradiance * max(0, sin(2*pi*time/day_length))\" }",
            "irradiance: { unit: \"W/m^2\", eq: \"peak_irradiance * max(0, sin(2*pi*time/day_length)) * (1 + 0 * pv_power / (1[W]))\" }",
        );
        let model = Model::from_yaml(&yaml).expect("parse");
        let diagnostics = compile_model(&model).expect_err("must fail");
        assert!(
            diagnostics.iter().any(|d| d.code == "E3101"),
            "{diagnostics:?}"
        );
    }
}

#[cfg(test)]
mod future_paradigm_tests {
    use super::*;

    #[test]
    fn complex_model_typechecks_then_reports_e1901() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/fixtures/complex-fixture.yaml");
        let source = std::fs::read_to_string(path).expect("fixture");
        let model = Model::from_yaml(&source).expect("parse");
        let diagnostics = compile_model(&model).expect_err("complex is not evaluable in M1");
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E1901"),
            "{diagnostics:?}"
        );
    }
}
