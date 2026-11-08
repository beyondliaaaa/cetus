//! `cetus` — the command line tool.
//!
//! Every command supports `--json` so that other agents can parse the output
//! mechanically (§0.2). Exit codes: 0 success, 1 user/model error, 2 internal
//! error.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use cetus_compile::{CompiledPlan, Diagnostic, Severity, compile_model_with_warnings};
use cetus_kernel::{Column, Dataset, DimAxis, Sim};
use cetus_model::Model;
use cetus_sd::SdSubsystem;
use cetus_units::{Dim, Unit, UnitRegistry};
use clap::{Parser, Subcommand};

mod verify;

#[derive(Debug, Parser)]
#[command(name = "cetus", version, about = "Cetus engine CLI (鲸鱼娘历险记)")]
struct Cli {
    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the cross-target determinism witness and print its digest.
    Hello,
    /// Validate a model file (V0 + V1).
    Check {
        /// Path to `module.yaml`.
        path: PathBuf,
    },
    /// Verify a module or lesson (V0–V2 in this build).
    Verify {
        /// Path to `module.yaml` or `lesson.yaml`.
        path: PathBuf,
        /// Verification level: V0, V1 or V2.
        #[arg(long, default_value = "V1")]
        level: String,
        /// Report format(s), comma separated: md, json.
        #[arg(long, default_value = "md")]
        report: String,
    },
    /// Run a model and export probe data as CSV.
    Run {
        /// Path to `module.yaml`.
        path: PathBuf,
        /// Simulated horizon, e.g. `72h`. Defaults to `time.horizon`.
        #[arg(long)]
        until: Option<String>,
        /// Observable to record; repeatable. Defaults to every observable.
        #[arg(long = "probe")]
        probes: Vec<String>,
        /// Write CSV here instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Random seed.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Print the final state hash instead of the CSV.
        #[arg(long)]
        hash: bool,
    },
    /// Sweep parameters on a grid and report the final value of one observable.
    Sweep {
        /// Path to `module.yaml`.
        path: PathBuf,
        /// Grid specification: `area=1..100:8,load=0..2000:4`.
        #[arg(long)]
        space: String,
        /// Observable reported for each point.
        #[arg(long)]
        probe: String,
        /// Simulated horizon, e.g. `72h`. Defaults to `time.horizon`.
        #[arg(long)]
        until: Option<String>,
        /// Write CSV here instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Random seed.
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// Print the explanation page for a diagnostic code.
    Explain {
        /// Diagnostic code, e.g. `E1203`.
        code: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Hello => hello(cli.json),
        Command::Check { path } => check(&path, cli.json),
        Command::Verify {
            path,
            level,
            report,
        } => verify::run(&path, &level, &report, cli.json),
        Command::Run {
            path,
            until,
            probes,
            out,
            seed,
            hash,
        } => run(
            &path,
            until.as_deref(),
            &probes,
            out.as_deref(),
            seed,
            hash,
            cli.json,
        ),
        Command::Sweep {
            path,
            space,
            probe,
            until,
            out,
            seed,
        } => sweep(
            &path,
            &space,
            &probe,
            until.as_deref(),
            out.as_deref(),
            seed,
            cli.json,
        ),
        Command::Explain { code } => explain(&code, cli.json),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code),
    }
}

// All subcommands share one signature so `main` can dispatch uniformly.
#[allow(clippy::unnecessary_wraps)]
fn hello(json: bool) -> Result<(), u8> {
    let digest = cetus_kernel::hello_digest_hex();
    if json {
        let payload = serde_json::json!({
            "command": "hello",
            "digest": digest,
            "target_arch": std::env::consts::ARCH,
            "target_os": std::env::consts::OS,
        });
        println!("{payload}");
    } else {
        println!("{digest}");
    }
    Ok(())
}

fn load_model(path: &Path) -> Result<Model, u8> {
    let source = std::fs::read_to_string(path).map_err(|error| {
        eprintln!("error: cannot read {}: {error}", path.display());
        1
    })?;
    Model::from_yaml(&source).map_err(|error| {
        eprintln!("error: {error}");
        1
    })
}

