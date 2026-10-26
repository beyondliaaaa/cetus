//! V2 numerical checks: parameter sweep, step-size convergence, determinism.

use std::collections::BTreeMap;
use std::sync::Arc;

use cetus_compile::{CompiledPlan, Diagnostic, compile_model};
use cetus_kernel::{Sim, SimError};
use cetus_math::sqrt;
use cetus_model::{Integrator, Model, ParadigmId, TimeKind};
use cetus_sd::SdSubsystem;

use crate::Level;
use crate::report::{CheckResult, Report};

const LHS_SAMPLES: usize = 64;
const SWEEP_SEEDS: [u64; 4] = [1, 2, 3, 4];
const CONVERGENCE_SAMPLES: usize = 32;
const CONVERGENCE_MAX_NRMSE: f64 = 0.01;
const CONVERGENCE_DT_DIVISORS: [f64; 3] = [1.0, 2.0, 4.0];
const CLAMP_WARNING_PERCENT: f64 = 50.0;
const EPSILON: f64 = 1.0e-12;
const SAMPLING_SEED: u64 = 0x4354_5553;

/// Runs the cumulative V2 checks for `model` and its compiled `plan`.
pub(crate) fn check(model: &Model, plan: &Arc<CompiledPlan>, report: &mut Report) {
    if !model.paradigms.contains(&ParadigmId::Sd) {
        report.checks.push(CheckResult {
            level: Level::V2,
            name: "numeric sweep".to_owned(),
            passed: true,
            detail: "skipped: V2 currently implements the SD/ODE paradigm".to_owned(),
        });
        return;
    }
    let Some(horizon) = horizon_seconds(model, plan) else {
        let detail = "model has no time.horizon; V2 needs a finite validation horizon";
        report.checks.push(CheckResult {
            level: Level::V2,
            name: "numeric sweep".to_owned(),
            passed: false,
            detail: detail.to_owned(),
        });
        report.diagnostics.push(
            Diagnostic::error("E7201", detail, Some("time.horizon"))
                .with_help("set time.horizon in the module"),
        );
        return;
    };
    let sweep = check_sweep(model, plan, horizon, report);
    check_clamp_count(&sweep, report);
    check_convergence(model, plan, horizon, &sweep.worst_params, report);
    check_determinism(model, plan, horizon, report);
}

fn horizon_seconds(model: &Model, plan: &CompiledPlan) -> Option<f64> {
    model
        .time
        .horizon
        .map(|horizon| horizon * plan.time.unit_factor)
}

#[derive(Debug, Default)]
struct SweepSummary {
    clamps: BTreeMap<String, usize>,
    max_fraction: BTreeMap<String, f64>,
    steps: usize,
    worst_params: Vec<(String, f64)>,
    worst_fraction: f64,
}

#[derive(Debug)]
struct RunSummary {
    clamps: BTreeMap<String, usize>,
    steps: usize,
}

