//! Units: a dimension plus an affine conversion to SI.
//!
//! Internal simulation state is always SI base units. Units appear only at
//! boundaries: parsing model files, formatting diagnostics, and displaying
//! values to the player (P5, K14).

use core::fmt;

use crate::dim::Dim;

/// A unit of measure.
///
/// Conversion: `si = value * factor + offset`. `offset` is non-zero only for
/// affine units such as `°C` and `°F`, which may not appear inside compound
/// expressions.
/// One factor in the canonical display form of a unit.
#[derive(Debug, Clone, PartialEq)]
struct Term {
    symbol: String,
    exponent: crate::rational::Rational,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    symbol: String,
    dim: Dim,
    factor: f64,
    offset: f64,
    angle: bool,
    terms: Vec<Term>,
}

impl Unit {
    /// Creates a linear unit.
    ///
    /// # Panics
    /// Panics if `factor` is zero, NaN or infinite: a unit with a zero factor
    /// cannot be converted back to SI.
    #[must_use]
    pub fn linear(symbol: impl Into<String>, dim: Dim, factor: f64) -> Self {
        assert!(
            factor.is_finite() && factor != 0.0,
            "unit factor must be finite and non-zero"
        );
        let symbol = symbol.into();
        let terms = vec![Term {
            symbol: symbol.clone(),
            exponent: crate::rational::Rational::ONE,
        }];
        Self {
            symbol,
            dim,
            factor,
            offset: 0.0,
            angle: false,
            terms,
        }
    }

    /// Creates an affine unit such as `°C`.
    ///
    /// # Panics
    /// Panics if `factor` is zero, NaN or infinite.
    #[must_use]
    pub fn affine(symbol: impl Into<String>, dim: Dim, factor: f64, offset: f64) -> Self {
        assert!(
            factor.is_finite() && factor != 0.0,
            "unit factor must be finite and non-zero"
        );
        assert!(offset.is_finite(), "unit offset must be finite");
        let symbol = symbol.into();
        let terms = vec![Term {
            symbol: symbol.clone(),
            exponent: crate::rational::Rational::ONE,
        }];
        Self {
            symbol,
            dim,
            factor,
            offset,
            angle: false,
            terms,
        }
    }

    /// Marks this unit as carrying the `angle` label (K14).
    #[must_use]
    pub fn with_angle(mut self, angle: bool) -> Self {
        self.angle = angle;
        self
    }

    /// The dimensionless unit `"1"`.
    #[must_use]
    pub fn one() -> Self {
        Self::linear("1", Dim::dimensionless(), 1.0)
    }

    /// The canonical symbol as written in the model file.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// The dimension.
    #[must_use]
    pub fn dim(&self) -> &Dim {
        &self.dim
    }

    /// Multiplicative factor to SI.
    #[must_use]
    pub fn factor(&self) -> f64 {
        self.factor
    }

    /// Additive offset to SI (zero for linear units).
    #[must_use]
    pub fn offset(&self) -> f64 {
        self.offset
    }

    /// Whether this unit carries the `angle` label.
    #[must_use]
    pub fn is_angle(&self) -> bool {
        self.angle
    }

    /// Whether this unit is affine (`°C`, `°F`).
    #[must_use]
    pub fn is_affine(&self) -> bool {
        self.offset != 0.0
    }

    /// Converts a value in this unit to SI.
    #[must_use]
    pub fn to_si(&self, value: f64) -> f64 {
        value * self.factor + self.offset
    }

    /// Converts a value in SI to this unit.
    #[must_use]
    pub fn from_si(&self, value: f64) -> f64 {
        (value - self.offset) / self.factor
    }

    /// Whether two units measure the same dimension.
    #[must_use]
    pub fn compatible_with(&self, other: &Self) -> bool {
        self.dim == other.dim
    }

    /// `self * rhs`.
    ///
    /// # Errors
    /// Returns [`UnitError::AffineInCompound`] if either unit is affine.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, UnitError> {
        self.require_linear()?;
        rhs.require_linear()?;
        let terms = combine_terms(&self.terms, &rhs.terms, false);
        let mut unit = Self::from_terms(terms, &self.dim * &rhs.dim, self.factor * rhs.factor);
        unit.angle = self.angle && rhs.angle;
        Ok(unit)
    }

    /// `self / rhs`.
    ///
    /// # Errors
    /// Returns [`UnitError::AffineInCompound`] if either unit is affine.
    pub fn try_div(&self, rhs: &Self) -> Result<Self, UnitError> {
        self.require_linear()?;
        rhs.require_linear()?;
        let terms = combine_terms(&self.terms, &rhs.terms, true);
        Ok(Self::from_terms(
            terms,
            &self.dim / &rhs.dim,
            self.factor / rhs.factor,
        ))
    }

    /// `self ^ exponent`, where the exponent is a compile-time rational (P5).
    ///
    /// # Errors
    /// Returns [`UnitError::AffineInCompound`] for affine units.
    pub fn try_pow(&self, exponent: crate::rational::Rational) -> Result<Self, UnitError> {
        self.require_linear()?;
        let terms: Vec<Term> = self
            .terms
            .iter()
            .map(|term| Term {
                symbol: term.symbol.clone(),
                exponent: term.exponent * exponent,
            })
            .filter(|term| term.exponent != crate::rational::Rational::ZERO)
            .collect();
        let factor = match exponent.as_integer() {
            Some(e) => cetus_math::powi(self.factor, e),
            None => cetus_math::powf(self.factor, exponent.as_f64()),
        };
        Ok(Self::from_terms(terms, self.dim.pow(exponent), factor))
    }

    fn require_linear(&self) -> Result<(), UnitError> {
        if self.is_affine() {
            Err(UnitError::AffineInCompound {
                unit: self.symbol.clone(),
            })
        } else {
            Ok(())
        }
    }
}