fn compile_or_report(model: &Model, json: bool) -> Result<CompiledPlan, u8> {
    let (plan, diagnostics) = compile_model_with_warnings(model);
    let Some(plan) = plan else {
        if json {
            let payload = serde_json::json!({
                "ok": false,
                "diagnostics": diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
            });
            println!("{payload}");
        } else {
            for diagnostic in &diagnostics {
                eprintln!("{}", diagnostic.to_line());
            }
        }
        return Err(1);
    };
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "model": plan.model_id,
                "version": plan.model_version,
                "stocks": plan.stocks.len(),
                "parameters": plan.parameters.len(),
                "auxiliaries": plan.auxiliaries.len(),
                "flows": plan.flows.len(),
                "observables": plan.observables.len(),
                "quantum_s": plan.time.quantum,
                "diagnostics": diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
            })
        );
    } else {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic.to_line());
        }
        println!(
            "ok: {}@{} ({} stocks, {} parameters, {} flows)",
            plan.model_id,
            plan.model_version,
            plan.stocks.len(),
            plan.parameters.len(),
            plan.flows.len()
        );
    }
    Ok(plan)
}

fn diagnostic_json(diagnostic: &Diagnostic) -> serde_json::Value {
    serde_json::json!({
        "code": diagnostic.code,
        "severity": match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        },
        "path": diagnostic.path,
        "message": diagnostic.message,
        "span": diagnostic.span,
        "help": diagnostic.help,
    })
}

fn check(path: &Path, json: bool) -> Result<(), u8> {
    let model = load_model(path)?;
    compile_or_report(&model, json)?;
    Ok(())
}

