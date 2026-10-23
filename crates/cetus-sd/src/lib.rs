//! Paradigm: system dynamics / ODE (§5.7.1).
//!
//! The subsystem evaluates auxiliaries in dependency order, then integrates the
//! stocks with a fixed macro step. Integrators are deterministic: fixed step
//! counts, no wall clock, all math through `cetus-math`.

#![forbid(unsafe_code)]

use std::sync::Arc;

use cetus_compile::{CompiledPlan, CompiledTable};
use cetus_kernel::{EventSink, InvariantViolation, SimError, StateContext, StepContext, Subsystem};
use cetus_model::Integrator;

/// A system-dynamics subsystem.
pub struct SdSubsystem {
    plan: Arc<CompiledPlan>,
    integrator: Integrator,
    stock_count: usize,
    k1: Vec<f64>,
    k2: Vec<f64>,
    k3: Vec<f64>,
    k4: Vec<f64>,
}

impl std::fmt::Debug for SdSubsystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdSubsystem")
            .field("model", &self.plan.model_id)
            .field("integrator", &self.integrator)
            .field("stocks", &self.stock_count)
            .finish_non_exhaustive()
    }
}

impl SdSubsystem {
    /// Builds a subsystem for a compiled plan.
    #[must_use]
    pub fn new(plan: Arc<CompiledPlan>) -> Self {
        let stock_count = plan.stock_count();
        Self {
            integrator: Integrator::Rk4,
            k1: vec![0.0; stock_count],
            k2: vec![0.0; stock_count],
            k3: vec![0.0; stock_count],
            k4: vec![0.0; stock_count],
            plan,
            stock_count,
        }
    }

    /// Selects the integrator declared by the model.
    ///
    /// # Errors
    /// Returns an error for integrators that are not implemented yet.
    pub fn with_integrator(mut self, integrator: Integrator) -> Result<Self, SimError> {
        match integrator {
            Integrator::Euler | Integrator::Rk4 => {
                self.integrator = integrator;
                Ok(self)
            }
            Integrator::Ros2 => Err(SimError::Subsystem(
                "the `ros2` integrator lands in M6".to_owned(),
            )),
            Integrator::Dopri5 => Err(SimError::Subsystem(
                "the `dopri5` integrator lands in M1.5".to_owned(),
            )),
        }
    }

    /// Computes all auxiliaries in dependency order and writes them into `state`.
    fn compute_auxiliaries(plan: &CompiledPlan, state: &mut [f64], t: f64) {
        for auxiliary in &plan.auxiliaries {
            let value = {
                let ctx = StateContext {
                    state,
                    t,
                    dt: plan.time.quantum,
                    tables: &plan.tables,
                };
                auxiliary.expr.eval(&ctx)
            };
            state[auxiliary.slot] = value;
        }
    }

    fn derivative(plan: &CompiledPlan, state: &mut [f64], t: f64, out: &mut [f64]) {
        Self::compute_auxiliaries(plan, state, t);
        out.fill(0.0);
        let ctx = StateContext {
            state,
            t,
            dt: plan.time.quantum,
            tables: &plan.tables,
        };
        for flow in &plan.flows {
            let rate = flow.rate.eval(&ctx);
            for (slot, sign) in &flow.targets {
                out[*slot] += f64::from(*sign) * rate;
            }
        }
    }

    // The RK4 stage updates index four arrays in lockstep; the indexed form is
    // the mathematical formula and is clearer than a zip of four iterators.
    #[allow(clippy::needless_range_loop)]
    fn integrate(&mut self, state: &mut [f64], t: f64, h: f64) -> Result<(), SimError> {
        match self.integrator {
            Integrator::Euler => {
                let mut k1 = vec![0.0; self.stock_count];
                Self::derivative(&self.plan, state, t, &mut k1);
                for index in 0..self.stock_count {
                    state[index] += h * k1[index];
                }
                Ok(())
            }
            Integrator::Rk4 => {
                let n = self.stock_count;
                let base: Vec<f64> = state.to_vec();

                Self::derivative(&self.plan, state, t, &mut self.k1);

                let mut y = base.clone();
                for index in 0..n {
                    y[index] += 0.5 * h * self.k1[index];
                }
                Self::derivative(&self.plan, &mut y, t + 0.5 * h, &mut self.k2);

                let mut y = base.clone();
                for index in 0..n {
                    y[index] += 0.5 * h * self.k2[index];
                }
                Self::derivative(&self.plan, &mut y, t + 0.5 * h, &mut self.k3);

                let mut y = base.clone();
                for index in 0..n {
                    y[index] += h * self.k3[index];
                }
                Self::derivative(&self.plan, &mut y, t + h, &mut self.k4);

                for index in 0..n {
                    let delta = h / 6.0
                        * (self.k1[index]
                            + 2.0 * self.k2[index]
                            + 2.0 * self.k3[index]
                            + self.k4[index]);
                    state[index] += delta;
                }
                Ok(())
            }
            Integrator::Dopri5 | Integrator::Ros2 => Err(SimError::Subsystem(
                "integrator not implemented for this model".to_owned(),
            )),
        }
    }

    fn clamp_bounds(&self, state: &mut [f64], tick: u64, events: &mut EventSink) {
        for stock in &self.plan.stocks {
            let Some((min, max)) = stock.bounds else {
                continue;
            };
            let before = state[stock.slot];
            let mut after = before;
            if let Some(min) = min
                && after < min
            {
                after = min;
            }
            if let Some(max) = max
                && after > max
            {
                after = max;
            }
            // Clamping is exact by definition: any change, however small, is a clamp event.
            #[allow(clippy::float_cmp)]
            if after != before {
                events.clamps.push(cetus_kernel::ClampEvent {
                    slot: stock.slot,
                    name: stock.name.clone(),
                    before,
                    after,
                    tick,
                });
                state[stock.slot] = after;
            }
        }
    }

    fn check_invariants(
        plan: &CompiledPlan,
        state: &[f64],
        t: f64,
        tick: u64,
        events: &mut EventSink,
    ) -> Result<(), SimError> {
        let ctx = StateContext {
            state,
            t,
            dt: plan.time.quantum,
            tables: &plan.tables,
        };
        for invariant in &plan.invariants {
            let Some(expr) = &invariant.expr else {
                continue;
            };
            let value = expr.eval(&ctx);
            if value == 0.0 {
                events.violations.push(InvariantViolation {
                    id: invariant.id.clone(),
                    tick,
                });
                return Err(SimError::Invariant {
                    id: invariant.id.clone(),
                });
            }
        }
        Ok(())
    }
}

impl Subsystem for SdSubsystem {
    fn id(&self) -> &'static str {
        "sd"
    }

    fn init(&mut self, state: &mut [f64], t: f64) -> Result<(), SimError> {
        Self::compute_auxiliaries(&self.plan, state, t);
        Ok(())
    }

    fn advance(
        &mut self,
        state: &mut [f64],
        ctx: &StepContext,
        events: &mut EventSink,
    ) -> Result<(), SimError> {
        self.integrate(state, ctx.t, ctx.h)?;
        self.clamp_bounds(state, ctx.tick, events);
        Self::compute_auxiliaries(&self.plan, state, ctx.t + ctx.h);
        Self::check_invariants(&self.plan, state, ctx.t + ctx.h, ctx.tick, events)?;
        Ok(())
    }
}

/// Unused helper kept for the future multi-paradigm bridge layer.
#[allow(dead_code)]
fn table_lookup(tables: &[CompiledTable], id: u32, x: f64) -> f64 {
    tables.get(id as usize).map_or(0.0, |table| table.lookup(x))
}
