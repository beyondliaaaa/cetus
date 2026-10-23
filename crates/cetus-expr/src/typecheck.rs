//! Value types (K1) and the type/dimension checker.
//!
//! The `Type` enum covers every shape a future paradigm may need. M1 implements
//! the scalar family; other branches parse, resolve and type-check but report
//! `E1901` when evaluated.

use crate::parser::{BinOp, Expr, ExprKind, Span, UnOp};
use cetus_units::{BaseDim, Dim, Rational, UnitRegistry};

/// Identifier of an enum type declared by a model.
pub type EnumId = String;
/// Identifier of an agent type.
pub type AgentTypeId = String;
/// Identifier of a graph.
pub type GraphId = String;
/// Identifier of a grid.
pub type GridId = String;
/// Identifier of a geometry kind.
pub type GeometryKind = String;
/// Identifier of a dataset schema.
pub type DatasetSchemaId = String;

/// A value type.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// A real number with a dimension.
    Scalar(Dim),
    /// An integer with a dimension (counts, indices).
    Int(Dim),
    /// A boolean.
    Bool,
    /// A declared enum.
    Enum(EnumId),
    /// A complex number with a dimension (quantum, waves, AC circuits).
    Complex(Dim),
    /// A fixed-length vector.
    Vector(Box<Self>, usize),
    /// A fixed-size matrix.
    Matrix(Box<Self>, usize, usize),
    /// A variable-length array.
    Array(Box<Self>),
    /// A field on a grid.
    Field(Box<Self>, GridId),
    /// An agent table.
    Table(AgentTypeId),
    /// A graph.
    Graph(GraphId),
    /// A geometric object.
    Geometry(GeometryKind),
    /// A dataset.
    Dataset(DatasetSchemaId),
}

impl Type {
    /// A dimensionless scalar.
    #[must_use]
    pub fn number() -> Self {
        Self::Scalar(Dim::dimensionless())
    }

    /// A scalar with the given dimension.
    #[must_use]
    pub fn scalar(dim: Dim) -> Self {
        Self::Scalar(dim)
    }

    /// The dimension of a numeric type, if any.
    #[must_use]
    pub fn dim(&self) -> Option<&Dim> {
        match self {
            Self::Scalar(dim) | Self::Int(dim) | Self::Complex(dim) => Some(dim),
            _ => None,
        }
    }

    /// Whether this is a number (scalar, integer or complex).
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        self.dim().is_some()
    }

    /// Whether this is a boolean.
    #[must_use]
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool)
    }

    /// Whether M1 evaluation supports this type. Other branches are parsed,
    /// resolved and type-checked, then rejected with E1901 (K1).
    #[must_use]
    pub fn is_m1_supported(&self) -> bool {
        matches!(self, Self::Scalar(_) | Self::Int(_) | Self::Bool)
    }

    /// Human-readable name used in diagnostics.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Scalar(dim) => format!("a scalar with dimension {dim}"),
            Self::Int(dim) => format!("an integer with dimension {dim}"),
            Self::Bool => "a boolean".to_owned(),
            Self::Enum(id) => format!("enum `{id}`"),
            Self::Complex(dim) => format!("a complex number with dimension {dim}"),
            Self::Vector(inner, len) => format!("a length-{len} vector of {}", inner.describe()),
            Self::Matrix(inner, rows, cols) => {
                format!("a {rows}x{cols} matrix of {}", inner.describe())
            }
            Self::Array(inner) => format!("an array of {}", inner.describe()),
            Self::Field(inner, grid) => format!("a field of {} on grid {grid}", inner.describe()),
            Self::Table(id) => format!("agent table `{id}`"),
            Self::Graph(id) => format!("graph `{id}`"),
            Self::Geometry(kind) => format!("geometry `{kind}`"),
            Self::Dataset(id) => format!("dataset `{id}`"),
        }
    }
}

/// What a name resolves to.
#[derive(Debug, Clone)]
pub enum Resolved {
    /// A variable (parameter, stock, auxiliary, port, built-in `time` / `dt`).
    Variable {
        /// Slot identifier assigned by the caller's state layout.
        id: u32,
        /// Its type.
        ty: Type,
    },
    /// A compile-time constant.
    Constant {
        /// Its value in SI units.
        value: f64,
        /// Its type.
        ty: Type,
    },
}

