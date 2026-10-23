//! Exact rational exponents.
//!
//! Dimension exponents are rational numbers: `m^(1/2)` must be expressible, and
//! exponent arithmetic (`(L^2)^(1/2) = L`) must be exact. `f64` would make
//! dimension equality unreliable, so exponents are stored as reduced `i32`
//! fractions.

use core::cmp::Ordering;
use core::fmt;

/// A reduced fraction with a positive denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    num: i32,
    den: i32,
}

impl Default for Rational {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Rational {
    /// Zero.
    pub const ZERO: Self = Self { num: 0, den: 1 };
    /// One.
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// Builds a reduced fraction.
    ///
    /// # Panics
    /// Panics if `den == 0`.
    #[must_use]
    pub fn new(num: i32, den: i32) -> Self {
        assert!(den != 0, "denominator must not be zero");
        let sign = if den < 0 { -1 } else { 1 };
        let g = gcd(num.unsigned_abs(), den.unsigned_abs());
        let g = if g == 0 { 1 } else { g };
        Self {
            num: sign * (num / g.cast_signed()),
            den: (den / g.cast_signed()).abs(),
        }
    }

    /// numerator / denominator.
    #[must_use]
    pub fn from_int(value: i32) -> Self {
        Self { num: value, den: 1 }
    }

    /// Numerator.
    #[must_use]
    pub fn num(self) -> i32 {
        self.num
    }

    /// Denominator (always positive).
    #[must_use]
    pub fn den(self) -> i32 {
        self.den
    }

    /// `true` if the value is an integer.
    #[must_use]
    pub fn is_integer(self) -> bool {
        self.den == 1
    }

    /// Returns the integer value, or `None` if not an integer.
    #[must_use]
    pub fn as_integer(self) -> Option<i32> {
        self.is_integer().then_some(self.num)
    }

    /// `self ^ exponent`, where `exponent` must be an integer.
    /// `self ^ exponent`, where `exponent` must be an integer.
    ///
    /// # Panics
    /// Panics if `exponent` is not an integer.
    #[must_use]
    pub fn pow_int(self, exponent: i32) -> Self {
        if exponent == 0 {
            return Self::ONE;
        }
        let n = exponent.unsigned_abs();
        let num = self.num.checked_pow(n).unwrap_or(i32::MAX);
        let den = self.den.checked_pow(n).unwrap_or(i32::MAX);
        if exponent < 0 {
            Self::new(den, num)
        } else {
            Self::new(num, den)
        }
    }

    /// Converts to `f64` (used only for display and tolerance inference).
    #[must_use]
    pub fn as_f64(self) -> f64 {
        f64::from(self.num) / f64::from(self.den)
    }
}

impl core::ops::Neg for Rational {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            num: self.num.checked_neg().unwrap_or(i32::MAX),
            den: self.den,
        }
    }
}

impl core::ops::Add for Rational {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let num = self
            .num
            .checked_mul(rhs.den)
            .and_then(|a| rhs.num.checked_mul(self.den).and_then(|b| a.checked_add(b)))
            .unwrap_or(i32::MAX);
        let den = self.den.checked_mul(rhs.den).unwrap_or(i32::MAX);
        Self::new(num, den)
    }
}

impl core::ops::Sub for Rational {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

impl core::ops::Mul for Rational {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let num = self.num.checked_mul(rhs.num).unwrap_or(i32::MAX);
        let den = self.den.checked_mul(rhs.den).unwrap_or(i32::MAX);
        Self::new(num, den)
    }
}

impl core::ops::Div for Rational {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        assert!(rhs.num != 0, "division by zero rational");
        let num = self.num.checked_mul(rhs.den).unwrap_or(i32::MAX);
        let den = self.den.checked_mul(rhs.num).unwrap_or(i32::MAX);
        Self::new(num, den)
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        self.num
            .saturating_mul(other.den)
            .cmp(&other.num.saturating_mul(self.den))
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduces() {
        assert_eq!(Rational::new(4, 8), Rational::new(1, 2));
        assert_eq!(Rational::new(-4, 8), Rational::new(-1, 2));
        assert_eq!(Rational::new(4, -8), Rational::new(-1, 2));
    }

    #[test]
    fn arithmetic_is_exact() {
        assert_eq!(Rational::new(1, 2) + Rational::new(1, 2), Rational::ONE);
        assert_eq!(Rational::new(2, 3) * Rational::new(3, 2), Rational::ONE);
        assert_eq!(Rational::new(2, 3) / Rational::new(2, 3), Rational::ONE);
        assert_eq!(Rational::new(4, 9).pow_int(2), Rational::new(16, 81));
    }

    #[test]
    fn integer_detection() {
        assert!(Rational::from_int(3).is_integer());
        assert!(!Rational::new(3, 2).is_integer());
        assert_eq!(Rational::new(6, 3).as_integer(), Some(2));
    }
}
