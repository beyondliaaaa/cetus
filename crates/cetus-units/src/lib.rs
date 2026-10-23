//! Physical dimensions, units and dimensional analysis (P5, K14).
//!
//! Every quantity in the simulation carries a dimension. Values are stored in SI
//! base units; [`Unit`] converts at the boundaries (model files, diagnostics,
//! player-facing display).
//!
//! ```
//! use cetus_units::{Dim, UnitRegistry};
//!
//! let registry = UnitRegistry::new();
//! let unit = registry.parse("kW*h").expect("valid unit");
//! assert_eq!(unit.factor(), 3.6e6);
//! assert!(!unit.dim().is_dimensionless());
//! # let _ = Dim::dimensionless();
//! ```

#![forbid(unsafe_code)]

mod dim;
mod rational;
mod registry;
mod unit;

pub use dim::{BaseDim, Dim};
pub use rational::Rational;
pub use registry::UnitRegistry;
pub use unit::{Unit, UnitError};

/// The SI second as a [`Unit`]; convenience for `time.quantum`.
#[must_use]
pub fn second() -> Unit {
    Unit::linear("s", Dim::base(BaseDim::Time), 1.0)
}