/// Table signature for `lookup(table, x)`.
#[derive(Debug, Clone)]
pub struct TableSignature {
    /// Table name as written in the model.
    pub name: String,
    /// Identifier assigned by the caller.
    pub id: u32,
    /// Dimension of the table's x axis.
    pub input: Dim,
    /// Dimension of the table's y axis.
    pub output: Dim,
}

/// Resolves identifiers during type checking.
pub trait NameResolver {
    /// Resolve a variable or constant.
    fn resolve(&self, name: &str) -> Option<Resolved>;

    /// Resolve a table name for `lookup`.
    fn resolve_table(&self, _name: &str) -> Option<TableSignature> {
        None
    }
}

/// A type-checking error with a diagnostic code.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message} (at byte {start})")]
pub struct TypeError {
    /// Diagnostic code, e.g. `E1201`.
    pub code: &'static str,
    /// Explanation.
    pub message: String,
    /// Start byte.
    pub start: usize,
    /// End byte.
    pub end: usize,
}

impl TypeError {
    fn new(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            start: span.start,
            end: span.end,
        }
    }
}

/// A typed, ready-to-compile expression.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    /// The type of the expression.
    pub ty: Type,
    /// True for the bare literal `0`, which is dimension-polymorphic (§5.1).
    pub zero_literal: bool,
    /// The typed node.
    pub kind: TypedKind,
}

/// A typed expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum TypedKind {
    /// Compile-time constant in SI units.
    Const(f64),
    /// Variable load.
    Load(u32),
    /// `time`
    Time,
    /// `dt`
    Dt,
    /// Unary operation.
    Unary(UnOp, Box<TypedExpr>),
    /// Binary operation.
    Binary(BinOp, Box<TypedExpr>, Box<TypedExpr>),
    /// Conditional.
    If(Box<TypedExpr>, Box<TypedExpr>, Box<TypedExpr>),
    /// Built-in function call.
    Call(Builtin, Vec<TypedExpr>),
    /// Table lookup; `u32` is the table id.
    Lookup(u32, Box<TypedExpr>),
}

/// Built-in functions (§5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    /// `abs(x)`
    Abs,
    /// `min(a, b)`
    Min,
    /// `max(a, b)`
    Max,
    /// `clamp(x, lo, hi)`
    Clamp,
    /// `exp(x)`
    Exp,
    /// `ln(x)`
    Ln,
    /// `log10(x)`
    Log10,
    /// `log2(x)`
    Log2,
    /// `sqrt(x)`
    Sqrt,
    /// `sin(x)`
    Sin,
    /// `cos(x)`
    Cos,
    /// `tan(x)`
    Tan,
    /// `asin(x)`
    Asin,
    /// `acos(x)`
    Acos,
    /// `atan(x)`
    Atan,
    /// `atan2(y, x)`
    Atan2,
    /// `floor(x)`
    Floor,
    /// `ceil(x)`
    Ceil,
    /// `round(x)`
    Round,
    /// `step(height, t0)`
    Step,
    /// `pulse(t0, width)`
    Pulse,
    /// `ramp(slope, t0)`
    Ramp,
    /// `delay1(x, tau)`
    Delay1,
    /// `delay3(x, tau)`
    Delay3,
    /// `smooth(x, tau)`
    Smooth,
}

