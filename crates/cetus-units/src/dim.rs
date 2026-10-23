//! Dimension vectors.
//!
//! A dimension is a vector of rational exponents over the seven SI base
//! dimensions plus any number of content-registered custom base dimensions
//! (`person`, `currency`, `bit`, `item`, ...). P5 requires every quantity to
//! carry a dimension; `Dim::dimensionless()` is written as `"1"` in model
//! files.

use core::fmt;

use indexmap::IndexMap;

use crate::rational::Rational;

/// The seven SI base dimensions, in the order used by [`Dim::to_exponents`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BaseDim {
    /// Length.
    Length,
    /// Mass.
    Mass,
    /// Time.
    Time,
    /// Electric current.
    Current,
    /// Thermodynamic temperature.
    Temperature,
    /// Amount of substance.
    Amount,
    /// Luminous intensity.
    Luminous,
}

impl BaseDim {
    /// All seven base dimensions in canonical order.
    pub const ALL: [Self; 7] = [
        Self::Length,
        Self::Mass,
        Self::Time,
        Self::Current,
        Self::Temperature,
        Self::Amount,
        Self::Luminous,
    ];

    /// The canonical symbol (`L`, `M`, `T`, `I`, `Θ`, `N`, `J`).
    #[must_use]
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Length => "L",
            Self::Mass => "M",
            Self::Time => "T",
            Self::Current => "I",
            Self::Temperature => "Θ",
            Self::Amount => "N",
            Self::Luminous => "J",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Length => 0,
            Self::Mass => 1,
            Self::Time => 2,
            Self::Current => 3,
            Self::Temperature => 4,
            Self::Amount => 5,
            Self::Luminous => 6,
        }
    }
}

/// A dimension: rational exponents over base dimensions.
///
/// Custom dimensions use a sorted map so that equality and display never depend
/// on insertion order (determinism rule D1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dim {
    base: [Rational; 7],
    custom: IndexMap<String, Rational>,
}

impl Dim {
    /// The dimensionless quantity, written `"1"` in model files.
    #[must_use]
    pub fn dimensionless() -> Self {
        Self::default()
    }

    /// A single base dimension with exponent one.
    #[must_use]
    pub fn base(base: BaseDim) -> Self {
        let mut dim = Self::dimensionless();
        dim.base[base.index()] = Rational::ONE;
        dim
    }

    /// A content-registered custom base dimension with exponent one.
    #[must_use]
    pub fn custom(name: impl Into<String>) -> Self {
        let mut dim = Self::dimensionless();
        dim.custom.insert(name.into(), Rational::ONE);
        dim
    }

    /// Exponent of a base dimension.
    #[must_use]
    pub fn exponent_of(&self, base: BaseDim) -> Rational {
        self.base[base.index()]
    }

    /// Exponent of a custom dimension, if present.
    #[must_use]
    pub fn custom_exponent(&self, name: &str) -> Option<Rational> {
        self.custom.get(name).copied()
    }

    /// Whether this is the dimensionless quantity.
    #[must_use]
    pub fn is_dimensionless(&self) -> bool {
        self.base.iter().all(|e| *e == Rational::ZERO) && self.custom.is_empty()
    }

    /// `self ^ exponent`.
    #[must_use]
    pub fn pow(&self, exponent: Rational) -> Self {
        let mut out = Self::dimensionless();
        for (slot, value) in out.base.iter_mut().zip(self.base.iter()) {
            *slot = *value * exponent;
        }
        for (name, value) in &self.custom {
            out.custom.insert(name.clone(), *value * exponent);
        }
        out
    }

    fn combine(&self, rhs: &Self, subtract: bool) -> Self {
        let mut out = Self::dimensionless();
        for ((slot, left), right) in out
            .base
            .iter_mut()
            .zip(self.base.iter())
            .zip(rhs.base.iter())
        {
            *slot = if subtract {
                *left - *right
            } else {
                *left + *right
            };
        }
        for (name, value) in &self.custom {
            out.custom.insert(name.clone(), *value);
        }
        for (name, value) in &rhs.custom {
            let updated = match out.custom.get(name).copied() {
                Some(existing) => {
                    if subtract {
                        existing - *value
                    } else {
                        existing + *value
                    }
                }
                None => {
                    if subtract {
                        -*value
                    } else {
                        *value
                    }
                }
            };
            out.custom.insert(name.clone(), updated);
        }
        out.custom.retain(|_, exp| *exp != Rational::ZERO);
        out
    }

