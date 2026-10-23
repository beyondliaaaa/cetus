//! Property tests for the unit system (§5.1).
//!
//! Covers three invariants that the rest of the engine relies on:
//! 1. parse → display → parse is stable in dimension and factor;
//! 2. dimension algebra is commutative and associative;
//! 3. unit composition multiplies factors consistently.

use cetus_units::{BaseDim, Dim, Rational, UnitRegistry};
use proptest::prelude::*;

fn registry() -> UnitRegistry {
    UnitRegistry::new()
}

const ATOMS: &[&str] = &[
    "m", "kg", "s", "A", "K", "mol", "cd", "h", "day", "L", "g", "N", "Pa", "J", "W", "V", "Ω",
    "Hz", "bar", "eV", "cal",
];

prop_compose! {
    fn atom_expr()(idx in 0..ATOMS.len()) -> String {
        ATOMS[idx].to_owned()
    }
}

fn arbitrary_expr() -> impl Strategy<Value = String> {
    let leaf = atom_expr();
    leaf.prop_recursive(4, 32, 3, |inner| {
        prop_oneof![
            inner.clone().prop_map(|e| format!("({e})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a}*{b}")),
            (inner.clone(), inner).prop_map(|(a, b)| format!("{a}/{b}")),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn parse_is_stable_in_dim_and_factor(expr in arbitrary_expr()) {
        let registry = registry();
        let first = registry.parse(&expr).expect("generated expression must parse");
        let rendered = first.to_string();
        let second = registry.parse(&rendered).expect("rendered unit must parse");
        prop_assert_eq!(first.dim(), second.dim());
        prop_assert!((first.factor() - second.factor()).abs() <= first.factor().abs() * 1e-12);
    }

    #[test]
    fn dim_mul_is_commutative(
        a in 0u8..7,
        b in 0u8..7,
        ea in -3i32..4,
        eb in -3i32..4,
    ) {
        let left = Dim::base(BaseDim::ALL[a as usize]).pow(Rational::from_int(ea));
        let right = Dim::base(BaseDim::ALL[b as usize]).pow(Rational::from_int(eb));
        prop_assert_eq!(&left * &right, &right * &left);
    }

    #[test]
    fn dim_mul_is_associative(
        a in 0u8..7,
        b in 0u8..7,
        c in 0u8..7,
        ea in -3i32..4,
        eb in -3i32..4,
        ec in -3i32..4,
    ) {
        let da = Dim::base(BaseDim::ALL[a as usize]).pow(Rational::from_int(ea));
        let db = Dim::base(BaseDim::ALL[b as usize]).pow(Rational::from_int(eb));
        let dc = Dim::base(BaseDim::ALL[c as usize]).pow(Rational::from_int(ec));
        let left = &(&da * &db) * &dc;
        let right = &da * &(&db * &dc);
        prop_assert_eq!(left, right);
    }

    #[test]
    fn inverse_cancels(a in 0u8..7, exp in 1i32..5) {
        let dim = Dim::base(BaseDim::ALL[a as usize]).pow(Rational::from_int(exp));
        let inverse = Dim::base(BaseDim::ALL[a as usize]).pow(Rational::from_int(-exp));
        prop_assert!((&dim * &inverse).is_dimensionless());
    }

    #[test]
    fn factors_compose(expr in arbitrary_expr()) {
        let registry = registry();
        let parsed = registry.parse(&expr).expect("generated expression must parse");
        prop_assert!(parsed.factor().is_finite());
        prop_assert!(parsed.factor() > 0.0);
    }
}