impl Builtin {
    /// Looks up a function name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "abs" => Self::Abs,
            "min" => Self::Min,
            "max" => Self::Max,
            "clamp" => Self::Clamp,
            "exp" => Self::Exp,
            "ln" => Self::Ln,
            "log10" => Self::Log10,
            "log2" => Self::Log2,
            "sqrt" => Self::Sqrt,
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "asin" => Self::Asin,
            "acos" => Self::Acos,
            "atan" => Self::Atan,
            "atan2" => Self::Atan2,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "round" => Self::Round,
            "step" => Self::Step,
            "pulse" => Self::Pulse,
            "ramp" => Self::Ramp,
            "delay1" => Self::Delay1,
            "delay3" => Self::Delay3,
            "smooth" => Self::Smooth,
            _ => return None,
        })
    }

    /// The source name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Abs => "abs",
            Self::Min => "min",
            Self::Max => "max",
            Self::Clamp => "clamp",
            Self::Exp => "exp",
            Self::Ln => "ln",
            Self::Log10 => "log10",
            Self::Log2 => "log2",
            Self::Sqrt => "sqrt",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Atan2 => "atan2",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Round => "round",
            Self::Step => "step",
            Self::Pulse => "pulse",
            Self::Ramp => "ramp",
            Self::Delay1 => "delay1",
            Self::Delay3 => "delay3",
            Self::Smooth => "smooth",
        }
    }

    /// Whether this builtin needs simulation state (delay/smooth).
    #[must_use]
    pub fn needs_state(self) -> bool {
        matches!(self, Self::Delay1 | Self::Delay3 | Self::Smooth)
    }

    /// Number of arguments.
    #[must_use]
    pub fn arity(self) -> usize {
        match self {
            Self::Abs
            | Self::Exp
            | Self::Ln
            | Self::Log10
            | Self::Log2
            | Self::Sqrt
            | Self::Sin
            | Self::Cos
            | Self::Tan
            | Self::Asin
            | Self::Acos
            | Self::Atan
            | Self::Floor
            | Self::Ceil
            | Self::Round => 1,
            Self::Min
            | Self::Max
            | Self::Atan2
            | Self::Step
            | Self::Pulse
            | Self::Ramp
            | Self::Delay1
            | Self::Delay3
            | Self::Smooth => 2,
            Self::Clamp => 3,
        }
    }
}

/// Type-checks an expression.
///
/// # Errors
/// Returns a [`TypeError`] with a diagnostic code and byte span.
pub fn typecheck(
    expr: &Expr,
    resolver: &dyn NameResolver,
    registry: &UnitRegistry,
) -> Result<TypedExpr, TypeError> {
    check(expr, resolver, registry)
}

fn check(
    expr: &Expr,
    resolver: &dyn NameResolver,
    registry: &UnitRegistry,
) -> Result<TypedExpr, TypeError> {
    match &expr.kind {
        ExprKind::Number { value, unit } => {
            let (dim, si_value) = match unit {
                Some(text) => {
                    let parsed = registry.parse(text).map_err(|error| {
                        TypeError::new("E1205", format!("invalid unit: {error}"), expr.span)
                    })?;
                    if parsed.is_affine() {
                        return Err(TypeError::new(
                            "E1206",
                            "affine units are only allowed at inputs and outputs",
                            expr.span,
                        ));
                    }
                    (parsed.dim().clone(), parsed.to_si(*value))
                }
                None => (Dim::dimensionless(), *value),
            };
            Ok(TypedExpr {
                zero_literal: unit.is_none() && *value == 0.0,
                ty: Type::scalar(dim),
                kind: TypedKind::Const(si_value),
            })
        }
        ExprKind::Bool(value) => Ok(TypedExpr {
            zero_literal: false,
            ty: Type::Bool,
            kind: TypedKind::Const(if *value { 1.0 } else { 0.0 }),
        }),
        ExprKind::Ident(name) => resolve_name(name, expr.span, resolver),
        ExprKind::Unary { op, operand } => {
            let inner = check(operand, resolver, registry)?;
            match op {
                UnOp::Neg => {
                    if !inner.ty.is_numeric() {
                        return Err(TypeError::new(
                            "E1202",
                            format!("`-` expects a number, found {}", inner.ty.describe()),
                            expr.span,
                        ));
                    }
                    Ok(TypedExpr {
                        zero_literal: false,
                        ty: inner.ty.clone(),
                        kind: TypedKind::Unary(*op, Box::new(inner)),
                    })
                }
                UnOp::Not => {
                    if !inner.ty.is_bool() {
                        return Err(TypeError::new(
                            "E1202",
                            format!("`not` expects a boolean, found {}", inner.ty.describe()),
                            expr.span,
                        ));
                    }
                    Ok(TypedExpr {
                        zero_literal: false,
                        ty: Type::Bool,
                        kind: TypedKind::Unary(*op, Box::new(inner)),
                    })
                }
            }
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let left = check(lhs, resolver, registry)?;
            let right = check(rhs, resolver, registry)?;
            check_binary(*op, left, right, expr.span)
        }
        ExprKind::If {
            cond,
            then,
            otherwise,
        } => {
            let condition = check(cond, resolver, registry)?;
            if !condition.ty.is_bool() {
                return Err(TypeError::new(
                    "E1202",
                    format!(
                        "condition of `if` must be boolean, found {}",
                        condition.ty.describe()
                    ),
                    cond.span,
                ));
            }
            let then_branch = check(then, resolver, registry)?;
            let else_branch = check(otherwise, resolver, registry)?;
            let (mut then_branch, mut else_branch) = (then_branch, else_branch);
            if then_branch.ty != else_branch.ty {
                if then_branch.zero_literal {
                    then_branch.ty = else_branch.ty.clone();
                } else if else_branch.zero_literal {
                    else_branch.ty = then_branch.ty.clone();
                } else {
                    return Err(TypeError::new(
                        "E1207",
                        format!(
                            "branches of `if` have different types: {} vs {}",
                            then_branch.ty.describe(),
                            else_branch.ty.describe()
                        ),
                        expr.span,
                    ));
                }
            }
            Ok(TypedExpr {
                zero_literal: false,
                ty: then_branch.ty.clone(),
                kind: TypedKind::If(
                    Box::new(condition),
                    Box::new(then_branch),
                    Box::new(else_branch),
                ),
            })
        }
        ExprKind::Call { name, args } => {
            if name == "lookup" {
                return check_lookup(args, expr.span, resolver, registry);
            }
            let builtin = Builtin::from_name(name).ok_or_else(|| {
                TypeError::new("E2102", format!("unknown function `{name}`"), expr.span)
            })?;
            check_call(builtin, args, expr.span, resolver, registry)
        }
    }
}