fn parse_duration(text: &str, registry: &UnitRegistry) -> Result<f64, String> {
    let split = text
        .find(|ch: char| !(ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let value: f64 = number
        .parse()
        .map_err(|_| format!("bad duration `{text}`"))?;
    let unit = if unit.is_empty() { "s" } else { unit };
    let unit = registry
        .parse(unit)
        .map_err(|error| format!("bad duration unit `{unit}`: {error}"))?;
    Ok(value * unit.factor())
}

fn make_sim(model: &Model, plan: &CompiledPlan, seed: u64) -> Result<Sim, u8> {
    let plan = Arc::new(plan.clone());
    let subsystem = SdSubsystem::new(plan.clone())
        .with_integrator(model.time.integrator)
        .map_err(|error| {
            eprintln!("error: {error}");
            1
        })?;
    Sim::new(plan, seed, Box::new(subsystem)).map_err(|error| {
        eprintln!("error: {error}");
        1
    })
}

fn horizon_seconds(model: &Model, plan: &CompiledPlan, until: Option<&str>) -> Result<f64, u8> {
    match until {
        Some(text) => parse_duration(text, &UnitRegistry::new()).map_err(|error| {
            eprintln!("error: {error}");
            1
        }),
        None => Ok(model.time.horizon.unwrap_or(0.0) * plan.time.unit_factor),
    }
}

fn run(
    path: &Path,
    until: Option<&str>,
    probes: &[String],
    out: Option<&Path>,
    seed: u64,
    hash: bool,
    json: bool,
) -> Result<(), u8> {
    let model = load_model(path)?;
    let plan = compile_or_report(&model, json)?;
    let seconds = horizon_seconds(&model, &plan, until)?;
    if seconds <= 0.0 {
        eprintln!("error: no horizon; pass --until or set time.horizon");
        return Err(1);
    }

    let mut sim = make_sim(&model, &plan, seed)?;

    let selected: Vec<String> = if probes.is_empty() {
        plan.observables
            .iter()
            .map(|observable| observable.name.clone())
            .collect()
    } else {
        probes.to_vec()
    };
    for probe in &selected {
        sim.add_probe(probe).map_err(|error| {
            eprintln!("error: {error}");
            1
        })?;
    }

    sim.run_until(plan.time.tick_of(seconds)).map_err(|error| {
        eprintln!("error: {error}");
        1
    })?;

    if hash {
        println!("{}", sim.state_hash_hex());
        return Ok(());
    }

    let dataset = Dataset::from_probes(&plan, sim.probes());
    let rows = dataset.rows();
    let csv = dataset.to_csv();

    match out {
        Some(path) => {
            std::fs::write(path, &csv).map_err(|error| {
                eprintln!("error: cannot write {}: {error}", path.display());
                1
            })?;
            if !json {
                println!("wrote {rows} rows to {}", path.display());
            }
        }
        None => print!("{csv}"),
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct Axis {
    name: String,
    min: f64,
    max: f64,
    steps: usize,
}

impl Axis {
    fn value_at(&self, step: usize) -> f64 {
        if self.steps <= 1 {
            self.min
        } else {
            let t = step as f64 / (self.steps - 1) as f64;
            self.min + t * (self.max - self.min)
        }
    }
}

fn parse_space(text: &str) -> Result<Vec<Axis>, String> {
    let mut axes = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, range) = part
            .split_once('=')
            .ok_or_else(|| format!("bad axis `{part}`; expected name=min..max:steps"))?;
        let (bounds, steps) = range
            .split_once(':')
            .ok_or_else(|| format!("bad axis `{part}`; expected name=min..max:steps"))?;
        let (min, max) = bounds
            .split_once("..")
            .ok_or_else(|| format!("bad range `{bounds}`"))?;
        axes.push(Axis {
            name: name.trim().to_owned(),
            min: min
                .trim()
                .parse()
                .map_err(|_| format!("bad min in `{part}`"))?,
            max: max
                .trim()
                .parse()
                .map_err(|_| format!("bad max in `{part}`"))?,
            steps: steps
                .trim()
                .parse()
                .map_err(|_| format!("bad step count in `{part}`"))?,
        });
    }
    if axes.is_empty() {
        return Err("empty sweep space".to_owned());
    }
    Ok(axes)
}

fn axis_unit(plan: &CompiledPlan, name: &str) -> Result<Unit, u8> {
    let Some(slot) = plan.layout.slot_of(name) else {
        eprintln!("error: unknown parameter `{name}`");
        return Err(1);
    };
    Ok(plan.layout.slots[slot].unit.clone())
}

fn sweep(
    path: &Path,
    space: &str,
    probe: &str,
    until: Option<&str>,
    out: Option<&Path>,
    seed: u64,
    json: bool,
) -> Result<(), u8> {
    let model = load_model(path)?;
    let plan = compile_or_report(&model, json)?;
    let axes = parse_space(space).map_err(|error| {
        eprintln!("error: {error}");
        1
    })?;
    let seconds = horizon_seconds(&model, &plan, until)?;

    let mut units = Vec::with_capacity(axes.len());
    for axis in &axes {
        units.push(axis_unit(&plan, &axis.name)?);
    }

    let mut points: Vec<Vec<f64>> = vec![Vec::new()];
    for (axis, unit) in axes.iter().zip(&units) {
        let mut next = Vec::new();
        for point in &points {
            for step in 0..axis.steps {
                let mut extended = point.clone();
                extended.push(axis.value_at(step) * unit.factor());
                next.push(extended);
            }
        }
        points = next;
    }

    let dimensions: Vec<DimAxis> = axes
        .iter()
        .zip(&units)
        .map(|(axis, unit)| DimAxis::Parameter {
            id: axis.name.clone(),
            values: (0..axis.steps)
                .map(|step| axis.value_at(step) * unit.factor())
                .collect(),
            unit: unit.clone(),
            factor: unit.factor(),
        })
        .collect();

    let mut results = Vec::with_capacity(points.len());
    for point in &points {
        let mut sim = make_sim(&model, &plan, seed)?;
        for (axis, value) in axes.iter().zip(point.iter()) {
            sim.set_param(&axis.name, *value).map_err(|error| {
                eprintln!("error: {error}");
                1
            })?;
        }
        sim.add_probe(probe).map_err(|error| {
            eprintln!("error: {error}");
            1
        })?;
        sim.run_until(plan.time.tick_of(seconds)).map_err(|error| {
            eprintln!("error: {error}");
            1
        })?;
        results.push(
            sim.probes()
                .first()
                .and_then(|probe| probe.values.last().copied())
                .unwrap_or(f64::NAN),
        );
    }

    let probe_unit = plan
        .observables
        .iter()
        .find(|observable| observable.name == probe)
        .map_or_else(
            || Unit::linear("1", Dim::dimensionless(), 1.0),
            |observable| observable.unit.clone(),
        );
    let dataset = Dataset {
        dims: dimensions,
        columns: vec![Column {
            name: probe.to_owned(),
            unit: probe_unit,
            values: results,
        }],
    };
    let rows = dataset.rows();
    let csv = dataset.to_csv();

    match out {
        Some(path) => {
            std::fs::write(path, &csv).map_err(|error| {
                eprintln!("error: cannot write {}: {error}", path.display());
                1
            })?;
            println!("wrote {rows} points to {}", path.display());
        }
        None => print!("{csv}"),
    }
    Ok(())
}

fn explain(code: &str, json: bool) -> Result<(), u8> {
    let path = PathBuf::from(format!("docs/errors/{code}.md"));
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            if json {
                println!("{}", serde_json::json!({ "code": code, "text": text }));
            } else {
                println!("{text}");
            }
            Ok(())
        }
        Err(error) => {
            eprintln!("error: no explanation for {code} ({error})");
            Err(1)
        }
    }
}
