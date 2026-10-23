//! Numerical equivalence of expressions (§5.2).
//!
//! Deliberately not a symbolic simplifier: we sample the declared domain with a
//! fixed seed and compare relative error. This catches "same algebra, different
//! spelling" (`η*A*G` vs `G*A*η`) and returns a counterexample for the hint
//! ladder (§6.4).

use crate::eval::{CompiledExpr, EvalContext};
use crate::typecheck::Type;
use rand::RngExt as _;
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;

/// One variable in the sampling domain.
#[derive(Debug, Clone)]
pub struct DomainVar {
    /// Slot id.
    pub id: u32,
    /// Name used in counterexamples.
    pub name: String,
    /// Lower bound (inclusive, SI units).
    pub min: f64,
    /// Upper bound (inclusive, SI units).
    pub max: f64,
}

/// Options for [`equivalent`].
#[derive(Debug, Clone)]
pub struct EquivOptions {
    /// Number of samples (default 64).
    pub samples: usize,
    /// Relative tolerance (default `1e-9`).
    pub rel_tol: f64,
    /// Fixed seed so results are reproducible.
    pub seed: u64,
}

impl Default for EquivOptions {
    fn default() -> Self {
        Self {
            samples: 64,
            rel_tol: 1e-9,
            seed: 0x5eed_c3e5,
        }
    }
}

/// The result of an equivalence check.
#[derive(Debug, Clone, PartialEq)]
pub enum EquivOutcome {
    /// The two expressions agree on the sampled domain.
    Equivalent,
    /// They disagree; `example` is the sampled point with the largest error.
    Different {
        /// Variable assignments at the counterexample.
        example: Vec<(String, f64)>,
        /// Left-hand value.
        lhs: f64,
        /// Right-hand value.
        rhs: f64,
        /// Relative error at the counterexample.
        rel_error: f64,
    },
    /// The expressions have different dimensions and cannot be compared.
    TypeMismatch {
        /// Description of the mismatch.
        message: String,
    },
    /// A sample produced NaN or infinity in both expressions.
    Undefined {
        /// Description.
        message: String,
    },
}

type Counterexample = (Vec<(String, f64)>, f64, f64, f64);

struct SampleContext<'a> {
    values: &'a [(u32, f64)],
    time: f64,
}

impl EvalContext for SampleContext<'_> {
    fn var(&self, id: u32) -> f64 {
        self.values
            .iter()
            .find(|(slot, _)| *slot == id)
            .map_or(0.0, |(_, value)| *value)
    }

    fn time(&self) -> f64 {
        self.time
    }

    fn dt(&self) -> f64 {
        0.0
    }

    fn state(&self, _id: u32) -> f64 {
        0.0
    }

    fn lookup(&self, _table: u32, _x: f64) -> f64 {
        0.0
    }
}

/// Checks whether two compiled expressions agree on the sampled domain.
#[must_use]
pub fn equivalent(
    lhs: &CompiledExpr,
    rhs: &CompiledExpr,
    domain: &[DomainVar],
    options: &EquivOptions,
) -> EquivOutcome {
    let lhs_dim = lhs.ty().dim().cloned();
    let rhs_dim = rhs.ty().dim().cloned();
    if lhs_dim != rhs_dim {
        return EquivOutcome::TypeMismatch {
            message: format!(
                "dimensions differ: {} vs {}",
                describe_dim(lhs.ty()),
                describe_dim(rhs.ty())
            ),
        };
    }

    let mut rng = ChaCha8Rng::seed_from_u64(options.seed);
    let mut assignments: Vec<(u32, f64)> = Vec::with_capacity(domain.len());
    let mut worst: Option<Counterexample> = None;
    let mut all_undefined = true;

    for _ in 0..options.samples.max(1) {
        assignments.clear();
        for var in domain {
            let value = if var.max > var.min {
                rng.random_range(var.min..=var.max)
            } else {
                var.min
            };
            assignments.push((var.id, value));
        }
        let ctx = SampleContext {
            values: &assignments,
            time: 0.0,
        };
        let left = lhs.eval(&ctx);
        let right = rhs.eval(&ctx);
        if !left.is_finite() || !right.is_finite() {
            continue;
        }
        all_undefined = false;
        let scale = left.abs().max(right.abs()).max(1.0);
        let error = (left - right).abs() / scale;
        if error > options.rel_tol {
            let example = domain
                .iter()
                .zip(assignments.iter())
                .map(|(var, (_, value))| (var.name.clone(), *value))
                .collect::<Vec<_>>();
            let should_replace = worst.as_ref().is_none_or(|(_, _, _, best)| error > *best);
            if should_replace {
                worst = Some((example, left, right, error));
            }
        }
    }

    if let Some((example, left, right, error)) = worst {
        return EquivOutcome::Different {
            example,
            lhs: left,
            rhs: right,
            rel_error: error,
        };
    }
    if all_undefined {
        return EquivOutcome::Undefined {
            message: "every sample produced NaN or infinity".to_owned(),
        };
    }
    EquivOutcome::Equivalent
}

fn describe_dim(ty: &Type) -> String {
    ty.dim()
        .map_or_else(|| "non-numeric".to_owned(), ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::compile;
    use crate::typecheck::{NameResolver, Resolved, Type as ExprType, parse_and_check};
    use cetus_units::{Dim, UnitRegistry};

    struct Resolver;

    impl NameResolver for Resolver {
        fn resolve(&self, name: &str) -> Option<Resolved> {
            let id = match name {
                "a" => 0,
                "b" => 1,
                "c" => 2,
                _ => return None,
            };
            Some(Resolved::Variable {
                id,
                ty: ExprType::scalar(Dim::dimensionless()),
            })
        }
    }

    fn domain() -> Vec<DomainVar> {
        vec![
            DomainVar {
                id: 0,
                name: "a".to_owned(),
                min: 0.1,
                max: 10.0,
            },
            DomainVar {
                id: 1,
                name: "b".to_owned(),
                min: 0.1,
                max: 10.0,
            },
            DomainVar {
                id: 2,
                name: "c".to_owned(),
                min: 0.1,
                max: 10.0,
            },
        ]
    }

    #[test]
    fn detects_algebraic_equivalence() {
        let registry = UnitRegistry::new();
        let left = compile(&parse_and_check("a*b", &Resolver, &registry).expect("lhs"));
        let right = compile(&parse_and_check("b*a", &Resolver, &registry).expect("rhs"));
        assert_eq!(
            equivalent(&left, &right, &domain(), &EquivOptions::default()),
            EquivOutcome::Equivalent
        );
    }

    #[test]
    fn returns_a_counterexample_for_a_wrong_formula() {
        let registry = UnitRegistry::new();
        let left = compile(&parse_and_check("a*b", &Resolver, &registry).expect("lhs"));
        let right = compile(&parse_and_check("a*b*b", &Resolver, &registry).expect("rhs"));
        match equivalent(&left, &right, &domain(), &EquivOptions::default()) {
            EquivOutcome::Different {
                example, rel_error, ..
            } => {
                assert_eq!(example.len(), 3);
                assert!(rel_error > 0.0);
            }
            other => panic!("expected Different, got {other:?}"),
        }
    }

    #[test]
    fn dimension_mismatch_is_reported() {
        let registry = UnitRegistry::new();
        let left = compile(&parse_and_check("1[m]", &Resolver, &registry).expect("lhs"));
        let right = compile(&parse_and_check("1[s]", &Resolver, &registry).expect("rhs"));
        assert!(matches!(
            equivalent(&left, &right, &[], &EquivOptions::default()),
            EquivOutcome::TypeMismatch { .. }
        ));
    }
}
