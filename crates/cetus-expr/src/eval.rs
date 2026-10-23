//! Compilation of typed expressions to a small stack machine and evaluation.
//!
//! The stack machine is deterministic: instruction order is fixed, floats are
//! pushed and popped in a fixed order, and all transcendental functions go
//! through `cetus-math` (D2).

use crate::parser::{BinOp, UnOp};
use crate::typecheck::{Builtin, Type, TypedExpr, TypedKind};

/// A stack-machine instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    /// Push a constant (SI units).
    Const(f64),
    /// Push a variable slot.
    Load(u32),
    /// Push `time`.
    Time,
    /// Push `dt`.
    Dt,
    /// Negate.
    Neg,
    /// Logical not (0/1).
    Not,
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `^`
    Pow,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// Logical and (0/1).
    And,
    /// Logical or (0/1).
    Or,
    /// Pop `otherwise`, `then`, `cond`; push the selected branch.
    Select,
    /// One-argument builtin.
    Call1(Builtin),
    /// Two-argument builtin; the `u32` is the call-site id for stateful builtins.
    Call2(Builtin, u32),
    /// Three-argument builtin.
    Call3(Builtin),
    /// Table lookup; `u32` is the table id.
    Lookup(u32),
}

/// Reads the current simulation state for evaluation.
pub trait EvalContext {
    /// Value of a variable slot.
    fn var(&self, id: u32) -> f64;
    /// Current simulation time in seconds.
    fn time(&self) -> f64;
    /// Current macro step in seconds.
    fn dt(&self) -> f64;
    /// Current value of a stateful builtin slot (delay/smooth).
    fn state(&self, id: u32) -> f64;
    /// Table lookup.
    fn lookup(&self, table: u32, x: f64) -> f64;
}

/// A compiled expression.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledExpr {
    code: Vec<Instr>,
    ty: Type,
}

impl CompiledExpr {
    /// The static type of the expression.
    #[must_use]
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    /// The instructions, in evaluation order.
    #[must_use]
    pub fn code(&self) -> &[Instr] {
        &self.code
    }

