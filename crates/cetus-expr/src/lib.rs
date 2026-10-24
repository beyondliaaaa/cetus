//! The Cetus expression language (§5.2).
//!
//! A small, strict language shared by every paradigm. Pipeline:
//!
//! ```text
//! source ─parse─▶ AST ─typecheck─▶ TypedExpr ─compile─▶ CompiledExpr ─eval─▶ f64
//! ```
//!
//! Dimensions are checked at every step (P5). Transcendental functions reject
//! dimensioned arguments. Evaluation is a deterministic stack machine; all
//! transcendental functions go through [`cetus_math`] (D2).
//!
//! ```
//! use cetus_expr::{NameResolver, Resolved, compile, parse_and_check};
//! use cetus_units::{Dim, UnitRegistry};
//!
//! struct NoNames;
//! impl NameResolver for NoNames {
//!     fn resolve(&self, _name: &str) -> Option<Resolved> { None }
//! }
//!
//! let registry = UnitRegistry::new();
//! let typed = parse_and_check("2[m] + 3[m]", &NoNames, &registry).expect("type-checks");
//! let compiled = compile(&typed);
//! assert_eq!(compiled.ty().dim(), Some(&Dim::base(cetus_units::BaseDim::Length)));
//! ```

#![forbid(unsafe_code)]

pub mod equiv;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod typecheck;

pub use equiv::{
    DomainVar, EquivOptions, EquivOutcome, TableLookup, equivalent, equivalent_with_tables,
};
pub use eval::{CompiledExpr, EvalContext, Instr, SlotContext, compile};
pub use parser::{BinOp, Expr, ExprKind, ParseError, Span, UnOp, parse};
pub use typecheck::{
    AgentTypeId, Builtin, DatasetSchemaId, EnumId, GeometryKind, GraphId, GridId, NameResolver,
    Resolved, TableSignature, Type, TypeError, TypedExpr, TypedKind, parse_and_check, typecheck,
};

/// Errors from the expression pipeline.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExprError {
    /// Lexer or parser error.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Type or dimension error.
    #[error(transparent)]
    Type(#[from] TypeError),
}

impl ExprError {
    /// The diagnostic code (`E...`).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Parse(_) => "E0003",
            Self::Type(error) => error.code,
        }
    }

    /// Byte span of the error.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Parse(error) => Span::new(error.start, error.end),
            Self::Type(error) => Span::new(error.start, error.end),
        }
    }

    /// Human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Parse(error) => &error.message,
            Self::Type(error) => &error.message,
        }
    }
}