fn resolve_name(
    name: &str,
    span: Span,
    resolver: &dyn NameResolver,
) -> Result<TypedExpr, TypeError> {
    if name == "pi" {
        return Ok(TypedExpr {
            zero_literal: false,
            ty: Type::number(),
            kind: TypedKind::Const(core::f64::consts::PI),
        });
    }
    if name == "e" {
        return Ok(TypedExpr {
            zero_literal: false,
            ty: Type::number(),
            kind: TypedKind::Const(core::f64::consts::E),
        });
    }
    if name == "time" {
        return Ok(TypedExpr {
            zero_literal: false,
            ty: Type::scalar(Dim::base(BaseDim::Time)),
            kind: TypedKind::Time,
        });
    }
    if name == "dt" {
        return Ok(TypedExpr {
            zero_literal: false,
            ty: Type::scalar(Dim::base(BaseDim::Time)),
            kind: TypedKind::Dt,
        });
    }
    match resolver.resolve(name) {
        Some(Resolved::Variable { id, ty }) => Ok(TypedExpr {
            zero_literal: false,
            ty,
            kind: TypedKind::Load(id),
        }),
        Some(Resolved::Constant { value, ty }) => Ok(TypedExpr {
            zero_literal: false,
            ty,
            kind: TypedKind::Const(value),
        }),
        None => Err(TypeError::new(
            "E2101",
            format!("undefined identifier `{name}`"),
            span,
        )),
    }
}