fn check_sweep(
    model: &Model,
    plan: &Arc<CompiledPlan>,
    horizon: f64,
    report: &mut Report,
) -> SweepSummary {
    let ranged: Vec<(&str, f64, f64)> = plan
        .parameters
        .iter()
        .filter_map(|parameter| {
            parameter
                .range
                .map(|(low, high)| (parameter.name.as_str(), low, high))
        })
        .collect();
    let ranges: Vec<(f64, f64)> = ranged.iter().map(|(_, low, high)| (*low, *high)).collect();
    let samples = latin_hypercube(&ranges, LHS_SAMPLES, sampling_seed(plan));
    let mut failures = Vec::new();
    let mut summary = SweepSummary::default();
    let mut runs = 0usize;
    for (index, sample) in samples.iter().enumerate() {
        for seed in SWEEP_SEEDS {
            runs += 1;
            match run_one(model, plan, horizon, &ranged, sample, seed) {
                Ok(run) => {
                    summary.steps += run.steps;
                    let mut run_worst = 0.0_f64;
                    if run.steps > 0 {
                        for count in run.clamps.values() {
                            run_worst = run_worst.max(*count as f64 / run.steps as f64);
                        }
                    }
                    if run_worst > summary.worst_fraction {
                        summary.worst_fraction = run_worst;
                        summary.worst_params = ranged
                            .iter()
                            .zip(sample)
                            .map(|((name, _, _), value)| ((*name).to_owned(), *value))
                            .collect();
                    }
                    for (name, count) in run.clamps {
                        *summary.clamps.entry(name.clone()).or_default() += count;
                        if run.steps > 0 {
                            let fraction = count as f64 / run.steps as f64;
                            summary
                                .max_fraction
                                .entry(name)
                                .and_modify(|max| *max = (*max).max(fraction))
                                .or_insert(fraction);
                        }
                    }
                }
                Err(error) => failures.push(format!("sample {index}, seed {seed}: {error}")),
            }
        }
    }
    let passed = failures.is_empty();
    report.checks.push(CheckResult {
        level: Level::V2,
        name: "parameter sweep".to_owned(),
        passed,
        detail: if passed {
            format!(
                "{runs} runs across {} LHS samples x {} seeds",
                samples.len(),
                SWEEP_SEEDS.len()
            )
        } else {
            format!("{} of {runs} runs failed", failures.len())
        },
    });
    for failure in failures.iter().take(3) {
        report.diagnostics.push(
            Diagnostic::error(
                "E7201",
                format!("numeric sweep failed: {failure}"),
                Some("v2.sweep"),
            )
            .with_help("fix the model's numerics or narrow the declared parameter ranges"),
        );
    }
    if failures.len() > 3 {
        report.diagnostics.push(Diagnostic::error(
            "E7201",
            format!(
                "numeric sweep failed in {} more run(s); first failure above",
                failures.len() - 3
            ),
            Some("v2.sweep"),
        ));
    }
    summary
}

fn run_one(
    model: &Model,
    plan: &Arc<CompiledPlan>,
    horizon: f64,
    ranged: &[(&str, f64, f64)],
    sample: &[f64],
    seed: u64,
) -> Result<RunSummary, String> {
    let mut sim = make_sim(plan, model.time.integrator, seed).map_err(|error| error.to_string())?;
    for ((name, _, _), value) in ranged.iter().zip(sample) {
        sim.set_param(name, *value)
            .map_err(|error| error.to_string())?;
    }
    let target = plan.time.tick_of(horizon);
    let steps = target.saturating_sub(plan.time.start_tick) as usize;
    let mut clamps: BTreeMap<String, usize> = BTreeMap::new();
    while sim.tick() < target {
        let step = sim.step().map_err(|error| error.to_string())?;
        for clamp in &step.clamps {
            *clamps.entry(clamp.name.clone()).or_default() += 1;
        }
    }
    for observable in &plan.observables {
        let value = sim
            .observable(&observable.name)
            .map_err(|error| error.to_string())?;
        if !value.is_finite() {
            return Err(format!("observable `{}` is not finite", observable.name));
        }
    }
    Ok(RunSummary { clamps, steps })
}

fn make_sim(plan: &Arc<CompiledPlan>, integrator: Integrator, seed: u64) -> Result<Sim, SimError> {
    let subsystem = SdSubsystem::new(plan.clone()).with_integrator(integrator)?;
    Sim::new(plan.clone(), seed, Box::new(subsystem))
}

fn check_clamp_count(sweep: &SweepSummary, report: &mut Report) {
    let total: usize = sweep.clamps.values().sum();
    let mut above = 0usize;
    let mut highest = 0.0_f64;
    for (name, fraction) in &sweep.max_fraction {
        highest = highest.max(*fraction);
        let percent = fraction * 100.0;
        if percent > CLAMP_WARNING_PERCENT {
            above += 1;
            report.diagnostics.push(
                Diagnostic::warning(
                    "W7201",
                    format!(
                        "stock `{name}` clamped on up to {percent:.1}% of steps in one sampled run \
                         (threshold {CLAMP_WARNING_PERCENT:.0}%)"
                    ),
                    Some(&format!("stocks.{name}")),
                )
                .with_help("check the stock bounds and the flows that drive the stock toward them"),
            );
        }
    }
    report.checks.push(CheckResult {
        level: Level::V2,
        name: "clamp count".to_owned(),
        passed: true,
        detail: format!(
            "{total} clamp event(s) over {} sampled steps; max {:.1}%{}",
            sweep.steps,
            highest * 100.0,
            if above == 0 { "" } else { "; above threshold" }
        ),
    });
}

