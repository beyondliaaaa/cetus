//! Numerical equivalence of expressions (§5.2).
//!
//! Deliberately not a symbolic simplifier: we sample the declared domain with a
//! fixed seed and compare relative error. This catches "same algebra, different
//! spelling" (`η*A*G` vs `G*A*η`) and returns a counterexample for the hint
//! ladder (§6.4).
//!
//! Sampling is only meaningful when every free variable is covered by the
//! domain and every built-in that needs external data has it. Missing coverage
//! is reported, never silently evaluated as zero.

use crate::eval::{CompiledExpr, EvalContext, Instr};

mod inspect;

use inspect::{describe_dim, loads_of, missing_domain, used_lookups, uses};
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
    /// Time range in SI seconds for expressions that read `time`; required
    /// when they do.
    pub time: Option<(f64, f64)>,
    /// Macro step in SI seconds for expressions that read `dt`; required when
    /// they do.
    pub dt: Option<f64>,
    /// Typical output magnitude in SI units (K13), used as the denominator
    /// floor. `None` means pure relative error.
    pub scale: Option<f64>,
}

impl Default for EquivOptions {
    fn default() -> Self {
        Self {
            samples: 64,
            rel_tol: 1e-9,
            seed: 0x5eed_c3e5,
            time: None,
            dt: None,
            scale: None,
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
        /// Variable (and time, when sampled) assignments at the counterexample.
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
    /// The comparison could not produce a meaningful value.
    Undefined {
        /// Why the comparison was impossible.
        message: String,
    },
    /// The domain does not cover every variable the expressions read.
    IncompleteDomain {
        /// Slot ids missing from the domain.
        missing: Vec<u32>,
    },
}

/// Resolves table ids for expressions that use `lookup`.
pub trait TableLookup {
    /// Returns the table value, or `None` when the table id is unavailable.
    ///
    /// Implementations must decide availability by the table id, not by `x`:
    /// the sampler may probe with an arbitrary value.
    fn lookup(&self, table: u32, x: f64) -> Option<f64>;
}

struct NoTables;

impl TableLookup for NoTables {
    fn lookup(&self, _table: u32, _x: f64) -> Option<f64> {
        None
    }
}

type Counterexample = (Vec<(String, f64)>, f64, f64, f64);

struct SampleContext<'a> {
    values: &'a [(u32, f64)],
    time: f64,
    dt: f64,
    tables: &'a dyn TableLookup,
}

impl EvalContext for SampleContext<'_> {
    fn var(&self, id: u32) -> f64 {
        self.values
            .iter()
            .find(|(slot, _)| *slot == id)
            .map_or(f64::NAN, |(_, value)| *value)
    }

    fn time(&self) -> f64 {
        self.time
    }

    fn dt(&self) -> f64 {
        self.dt
    }

    fn state(&self, _id: u32) -> f64 {
        f64::NAN
    }

    fn lookup(&self, table: u32, x: f64) -> f64 {
        self.tables.lookup(table, x).unwrap_or(f64::NAN)
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
    equivalent_with_tables(lhs, rhs, domain, options, &NoTables)
}

/// Checks equivalence with external lookup tables available.
#[must_use]
pub fn equivalent_with_tables(
    lhs: &CompiledExpr,
    rhs: &CompiledExpr,
    domain: &[DomainVar],
    options: &EquivOptions,
    tables: &dyn TableLookup,
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
    if !options.rel_tol.is_finite() || options.rel_tol < 0.0 {
        return EquivOutcome::Undefined {
            message: "rel_tol must be finite and non-negative".to_owned(),
        };
    }

    let lhs_loads = loads_of(lhs);
    let rhs_loads = loads_of(rhs);
    let missing = missing_domain(&lhs_loads, &rhs_loads, domain);
    if !missing.is_empty() {
        return EquivOutcome::IncompleteDomain { missing };
    }

    let uses_time = uses(lhs, |instr| matches!(instr, Instr::Time))
        || uses(rhs, |instr| matches!(instr, Instr::Time));
    let time_range = if uses_time {
        match options.time {
            Some((start, end)) if start.is_finite() && end.is_finite() && start <= end => {
                Some((start, end))
            }
            Some(_) => {
                return EquivOutcome::Undefined {
                    message: "EquivOptions::time must be a finite [start, end] range".to_owned(),
                };
            }
            None => {
                return EquivOutcome::Undefined {
                    message: "expression depends on `time`; set EquivOptions::time".to_owned(),
                };
            }
        }
    } else {
        None
    };

    let uses_dt = uses(lhs, |instr| matches!(instr, Instr::Dt))
        || uses(rhs, |instr| matches!(instr, Instr::Dt));
    let dt_value = if uses_dt {
        match options.dt {
            Some(value) if value.is_finite() => value,
            _ => {
                return EquivOutcome::Undefined {
                    message: "expression depends on `dt`; set EquivOptions::dt".to_owned(),
                };
            }
        }
    } else {
        0.0
    };

    if lhs.state_slots() > 0 || rhs.state_slots() > 0 {
        return EquivOutcome::Undefined {
            message: "stateful builtins (delay1/delay3/smooth) cannot be compared".to_owned(),
        };
    }

    let lookup_ids = used_lookups(lhs, rhs);
    let missing_tables: Vec<u32> = lookup_ids
        .iter()
        .copied()
        .filter(|id| tables.lookup(*id, 0.0).is_none())
        .collect();
    if !missing_tables.is_empty() {
        return EquivOutcome::Undefined {
            message: format!("lookup table(s) {missing_tables:?} are not available"),
        };
    }

    let scale = options.scale.map_or(0.0, f64::abs);
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
        let time = time_range.map_or(0.0, |(start, end)| {
            if end > start {
                rng.random_range(start..=end)
            } else {
                start
            }
        });
        let ctx = SampleContext {
            values: &assignments,
            time,
            dt: dt_value,
            tables,
        };
        let left = lhs.eval(&ctx);
        let right = rhs.eval(&ctx);
        if !left.is_finite() || !right.is_finite() {
            continue;
        }
        all_undefined = false;
        let denominator = left.abs().max(right.abs()).max(scale);
        let error = if denominator == 0.0 {
            0.0
        } else {
            (left - right).abs() / denominator
        };
        if error > options.rel_tol {
            let mut example = domain
                .iter()
                .zip(assignments.iter())
                .map(|(var, (_, value))| (var.name.clone(), *value))
                .collect::<Vec<_>>();
            if time_range.is_some() {
                example.push(("time".to_owned(), time));
            }
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

#[cfg(test)]
mod tests;