fn check_binary(
    op: BinOp,
    left: TypedExpr,
    right: TypedExpr,
    span: Span,
) -> Result<TypedExpr, TypeError> {
    if op.is_comparison() {
        let (left, right, _dim) = unify_numeric(left, right, span, "compare")?;
        return Ok(TypedExpr {
            zero_literal: false,
            ty: Type::Bool,
            kind: TypedKind::Binary(op, Box::new(left), Box::new(right)),
        });
    }

    if matches!(op, BinOp::And | BinOp::Or) {
        if !left.ty.is_bool() || !right.ty.is_bool() {
            return Err(TypeError::new(
                "E1202",
                format!(
                    "`{}` expects booleans, found {} and {}",
                    op.as_str(),
                    left.ty.describe(),
                    right.ty.describe()
                ),
                span,
            ));
        }
        return Ok(TypedExpr {
            zero_literal: false,
            ty: Type::Bool,
            kind: TypedKind::Binary(op, Box::new(left), Box::new(right)),
        });
    }

    let (Some(left_dim), Some(right_dim)) = (left.ty.dim(), right.ty.dim()) else {
        return Err(TypeError::new(
            "E1202",
            format!(
                "`{}` expects numbers, found {} and {}",
                op.as_str(),
                left.ty.describe(),
                right.ty.describe()
            ),
            span,
        ));
    };
    let left_dim = left_dim.clone();
    let right_dim = right_dim.clone();

    let (left, right, unified) = if matches!(op, BinOp::Add | BinOp::Sub) {
        unify_numeric(left, right, span, op.as_str())?
    } else {
        (left, right, left_dim.clone())
    };
    let result_dim = match op {
        BinOp::Add | BinOp::Sub => unified,
        BinOp::Mul => {
            if left.zero_literal {
                right_dim.clone()
            } else if right.zero_literal {
                left_dim.clone()
            } else {
                &left_dim * &right_dim
            }
        }
        BinOp::Div => {
            if left.zero_literal {
                let _ = &right_dim;
                right_dim.clone()
            } else if right.zero_literal {
                left_dim.clone()
            } else {
                &left_dim / &right_dim
            }
        }
        BinOp::Pow => {
            if !right_dim.is_dimensionless() {
                return Err(TypeError::new(
                    "E1204",
                    format!("exponent must be dimensionless, found {right_dim}"),
                    span,
                ));
            }
            if left_dim.is_dimensionless() {
                left_dim
            } else {
                let TypedKind::Const(exponent) = right.kind else {
                    return Err(TypeError::new(
                        "E1204",
                        "a dimensioned base requires a compile-time integer exponent",
                        span,
                    ));
                };
                let Some(integer) = exact_integer(exponent) else {
                    return Err(TypeError::new(
                        "E1204",
                        format!("exponent {exponent} is not an integer"),
                        span,
                    ));
                };
                left_dim.pow(Rational::from_int(integer))
            }
        }
        _ => unreachable!("comparison and logic handled above"),
    };

    let ty = numeric_result(&left.ty, &right.ty, result_dim);
    Ok(TypedExpr {
        zero_literal: false,
        ty,
        kind: TypedKind::Binary(op, Box::new(left), Box::new(right)),
    })
}

fn numeric_result(left: &Type, right: &Type, dim: Dim) -> Type {
    if matches!(left, Type::Complex(_)) || matches!(right, Type::Complex(_)) {
        Type::Complex(dim)
    } else {
        Type::Scalar(dim)
    }
}

fn exact_integer(value: f64) -> Option<i32> {
    if value.is_finite() && value.fract() == 0.0 && value.abs() <= f64::from(i32::MAX) {
        Some(value as i32)
    } else {
        None
    }
}