fn check_convergence(
    model: &Model,
    plan: &Arc<CompiledPlan>,
    horizon: f64,
    worst_params: &[(String, f64)],
    report: &mut Report,
) {
    if model.time.kind != TimeKind::Continuous {
        report.checks.push(CheckResult {
            level: Level::V2,
            name: "step-size convergence".to_owned(),
            passed: true,
            detail: "skipped: non-continuous time".to_owned(),
        });
        return;
    }
    if model.time.dt <= 0.0 {
        report.checks.push(CheckResult {
            level: Level::V2,
            name: "step-size convergence".to_owned(),
            passed: false,
            detail: "time.dt must be positive".to_owned(),
        });
        report.diagnostics.push(Diagnostic::error(
            "E7202",
            "step-size convergence needs a positive time.dt",
            Some("time.dt"),
        ));
        return;
    }

    let mut scenarios: Vec<(&str, &[(String, f64)])> = vec![("default", &[])];
    if !worst_params.is_empty() {
        scenarios.push(("worst-clamp sample", worst_params));
    }
    let scenario_count = scenarios.len();

    let mut series: Vec<(String, String)> = Vec::new();
    for stock in &plan.stocks {
        series.push((
            format!("stock `{}`", stock.name),
            format!("stock:{}", stock.name),
        ));
    }
    for observable in &plan.observables {
        series.push((
            format!("observable `{}`", observable.name),
            format!("observable:{}", observable.name),
        ));
    }

    let mut worst = 0.0_f64;
    let mut failures = Vec::new();
    for (label, params) in scenarios {
        let mut runs = Vec::new();
        for divisor in CONVERGENCE_DT_DIVISORS {
            let mut variant = model.clone();
            variant.time.dt = model.time.dt / divisor;
            let variant_plan = match compile_model(&variant) {
                Ok(plan) => Arc::new(plan),
                Err(diagnostics) => {
                    let message = diagnostics
                        .iter()
                        .map(cetus_compile::Diagnostic::to_line)
                        .collect::<Vec<_>>()
                        .join("; ");
                    report.checks.push(CheckResult {
                        level: Level::V2,
                        name: "step-size convergence".to_owned(),
                        passed: false,
                        detail: format!("dt/{divisor} failed to compile"),
                    });
                    report.diagnostics.push(
                        Diagnostic::error(
                            "E7202",
                            format!(
                                "step-size convergence: dt/{divisor} failed to compile: {message}"
                            ),
                            Some("time.dt"),
                        )
                        .with_help("keep the model compilable at smaller steps"),
                    );
                    return;
                }
            };
            match sample_trajectory(&variant_plan, horizon, model.time.integrator, params) {
                Ok(samples) => runs.push(samples),
                Err(error) => {
                    report.checks.push(CheckResult {
                        level: Level::V2,
                        name: "step-size convergence".to_owned(),
                        passed: false,
                        detail: format!("dt/{divisor} failed to run"),
                    });
                    report.diagnostics.push(
                        Diagnostic::error(
                            "E7202",
                            format!("step-size convergence: dt/{divisor} failed to run: {error}"),
                            Some("time.dt"),
                        )
                        .with_help("fix the numerical failure before checking convergence"),
                    );
                    return;
                }
            }
        }
        for (display, key) in &series {
            let Some(coarse) = runs[0].get(key) else {
                continue;
            };
            let Some(mid) = runs[1].get(key) else {
                continue;
            };
            let Some(fine) = runs[2].get(key) else {
                continue;
            };
            let coarse_mid = nrmse(coarse, mid);
            let mid_fine = nrmse(mid, fine);
            let coarse_fine = nrmse(coarse, fine);
            worst = worst.max(coarse_fine);
            let monotonic = coarse_mid + EPSILON >= mid_fine;
            if !monotonic || coarse_fine > CONVERGENCE_MAX_NRMSE {
                failures.push((
                    label.to_owned(),
                    display.clone(),
                    coarse_mid,
                    mid_fine,
                    coarse_fine,
                ));
            }
        }
    }
    let passed = failures.is_empty();
    report.checks.push(CheckResult {
        level: Level::V2,
        name: "step-size convergence".to_owned(),
        passed,
        detail: if passed {
            format!(
                "{scenario_count} scenario(s), {} series; worst nrmse {worst:.3e}",
                series.len()
            )
        } else {
            format!(
                "{} series check(s) failed; worst nrmse {worst:.3e}",
                failures.len()
            )
        },
    });
    for (label, display, coarse_mid, mid_fine, coarse_fine) in failures.iter().take(3) {
        report.diagnostics.push(
            Diagnostic::error(
                "E7202",
                format!(
                    "step-size convergence failed for {display} ({label}): \
                     nrmse(dt,dt/2)={coarse_mid:.3e}, nrmse(dt/2,dt/4)={mid_fine:.3e}, \
                     nrmse(dt,dt/4)={coarse_fine:.3e}"
                ),
                Some("time.dt"),
            )
            .with_help("reduce dt, smooth the model, or check event-time resolution"),
        );
    }
}