    /// Canonical exponent list, including custom dimensions sorted by name.
    #[must_use]
    pub fn to_exponents(&self) -> Vec<(String, Rational)> {
        let mut out: Vec<(String, Rational)> = BaseDim::ALL
            .iter()
            .map(|base| (base.symbol().to_owned(), self.exponent_of(*base)))
            .filter(|(_, exp)| *exp != Rational::ZERO)
            .collect();
        out.extend(self.custom.iter().map(|(name, exp)| (name.clone(), *exp)));
        out
    }

    /// Parses a canonical dimension string such as `"L^2 T^-1"` or `"1"`.
    ///
    /// Only the canonical symbols (`L M T I Θ N J`) and registered custom
    /// dimension names are accepted. Model files normally use unit expressions
    /// instead; this is for diagnostics and tests.
    #[must_use]
    pub fn parse_canonical(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() || trimmed == "1" {
            return Some(Self::dimensionless());
        }
        let mut dim = Self::dimensionless();
        for token in trimmed.split_whitespace() {
            let (name, exp) = match token.split_once('^') {
                Some((name, exp)) => (name, parse_exponent(exp)?),
                None => (token, Rational::ONE),
            };
            let base = BaseDim::ALL
                .iter()
                .copied()
                .find(|base| base.symbol() == name);
            if let Some(base) = base {
                dim.base[base.index()] = dim.exponent_of(base) + exp;
            } else {
                let existing = dim.custom.get(name).copied().unwrap_or(Rational::ZERO);
                dim.custom.insert(name.to_owned(), existing + exp);
            }
        }
        dim.custom.retain(|_, exp| *exp != Rational::ZERO);
        Some(dim)
    }
}

fn parse_exponent(input: &str) -> Option<Rational> {
    if let Some((num, den)) = input.split_once('/') {
        Some(Rational::new(num.parse().ok()?, den.parse().ok()?))
    } else {
        Some(Rational::from_int(input.parse().ok()?))
    }
}

impl<'a> core::ops::Mul<&'a Dim> for &'a Dim {
    type Output = Dim;

    fn mul(self, rhs: &'a Dim) -> Dim {
        self.combine(rhs, false)
    }
}

impl<'a> core::ops::Div<&'a Dim> for &'a Dim {
    type Output = Dim;

    fn div(self, rhs: &'a Dim) -> Dim {
        self.combine(rhs, true)
    }
}

impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dimensionless() {
            return f.write_str("1");
        }
        let mut first = true;
        for (name, exp) in self.to_exponents() {
            if !first {
                f.write_str(" ")?;
            }
            first = false;
            if exp == Rational::ONE {
                f.write_str(&name)?;
            } else {
                write!(f, "{name}^{exp}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_and_div_cancel() {
        let l = Dim::base(BaseDim::Length);
        let t = Dim::base(BaseDim::Time);
        let speed = &l / &t;
        assert_eq!(&speed * &t, l);
        assert!((&speed / &speed).is_dimensionless());
    }

    #[test]
    fn rational_powers_are_exact() {
        let l2 = Dim::base(BaseDim::Length).pow(Rational::from_int(2));
        let root = l2.pow(Rational::new(1, 2));
        assert_eq!(root, Dim::base(BaseDim::Length));
    }

    #[test]
    fn custom_dimensions_do_not_mix_with_dimensionless() {
        let people = Dim::custom("person");
        assert!(!people.is_dimensionless());
        assert_ne!(people, Dim::dimensionless());
    }

    #[test]
    fn display_roundtrip() {
        let dim =
            &Dim::base(BaseDim::Length).pow(Rational::from_int(2)) / &Dim::base(BaseDim::Time);
        assert_eq!(dim.to_string(), "L^2 T^-1");
        assert_eq!(Dim::parse_canonical(&dim.to_string()), Some(dim));
    }
}