fn check_call(
    builtin: Builtin,
    args: &[Expr],
    span: Span,
    resolver: &dyn NameResolver,
    registry: &UnitRegistry,
) -> Result<TypedExpr, TypeError> {
    let expected = builtin.arity();
    if args.len() != expected {
        return Err(TypeError::new(
            "E2103",
            format!(
                "`{}` expects {expected} argument(s), found {}",
                builtin.name(),
                args.len()
            ),
            span,
        ));
    }
    let mut typed: Vec<TypedExpr> = args
        .iter()
        .map(|arg| check(arg, resolver, registry))
        .collect::<Result<_, _>>()?;

    let result = match builtin {
        Builtin::Abs | Builtin::Floor | Builtin::Ceil | Builtin::Round => typed[0].ty.clone(),
        Builtin::Min | Builtin::Max => {
            let dim = unify_pair(&mut typed, 0, 1, span, builtin.name())?;
            Type::scalar(dim)
        }
        Builtin::Clamp => {
            let dim = unify_pair(&mut typed, 0, 1, span, builtin.name())?;
            unify_pair(&mut typed, 1, 2, span, builtin.name())?;
            Type::scalar(dim)
        }
        Builtin::Exp
        | Builtin::Ln
        | Builtin::Log10
        | Builtin::Log2
        | Builtin::Asin
        | Builtin::Acos
        | Builtin::Atan => {
            require_dimensionless(&typed[0], span, builtin.name())?;
            Type::number()
        }
        Builtin::Sqrt => {
            let Some(dim) = typed[0].ty.dim() else {
                return Err(TypeError::new("E1202", "sqrt expects a number", span));
            };
            Type::scalar(dim.pow(Rational::new(1, 2)))
        }
        Builtin::Sin | Builtin::Cos | Builtin::Tan => {
            require_angle_or_dimensionless(&typed[0], span, builtin.name())?;
            Type::number()
        }
        Builtin::Atan2 => {
            require_same_dim(&typed[0], &typed[1], span, builtin.name())?;
            Type::number()
        }
        Builtin::Step | Builtin::Delay1 | Builtin::Delay3 | Builtin::Smooth => {
            require_time(&typed[1], span, builtin.name())?;
            typed[0].ty.clone()
        }
        Builtin::Pulse => {
            require_time(&typed[0], span, builtin.name())?;
            require_time(&typed[1], span, builtin.name())?;
            Type::number()
        }
        Builtin::Ramp => {
            let Some(dim) = typed[0].ty.dim() else {
                return Err(TypeError::new("E1202", "ramp slope must be a number", span));
            };
            require_time(&typed[1], span, builtin.name())?;
            Type::scalar(dim / &Dim::base(BaseDim::Time))
        }
    };

    Ok(TypedExpr {
        zero_literal: false,
        ty: result,
        kind: TypedKind::Call(builtin, typed),
    })
}

fn check_lookup(
    args: &[Expr],
    span: Span,
    resolver: &dyn NameResolver,
    registry: &UnitRegistry,
) -> Result<TypedExpr, TypeError> {
    if args.len() != 2 {
        return Err(TypeError::new(
            "E2103",
            format!("`lookup` expects 2 arguments, found {}", args.len()),
            span,
        ));
    }
    let ExprKind::Ident(table_name) = &args[0].kind else {
        return Err(TypeError::new(
            "E2104",
            "the first argument of `lookup` must be a table name",
            span,
        ));
    };
    let signature = resolver
        .resolve_table(table_name)
        .ok_or_else(|| TypeError::new("E2105", format!("unknown table `{table_name}`"), span))?;
    let x = check(&args[1], resolver, registry)?;
    let Some(x_dim) = x.ty.dim() else {
        return Err(TypeError::new(
            "E1201",
            "lookup input must be a number",
            span,
        ));
    };
    if x_dim != &signature.input {
        return Err(TypeError::new(
            "E1201",
            format!(
                "lookup input has dimension {x_dim}, table expects {}",
                signature.input
            ),
            span,
        ));
    }
    Ok(TypedExpr {
        zero_literal: false,
        ty: Type::scalar(signature.output),
        kind: TypedKind::Lookup(signature.id, Box::new(x)),
    })
}

fn unify_numeric(
    mut left: TypedExpr,
    mut right: TypedExpr,
    span: Span,
    context: &str,
) -> Result<(TypedExpr, TypedExpr, Dim), TypeError> {
    if left.zero_literal && !right.zero_literal {
        let Some(dim) = right.ty.dim().cloned() else {
            return Err(TypeError::new(
                "E1201",
                format!("`{context}` expects numbers, found {}", right.ty.describe()),
                span,
            ));
        };
        left.ty = Type::scalar(dim.clone());
        return Ok((left, right, dim));
    }
    if right.zero_literal && !left.zero_literal {
        let Some(dim) = left.ty.dim().cloned() else {
            return Err(TypeError::new(
                "E1201",
                format!("`{context}` expects numbers, found {}", left.ty.describe()),
                span,
            ));
        };
        right.ty = Type::scalar(dim.clone());
        return Ok((left, right, dim));
    }
    let (Some(left_dim), Some(right_dim)) = (left.ty.dim().cloned(), right.ty.dim().cloned())
    else {
        return Err(TypeError::new(
            "E1201",
            format!(
                "`{context}` expects numbers, found {} and {}",
                left.ty.describe(),
                right.ty.describe()
            ),
            span,
        ));
    };
    if left_dim != right_dim {
        return Err(TypeError::new(
            "E1201",
            format!("`{context}` arguments have dimensions {left_dim} and {right_dim}"),
            span,
        ));
    }
    Ok((left, right, left_dim))
}

