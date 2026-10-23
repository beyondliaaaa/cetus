//! Deterministic math wrappers for Cetus.
//!
//! Every transcendental function used by the simulation kernel must go through
//! this crate (determinism rule D2, §5.12). Implementations delegate to [`libm`],
//! a pure-Rust implementation with bit-identical results on `x86_64`, `aarch64`
//! and `wasm32`.
//!
//! Do **not** call `f64::sin` and friends directly; `clippy.toml` rejects them.

#![forbid(unsafe_code)]

/// `sin(x)` — deterministic across targets.
#[must_use]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

/// `cos(x)` — deterministic across targets.
#[must_use]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

/// `tan(x)` — deterministic across targets.
#[must_use]
pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

/// `asin(x)` — deterministic across targets.
#[must_use]
pub fn asin(x: f64) -> f64 {
    libm::asin(x)
}

/// `acos(x)` — deterministic across targets.
#[must_use]
pub fn acos(x: f64) -> f64 {
    libm::acos(x)
}

/// `atan(x)` — deterministic across targets.
#[must_use]
pub fn atan(x: f64) -> f64 {
    libm::atan(x)
}

/// `atan2(y, x)` — deterministic across targets.
#[must_use]
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// `sinh(x)` — deterministic across targets.
#[must_use]
pub fn sinh(x: f64) -> f64 {
    libm::sinh(x)
}

/// `cosh(x)` — deterministic across targets.
#[must_use]
pub fn cosh(x: f64) -> f64 {
    libm::cosh(x)
}

/// `tanh(x)` — deterministic across targets.
#[must_use]
pub fn tanh(x: f64) -> f64 {
    libm::tanh(x)
}

/// `e^x` — deterministic across targets.
#[must_use]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// `2^x` — deterministic across targets.
#[must_use]
pub fn exp2(x: f64) -> f64 {
    libm::exp2(x)
}

/// `e^x - 1`, accurate for small `x`.
#[must_use]
pub fn exp_m1(x: f64) -> f64 {
    libm::expm1(x)
}

/// Natural logarithm.
#[must_use]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

/// `ln(1 + x)`, accurate for small `x`.
#[must_use]
pub fn ln_1p(x: f64) -> f64 {
    libm::log1p(x)
}

/// Base-10 logarithm.
#[must_use]
pub fn log10(x: f64) -> f64 {
    libm::log10(x)
}

/// Base-2 logarithm.
#[must_use]
pub fn log2(x: f64) -> f64 {
    libm::log2(x)
}

/// `x^y` — deterministic across targets.
#[must_use]
pub fn powf(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

/// Integer power implemented as a deterministic binary exponentiation.
///
/// This deliberately does not call `f64::powi`, whose rounding is unspecified
/// for negative exponents. The implementation is fixed here so that every
/// platform produces the same bits.
#[must_use]
pub fn powi(mut base: f64, mut exp: i32) -> f64 {
    if exp == 0 {
        return 1.0;
    }
    if exp < 0 {
        base = 1.0 / base;
        exp = exp.checked_neg().unwrap_or(i32::MAX);
    }
    let mut acc = 1.0;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            acc *= base;
        }
        base *= base;
        e >>= 1;
    }
    acc
}

/// Square root (IEEE 754 correctly rounded, wrapped for rule uniformity).
#[must_use]
pub fn sqrt(x: f64) -> f64 {
    libm::sqrt(x)
}

/// Cube root.
#[must_use]
pub fn cbrt(x: f64) -> f64 {
    libm::cbrt(x)
}

/// `sqrt(x*x + y*y)` without intermediate overflow where possible.
#[must_use]
pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

/// `floor(x)`; included here so the expression language never reaches for
/// floating-point intrinsics directly.
#[must_use]
pub fn floor(x: f64) -> f64 {
    libm::floor(x)
}

/// `ceil(x)`.
#[must_use]
pub fn ceil(x: f64) -> f64 {
    libm::ceil(x)
}

/// `round(x)`, half away from zero (the rule fixed by the expression language).
#[must_use]
pub fn round(x: f64) -> f64 {
    libm::round(x)
}

/// `x` clamped to `[lo, hi]`.
///
/// # Panics
/// Panics if `lo > hi` or either bound is NaN. Callers in the kernel validate
/// bounds at compile time, so this is a programming error.
#[must_use]
pub fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    assert!(lo <= hi, "clamp bounds must satisfy lo <= hi");
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

/// Canonicalise a float for state hashing (rule D9): `-0.0` becomes `0.0` and
/// every NaN collapses to one bit pattern.
#[must_use]
pub fn canonical_bits(x: f64) -> u64 {
    if x == 0.0 {
        0u64
    } else if x.is_nan() {
        0x7ff8_0000_0000_0000
    } else {
        x.to_bits()
    }
}

/// A fixed battery of transcendental results, used by the M0 cross-target
/// determinism check and by regression tests.
///
/// The array is ordered so that index `8 * i + j` is function `j` applied to
/// input `i`; see [`HELLO_INPUTS`] and [`HELLO_FUNCTIONS`].
#[must_use]
pub fn transcendental_battery() -> [f64; 64] {
    let mut out = [0.0f64; 64];
    let mut i = 0;
    for &x in &HELLO_INPUTS {
        let values = [
            sin(x),
            cos(x),
            tan(x),
            exp(x),
            ln(x.abs().max(f64::MIN_POSITIVE)),
            sqrt(x.abs()),
            powi(x, 3),
            powf(x.abs(), 1.5),
        ];
        out[i..i + 8].copy_from_slice(&values);
        i += 8;
    }
    out
}

/// Names of the functions in [`transcendental_battery`], in order.
pub const HELLO_FUNCTIONS: [&str; 8] = ["sin", "cos", "tan", "exp", "ln", "sqrt", "powi", "powf"];

/// Inputs of [`transcendental_battery`], in order.
pub const HELLO_INPUTS: [f64; 8] = [0.0, 0.1, 0.5, 1.0, 2.0, -0.75, 3.25, 10.0];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powi_matches_repeated_multiplication() {
        assert!((powi(1.5, 3) - 3.375).abs() < f64::EPSILON);
        assert!((powi(2.0, -2) - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn canonical_bits_folds_negative_zero_and_nan() {
        assert_eq!(canonical_bits(-0.0), canonical_bits(0.0));
        assert_eq!(canonical_bits(f64::NAN), canonical_bits(-f64::NAN));
    }

    #[test]
    fn battery_is_deterministic() {
        assert_eq!(transcendental_battery(), transcendental_battery());
    }
}