fn sample_trajectory(
    plan: &Arc<CompiledPlan>,
    horizon: f64,
    integrator: Integrator,
    params: &[(String, f64)],
) -> Result<BTreeMap<String, Vec<f64>>, String> {
    let mut sim = make_sim(plan, integrator, SWEEP_SEEDS[0]).map_err(|error| error.to_string())?;
    for (name, value) in params {
        sim.set_param(name, *value)
            .map_err(|error| error.to_string())?;
    }
    let target = plan.time.tick_of(horizon);
    let mut samples: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for stock in &plan.stocks {
        samples.insert(
            format!("stock:{}", stock.name),
            Vec::with_capacity(CONVERGENCE_SAMPLES),
        );
    }
    for observable in &plan.observables {
        samples.insert(
            format!("observable:{}", observable.name),
            Vec::with_capacity(CONVERGENCE_SAMPLES),
        );
    }
    for index in 0..CONVERGENCE_SAMPLES {
        let fraction = if CONVERGENCE_SAMPLES <= 1 {
            0.0
        } else {
            index as f64 / (CONVERGENCE_SAMPLES - 1) as f64
        };
        let tick = plan.time.tick_of(horizon * fraction).min(target);
        sim.run_until(tick).map_err(|error| error.to_string())?;
        for stock in &plan.stocks {
            let value = sim
                .stock(&stock.name)
                .ok_or_else(|| format!("unknown stock `{}`", stock.name))?;
            if !value.is_finite() {
                return Err(format!("stock `{}` is not finite", stock.name));
            }
            samples
                .get_mut(&format!("stock:{}", stock.name))
                .expect("stock was inserted above")
                .push(value);
        }
        for observable in &plan.observables {
            let value = sim
                .observable(&observable.name)
                .map_err(|error| error.to_string())?;
            if !value.is_finite() {
                return Err(format!("observable `{}` is not finite", observable.name));
            }
            samples
                .get_mut(&format!("observable:{}", observable.name))
                .expect("observable was inserted above")
                .push(value);
        }
    }
    Ok(samples)
}

fn nrmse(reference: &[f64], candidate: &[f64]) -> f64 {
    if reference.is_empty() || reference.len() != candidate.len() {
        return f64::INFINITY;
    }
    let max = reference.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = reference.iter().copied().fold(f64::INFINITY, f64::min);
    let span = max - min;
    let scale = if span.abs() > EPSILON {
        span
    } else {
        max.abs().max(EPSILON)
    };
    let mse = reference
        .iter()
        .zip(candidate)
        .map(|(left, right)| {
            let difference = left - right;
            difference * difference
        })
        .sum::<f64>()
        / (reference.len() as f64);
    sqrt(mse) / scale
}