fn unify_pair(
    args: &mut [TypedExpr],
    i: usize,
    j: usize,
    span: Span,
    context: &str,
) -> Result<Dim, TypeError> {
    let (left, right, dim) = unify_numeric(args[i].clone(), args[j].clone(), span, context)?;
    args[i] = left;
    args[j] = right;
    Ok(dim)
}

fn require_same_dim(
    left: &TypedExpr,
    right: &TypedExpr,
    span: Span,
    function: &str,
) -> Result<(), TypeError> {
    let (Some(left_dim), Some(right_dim)) = (left.ty.dim(), right.ty.dim()) else {
        return Err(TypeError::new(
            "E1202",
            format!("`{function}` expects numbers"),
            span,
        ));
    };
    if left_dim != right_dim {
        return Err(TypeError::new(
            "E1201",
            format!("`{function}` arguments have dimensions {left_dim} and {right_dim}"),
            span,
        ));
    }
    Ok(())
}

fn require_dimensionless(expr: &TypedExpr, span: Span, function: &str) -> Result<(), TypeError> {
    match expr.ty.dim() {
        Some(dim) if dim.is_dimensionless() => Ok(()),
        Some(dim) => Err(TypeError::new(
            "E1202",
            format!("`{function}` expects a dimensionless argument, found {dim}"),
            span,
        )),
        None => Err(TypeError::new(
            "E1202",
            format!("`{function}` expects a number"),
            span,
        )),
    }
}

fn require_angle_or_dimensionless(
    expr: &TypedExpr,
    span: Span,
    function: &str,
) -> Result<(), TypeError> {
    match expr.ty.dim() {
        Some(dim) if dim.is_dimensionless() => Ok(()),
        Some(dim) => Err(TypeError::new(
            "E1202",
            format!("`{function}` expects an angle, found {dim}"),
            span,
        )),
        None => Err(TypeError::new(
            "E1202",
            format!("`{function}` expects a number"),
            span,
        )),
    }
}

fn require_time(expr: &TypedExpr, span: Span, function: &str) -> Result<(), TypeError> {
    let expected = Dim::base(BaseDim::Time);
    match expr.ty.dim() {
        Some(dim) if *dim == expected => Ok(()),
        Some(dim) => Err(TypeError::new(
            "E1201",
            format!("`{function}` expects a time argument, found {dim}"),
            span,
        )),
        None => Err(TypeError::new(
            "E1202",
            format!("`{function}` expects a time argument"),
            span,
        )),
    }
}

/// Convenience: parse and type-check in one step.
///
/// # Errors
/// Returns a [`crate::ExprError`] on parse or type errors.
pub fn parse_and_check(
    source: &str,
    resolver: &dyn NameResolver,
    registry: &UnitRegistry,
) -> Result<TypedExpr, crate::ExprError> {
    let ast = crate::parser::parse(source)?;
    Ok(typecheck(&ast, resolver, registry)?)
}

#[cfg(test)]
mod future_paradigm_tests {
    use super::*;

    struct ComplexResolver;

    impl NameResolver for ComplexResolver {
        fn resolve(&self, name: &str) -> Option<Resolved> {
            (name == "psi").then(|| Resolved::Variable {
                id: 0,
                ty: Type::Complex(Dim::dimensionless()),
            })
        }
    }

    #[test]
    fn complex_variables_typecheck_but_are_not_supported_yet() {
        let typed = parse_and_check("psi * 2 + psi", &ComplexResolver, &UnitRegistry::new())
            .expect("type-checks");
        assert!(matches!(typed.ty, Type::Complex(_)));
        assert!(!typed.ty.is_m1_supported());
    }

    #[test]
    fn complex_dimension_still_propagates() {
        let typed = parse_and_check("psi * 1[m]", &ComplexResolver, &UnitRegistry::new())
            .expect("type-checks");
        let Type::Complex(dim) = typed.ty else {
            panic!("expected Complex");
        };
        assert_eq!(dim, Dim::base(BaseDim::Length));
    }
}