impl Unit {
    fn from_terms(terms: Vec<Term>, dim: Dim, factor: f64) -> Self {
        let symbol = render_terms(&terms);
        Self {
            symbol,
            dim,
            factor,
            offset: 0.0,
            angle: false,
            terms,
        }
    }
}

fn combine_terms(lhs: &[Term], rhs: &[Term], negate_rhs: bool) -> Vec<Term> {
    let mut out: Vec<Term> = Vec::with_capacity(lhs.len() + rhs.len());
    for term in lhs {
        push_term(&mut out, &term.symbol, term.exponent);
    }
    for term in rhs {
        let exponent = if negate_rhs {
            -term.exponent
        } else {
            term.exponent
        };
        push_term(&mut out, &term.symbol, exponent);
    }
    out
}

fn push_term(out: &mut Vec<Term>, symbol: &str, exponent: crate::rational::Rational) {
    if exponent == crate::rational::Rational::ZERO {
        return;
    }
    if let Some(existing) = out.iter_mut().find(|term| term.symbol == symbol) {
        existing.exponent = existing.exponent + exponent;
        if existing.exponent == crate::rational::Rational::ZERO {
            out.retain(|term| term.symbol != symbol);
        }
        return;
    }
    out.push(Term {
        symbol: symbol.to_owned(),
        exponent,
    });
}

fn render_terms(terms: &[Term]) -> String {
    if terms.is_empty() {
        return "1".to_owned();
    }
    let mut out = String::new();
    for (index, term) in terms.iter().enumerate() {
        let positive = term.exponent > crate::rational::Rational::ZERO;
        let magnitude = if positive {
            term.exponent
        } else {
            -term.exponent
        };
        if index == 0 {
            if !positive {
                out.push_str("1/");
            }
        } else if positive {
            out.push('*');
        } else {
            out.push('/');
        }
        out.push_str(&term.symbol);
        if magnitude != crate::rational::Rational::ONE {
            out.push('^');
            out.push_str(&magnitude.to_string());
        }
    }
    out
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.symbol)
    }
}

/// Errors produced while parsing or combining units.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnitError {
    /// The unit name is not registered.
    #[error("unknown unit `{name}`")]
    UnknownUnit {
        /// The unresolved symbol.
        name: String,
    },
    /// The expression ended unexpectedly.
    #[error("unexpected end of unit expression")]
    UnexpectedEnd,
    /// An unexpected character appeared.
    #[error("unexpected character `{ch}` at byte {offset}")]
    UnexpectedChar {
        /// The offending character.
        ch: char,
        /// Byte offset in the source.
        offset: usize,
    },
    /// The expression was empty.
    #[error("empty unit expression")]
    EmptyExpression,
    /// A factor was missing where one is required.
    #[error("expected a unit or `(` at byte {offset}")]
    ExpectedUnit {
        /// Byte offset in the source.
        offset: usize,
    },
    /// The exponent could not be parsed.
    #[error("bad exponent `{text}`")]
    BadExponent {
        /// The offending text.
        text: String,
    },
    /// An affine unit appeared inside a compound expression.
    #[error("affine unit `{unit}` cannot be used inside a compound unit expression")]
    AffineInCompound {
        /// The offending unit.
        unit: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dim::BaseDim;
    use crate::rational::Rational;

    #[test]
    fn linear_conversion_roundtrips() {
        let km = Unit::linear("km", Dim::base(BaseDim::Length), 1000.0);
        assert_eq!(km.to_si(2.0), 2000.0);
        assert_eq!(km.from_si(2000.0), 2.0);
    }

    #[test]
    fn affine_conversion() {
        let celsius = Unit::affine("°C", Dim::base(BaseDim::Temperature), 1.0, 273.15);
        assert_eq!(celsius.to_si(0.0), 273.15);
        assert!((celsius.from_si(273.15)).abs() < 1e-12);
    }

    #[test]
    fn compound_units() {
        let m = Unit::linear("m", Dim::base(BaseDim::Length), 1.0);
        let s = Unit::linear("s", Dim::base(BaseDim::Time), 1.0);
        let speed = m.try_div(&s).expect("linear units combine");
        assert_eq!(
            speed.dim(),
            &(&Dim::base(BaseDim::Length) / &Dim::base(BaseDim::Time))
        );
        assert_eq!(speed.to_si(3.0), 3.0);
        let squared = m.try_pow(Rational::from_int(2)).expect("integer power");
        assert_eq!(
            squared.dim(),
            &Dim::base(BaseDim::Length).pow(Rational::from_int(2))
        );
    }

    #[test]
    fn affine_units_cannot_compound() {
        let celsius = Unit::affine("°C", Dim::base(BaseDim::Temperature), 1.0, 273.15);
        let s = Unit::linear("s", Dim::base(BaseDim::Time), 1.0);
        assert!(celsius.try_div(&s).is_err());
    }
}