    /// Number of state slots the expression needs (stateful builtins).
    #[must_use]
    pub fn state_slots(&self) -> u32 {
        self.code
            .iter()
            .filter_map(|instr| match instr {
                Instr::Call2(builtin, id) if builtin.needs_state() => Some(*id + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    /// Evaluates the expression.
    ///
    /// # Panics
    /// Panics if the compiled code is malformed. Compilation is the only
    /// constructor of `CompiledExpr`, so this is a programming error.
    #[must_use]
    #[allow(clippy::float_cmp)] // `==` and `!=` in the language are exact; the semantics are defined, not approximate
    pub fn eval(&self, ctx: &dyn EvalContext) -> f64 {
        let mut stack: Vec<f64> = Vec::with_capacity(16);
        for instr in &self.code {
            match instr {
                Instr::Const(value) => stack.push(*value),
                Instr::Load(id) => stack.push(ctx.var(*id)),
                Instr::Time => stack.push(ctx.time()),
                Instr::Dt => stack.push(ctx.dt()),
                Instr::Neg => {
                    let value = pop(&mut stack);
                    stack.push(-value);
                }
                Instr::Not => {
                    let value = pop(&mut stack);
                    stack.push(if value == 0.0 { 1.0 } else { 0.0 });
                }
                Instr::Add => binary(&mut stack, &|a, b| a + b),
                Instr::Sub => binary(&mut stack, &|a, b| a - b),
                Instr::Mul => binary(&mut stack, &|a, b| a * b),
                Instr::Div => binary(&mut stack, &|a, b| a / b),
                Instr::Pow => binary(&mut stack, &cetus_math::powf),
                Instr::Lt => compare(&mut stack, &|a, b| a < b),
                Instr::Le => compare(&mut stack, &|a, b| a <= b),
                Instr::Gt => compare(&mut stack, &|a, b| a > b),
                Instr::Ge => compare(&mut stack, &|a, b| a >= b),
                Instr::Eq => compare(&mut stack, &|a, b| a == b),
                Instr::Ne => compare(&mut stack, &|a, b| a != b),
                Instr::And => binary_bool(&mut stack, &|a, b| a && b),
                Instr::Or => binary_bool(&mut stack, &|a, b| a || b),
                Instr::Select => {
                    let otherwise = pop(&mut stack);
                    let then = pop(&mut stack);
                    let cond = pop(&mut stack);
                    stack.push(if cond == 0.0 { otherwise } else { then });
                }
                Instr::Call1(builtin) => {
                    let value = pop(&mut stack);
                    stack.push(eval1(*builtin, value));
                }
                Instr::Call2(builtin, site) => {
                    let second = pop(&mut stack);
                    let first = pop(&mut stack);
                    stack.push(eval2(*builtin, first, second, *site, ctx));
                }
                Instr::Call3(builtin) => {
                    let third = pop(&mut stack);
                    let second = pop(&mut stack);
                    let first = pop(&mut stack);
                    stack.push(eval3(*builtin, first, second, third));
                }
                Instr::Lookup(table) => {
                    let x = pop(&mut stack);
                    stack.push(ctx.lookup(*table, x));
                }
            }
        }
        pop(&mut stack)
    }
}

fn pop(stack: &mut Vec<f64>) -> f64 {
    stack
        .pop()
        .expect("malformed expression bytecode: stack underflow")
}

fn binary(stack: &mut Vec<f64>, op: &impl Fn(f64, f64) -> f64) {
    let rhs = pop(stack);
    let lhs = pop(stack);
    stack.push(op(lhs, rhs));
}

fn binary_bool(stack: &mut Vec<f64>, op: &impl Fn(bool, bool) -> bool) {
    let rhs = pop(stack);
    let lhs = pop(stack);
    stack.push(if op(lhs != 0.0, rhs != 0.0) { 1.0 } else { 0.0 });
}

fn compare(stack: &mut Vec<f64>, op: &impl Fn(f64, f64) -> bool) {
    let rhs = pop(stack);
    let lhs = pop(stack);
    stack.push(if op(lhs, rhs) { 1.0 } else { 0.0 });
}

fn eval1(builtin: Builtin, value: f64) -> f64 {
    match builtin {
        Builtin::Abs => value.abs(),
        Builtin::Exp => cetus_math::exp(value),
        Builtin::Ln => cetus_math::ln(value),
        Builtin::Log10 => cetus_math::log10(value),
        Builtin::Log2 => cetus_math::log2(value),
        Builtin::Sqrt => cetus_math::sqrt(value),
        Builtin::Sin => cetus_math::sin(value),
        Builtin::Cos => cetus_math::cos(value),
        Builtin::Tan => cetus_math::tan(value),
        Builtin::Asin => cetus_math::asin(value),
        Builtin::Acos => cetus_math::acos(value),
        Builtin::Atan => cetus_math::atan(value),
        Builtin::Floor => cetus_math::floor(value),
        Builtin::Ceil => cetus_math::ceil(value),
        Builtin::Round => cetus_math::round(value),
        _ => unreachable!("{builtin:?} is not a one-argument builtin"),
    }
}

fn eval2(builtin: Builtin, first: f64, second: f64, site: u32, ctx: &dyn EvalContext) -> f64 {
    match builtin {
        Builtin::Min => first.min(second),
        Builtin::Max => first.max(second),
        Builtin::Atan2 => cetus_math::atan2(first, second),
        Builtin::Step => {
            if ctx.time() >= second {
                first
            } else {
                0.0
            }
        }
        Builtin::Pulse => {
            let t = ctx.time();
            if t >= first && t < first + second {
                1.0
            } else {
                0.0
            }
        }
        Builtin::Ramp => {
            let t = ctx.time();
            if t >= second {
                first * (t - second)
            } else {
                0.0
            }
        }
        Builtin::Delay1 | Builtin::Delay3 | Builtin::Smooth => ctx.state(site),
        _ => unreachable!("{builtin:?} is not a two-argument builtin"),
    }
}

fn eval3(builtin: Builtin, first: f64, second: f64, third: f64) -> f64 {
    match builtin {
        Builtin::Clamp => cetus_math::clamp(first, second, third),
        _ => unreachable!("{builtin:?} is not a three-argument builtin"),
    }
}

/// Compiles a typed expression.
#[must_use]
pub fn compile(typed: &TypedExpr) -> CompiledExpr {
    let mut code = Vec::new();
    emit(typed, &mut code);
    CompiledExpr {
        code,
        ty: typed.ty.clone(),
    }
}

fn emit(typed: &TypedExpr, code: &mut Vec<Instr>) {
    match &typed.kind {
        TypedKind::Const(value) => code.push(Instr::Const(*value)),
        TypedKind::Load(id) => code.push(Instr::Load(*id)),
        TypedKind::Time => code.push(Instr::Time),
        TypedKind::Dt => code.push(Instr::Dt),
        TypedKind::Unary(op, operand) => {
            emit(operand, code);
            code.push(match op {
                UnOp::Neg => Instr::Neg,
                UnOp::Not => Instr::Not,
            });
        }
        TypedKind::Binary(op, lhs, rhs) => {
            emit(lhs, code);
            emit(rhs, code);
            code.push(match op {
                BinOp::Add => Instr::Add,
                BinOp::Sub => Instr::Sub,
                BinOp::Mul => Instr::Mul,
                BinOp::Div => Instr::Div,
                BinOp::Pow => Instr::Pow,
                BinOp::Lt => Instr::Lt,
                BinOp::Le => Instr::Le,
                BinOp::Gt => Instr::Gt,
                BinOp::Ge => Instr::Ge,
                BinOp::Eq => Instr::Eq,
                BinOp::Ne => Instr::Ne,
                BinOp::And => Instr::And,
                BinOp::Or => Instr::Or,
            });
        }
        TypedKind::If(cond, then, otherwise) => {
            emit(cond, code);
            emit(then, code);
            emit(otherwise, code);
            code.push(Instr::Select);
        }
        TypedKind::Call(builtin, args) => {
            for arg in args {
                emit(arg, code);
            }
            let site = u32::try_from(code.len()).unwrap_or(u32::MAX);
            code.push(match args.len() {
                1 => Instr::Call1(*builtin),
                2 => Instr::Call2(*builtin, site),
                3 => Instr::Call3(*builtin),
                _ => unreachable!("builtin arity checked during type checking"),
            });
        }
        TypedKind::Lookup(table, x) => {
            emit(x, code);
            code.push(Instr::Lookup(*table));
        }
    }
}

/// A simple evaluation context backed by a slot vector.
#[derive(Debug, Clone, Default)]
pub struct SlotContext {
    slots: Vec<f64>,
    time: f64,
    dt: f64,
}

impl SlotContext {
    /// Builds a context with `count` zeroed slots.
    #[must_use]
    pub fn new(count: usize) -> Self {
        Self {
            slots: vec![0.0; count],
            time: 0.0,
            dt: 0.0,
        }
    }

    /// Sets a slot value.
    pub fn set(&mut self, id: u32, value: f64) {
        if let Some(slot) = self.slots.get_mut(id as usize) {
            *slot = value;
        }
    }

    /// Sets the current time.
    pub fn set_time(&mut self, time: f64) {
        self.time = time;
    }

    /// Sets the macro step.
    pub fn set_dt(&mut self, dt: f64) {
        self.dt = dt;
    }
}

impl EvalContext for SlotContext {
    fn var(&self, id: u32) -> f64 {
        self.slots.get(id as usize).copied().unwrap_or(0.0)
    }

    fn time(&self) -> f64 {
        self.time
    }

    fn dt(&self) -> f64 {
        self.dt
    }

    fn state(&self, _id: u32) -> f64 {
        0.0
    }

    fn lookup(&self, _table: u32, _x: f64) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typecheck::{NameResolver, Resolved, Type as ExprType, parse_and_check};
    use cetus_units::{Dim, UnitRegistry};

    struct Resolver {
        dims: Vec<(&'static str, u32, Dim)>,
    }

    impl NameResolver for Resolver {
        fn resolve(&self, name: &str) -> Option<Resolved> {
            self.dims
                .iter()
                .find(|(candidate, _, _)| *candidate == name)
                .map(|(_, id, dim)| Resolved::Variable {
                    id: *id,
                    ty: ExprType::scalar(dim.clone()),
                })
        }
    }

    #[test]
    fn evaluates_arithmetic() {
        let registry = UnitRegistry::new();
        let resolver = Resolver { dims: vec![] };
        let typed = parse_and_check("2 + 3 * 4", &resolver, &registry).expect("check");
        let compiled = compile(&typed);
        assert!((compiled.eval(&SlotContext::new(0)) - 14.0).abs() < f64::EPSILON);
    }

    #[test]
    fn evaluates_variables() {
        let registry = UnitRegistry::new();
        let resolver = Resolver {
            dims: vec![("x", 0, Dim::dimensionless())],
        };
        let typed = parse_and_check("x * 2", &resolver, &registry).expect("check");
        let compiled = compile(&typed);
        let mut ctx = SlotContext::new(1);
        ctx.set(0, 1.5);
        assert!((compiled.eval(&ctx) - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn evaluates_if() {
        let registry = UnitRegistry::new();
        let resolver = Resolver {
            dims: vec![("x", 0, Dim::dimensionless())],
        };
        let typed =
            parse_and_check("if x > 0 then x else -x", &resolver, &registry).expect("check");
        let compiled = compile(&typed);
        let mut ctx = SlotContext::new(1);
        ctx.set(0, -3.0);
        assert!((compiled.eval(&ctx) - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn step_depends_on_time() {
        let registry = UnitRegistry::new();
        let resolver = Resolver { dims: vec![] };
        let typed = parse_and_check("step(5, 2[s])", &resolver, &registry).expect("check");
        let compiled = compile(&typed);
        let mut ctx = SlotContext::new(0);
        ctx.set_time(1.0);
        assert_eq!(compiled.eval(&ctx), 0.0);
        ctx.set_time(2.5);
        assert_eq!(compiled.eval(&ctx), 5.0);
    }
}