fn check_determinism(model: &Model, plan: &Arc<CompiledPlan>, horizon: f64, report: &mut Report) {
    let seed = SWEEP_SEEDS[0];
    let first = run_hash(plan, model.time.integrator, seed, horizon);
    let second = run_hash(plan, model.time.integrator, seed, horizon);
    let passed = match (&first, &second) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    };
    let detail = match (&first, &second) {
        (Ok(left), Ok(right)) if passed => format!("seed {seed} reproduces {left}"),
        (Ok(_), Ok(_)) => "same seed produced different hashes".to_owned(),
        (Err(error), _) | (_, Err(error)) => format!("run failed: {error}"),
    };
    report.checks.push(CheckResult {
        level: Level::V2,
        name: "native determinism".to_owned(),
        passed,
        detail,
    });
    if !passed {
        report.diagnostics.push(
            Diagnostic::error(
                "E7203",
                "same-seed native runs produced different state hashes",
                Some("v2.determinism"),
            )
            .with_help("remove unordered iteration, wall-clock time or OS randomness"),
        );
    }
    report.diagnostics.push(
        Diagnostic::warning(
            "W7202",
            "cross-target determinism is not run by cetus verify; \
             scripts/check-cross-target.sh covers it in CI",
            Some("v2.determinism"),
        )
        .with_help("run ./scripts/check-cross-target.sh before release"),
    );
}

fn run_hash(
    plan: &Arc<CompiledPlan>,
    integrator: Integrator,
    seed: u64,
    horizon: f64,
) -> Result<String, String> {
    let mut sim = make_sim(plan, integrator, seed).map_err(|error| error.to_string())?;
    sim.run_until(plan.time.tick_of(horizon))
        .map_err(|error| error.to_string())?;
    Ok(sim.state_hash_hex())
}

fn sampling_seed(plan: &CompiledPlan) -> u64 {
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&plan.content_hash[..8]);
    u64::from_le_bytes(prefix) ^ SAMPLING_SEED
}

fn latin_hypercube(ranges: &[(f64, f64)], samples: usize, seed: u64) -> Vec<Vec<f64>> {
    if ranges.is_empty() || samples == 0 {
        return vec![Vec::new()];
    }
    let mut rng = SplitMix64::new(seed);
    let mut columns: Vec<Vec<f64>> = Vec::with_capacity(ranges.len());
    for &(low, high) in ranges {
        let mut strata: Vec<usize> = (0..samples).collect();
        for index in (1..samples).rev() {
            let swap_with = rng.below(index + 1);
            strata.swap(index, swap_with);
        }
        let mut column = Vec::with_capacity(samples);
        for &stratum in &strata {
            let fraction = (stratum as f64 + rng.unit()) / samples as f64;
            column.push(low + fraction * (high - low));
        }
        columns.push(column);
    }
    (0..samples)
        .map(|index| columns.iter().map(|column| column[index]).collect())
        .collect()
}

/// A small deterministic PRNG so the sample set never depends on OS entropy.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lhs_is_reproducible_and_stays_in_range() {
        let ranges = [(0.0, 1.0), (-2.0, 2.0)];
        let first = latin_hypercube(&ranges, 16, 7);
        let second = latin_hypercube(&ranges, 16, 7);
        assert_eq!(first, second);
        for sample in &first {
            assert!((0.0..=1.0).contains(&sample[0]));
            assert!((-2.0..=2.0).contains(&sample[1]));
        }
    }

    #[test]
    fn lhs_fills_every_stratum_once() {
        let samples = latin_hypercube(&[(0.0, 1.0)], 8, 3);
        let mut counts = [0_usize; 8];
        for sample in &samples {
            let index = (sample[0] * 8.0).floor() as usize;
            counts[index.min(7)] += 1;
        }
        assert!(counts.iter().all(|count| *count == 1), "{counts:?}");
    }

    #[test]
    fn nrmse_of_identical_series_is_zero() {
        let series = [1.0, 2.0, 3.0];
        assert!(nrmse(&series, &series) < EPSILON);
    }
}
