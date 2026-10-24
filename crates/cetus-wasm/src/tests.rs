//! Native tests for the wasm binding logic.

use serde_json::Value;

use crate::{edit, expressions, meta, model, sim};

const SIR: &str = include_str!("../../../content/modules/bio/epidemic-sir/module.yaml");
const SOLAR: &str = include_str!("../../../content/modules/energy/solar-battery/module.yaml");

fn compile(source: &str) -> crate::CompiledModel {
    model::compile(source).expect("model compiles")
}

fn start(compiled: &crate::CompiledModel) -> crate::SimHandle {
    model::start(compiled, 1).expect("start")
}

fn meta(compiled: &crate::CompiledModel) -> Value {
    serde_json::from_str(&meta::meta_json(compiled)).expect("meta JSON")
}

fn json(value: &str) -> Value {
    serde_json::from_str(value).expect("JSON")
}

#[test]
fn compiles_the_sir_metadata() {
    let compiled = compile(SIR);
    let meta = meta(&compiled);
    assert_eq!(meta["id"], "bio.epidemic-sir");
    assert_eq!(meta["views"], serde_json::json!(["series", "phase"]));
    assert_eq!(meta["parameters"].as_array().expect("parameters").len(), 2);
    assert_eq!(meta["stocks"].as_array().expect("stocks").len(), 3);
    assert_eq!(meta["flows"].as_array().expect("flows").len(), 2);
    assert_eq!(
        meta["observables"].as_array().expect("observables").len(),
        2
    );
}

#[test]
fn exposes_solar_display_units() {
    let compiled = compile(SOLAR);
    let meta = meta(&compiled);
    let area = meta["parameters"]
        .as_array()
        .expect("parameters")
        .iter()
        .find(|parameter| parameter["name"] == "area")
        .expect("area");
    assert_eq!(area["unitFactor"].as_f64().expect("factor"), 1.0);
    assert_eq!(area["siValue"].as_f64().expect("si value"), 12.0);

    let charge = meta["stocks"]
        .as_array()
        .expect("stocks")
        .iter()
        .find(|stock| stock["name"] == "charge")
        .expect("charge");
    assert_eq!(charge["unitFactor"].as_f64().expect("factor"), 3.6e6);
}

#[test]
fn steps_and_reads_probes() {
    let compiled = compile(SIR);
    let mut running = start(&compiled);
    sim::add_probe(&mut running, "infectious").expect("probe");
    let report = json(&sim::step(&mut running, 10).expect("step"));
    assert!(report["observables"]["infectious"].as_f64().is_some());

    let probe = sim::read_probe(&running, "infectious", 0.0).expect("read probe");
    assert_eq!(probe.len(), 20);
    assert_eq!(probe[0], 1.0);
}

#[test]
fn runs_to_the_declared_horizon() {
    let compiled = compile(SIR);
    let mut running = start(&compiled);
    let report = json(&sim::run_until(&mut running, 160.0).expect("run"));
    assert_eq!(report["done"], true);
    assert_eq!(report["tick"].as_f64().expect("tick"), 3200.0);
}

#[test]
fn validates_parameter_units_and_ranges() {
    let compiled = compile(SIR);
    let mut running = start(&compiled);
    assert!(sim::set_param(&mut running, "transmission", 0.0001, None).is_ok());
    assert!(sim::set_param(&mut running, "transmission", 1.0, None).is_err());

    let solar = compile(SOLAR);
    let mut solar_sim = start(&solar);
    assert!(sim::set_param(&mut solar_sim, "area", 200.0, Some("m^2")).is_err());
}

#[test]
fn restores_snapshots_with_probes() {
    let compiled = compile(SIR);
    let mut running = start(&compiled);
    sim::add_probe(&mut running, "infectious").expect("probe");
    sim::step(&mut running, 10).expect("step");
    let hash = sim::state_hash(&running);
    let tick = sim::tick(&running);
    let bytes = sim::snapshot(&running).expect("snapshot");

    let restored = model::restore(&compiled, &bytes).expect("restore");
    assert_eq!(sim::state_hash(&restored), hash);
    assert_eq!(sim::tick(&restored), tick);
    assert_eq!(
        sim::read_probe(&restored, "infectious", 0.0)
            .expect("probe")
            .len(),
        20
    );
}

#[test]
fn explains_why_a_snapshot_does_not_fit() {
    let sir = compile(SIR);
    let mut running = start(&sir);
    sim::step(&mut running, 3).expect("step");
    let bytes = sim::snapshot(&running).expect("snapshot");

    // Another model entirely.
    let solar = compile(SOLAR);
    let error = model::restore(&solar, &bytes).expect_err("wrong model");
    assert!(
        error.contains("belongs to model `bio.epidemic-sir`"),
        "{error}"
    );

    // The same model after an edit: same id, different content.
    let edited_yaml = edit::edit_model(
        &sir,
        r#"{"op":"set_eq","section":"flows","name":"infection","eq":"0"}"#,
    )
    .expect("edit");
    let edited = compile(&edited_yaml);
    let error = model::restore(&edited, &bytes).expect_err("edited model");
    assert!(error.contains("different version"), "{error}");

    // Garbage is still a readable error, not a crash.
    let mut huge = b"CETUSS\x01\x00".to_vec();
    huge.extend_from_slice(&u32::MAX.to_le_bytes());
    let error = model::restore(&sir, &huge).expect_err("oversized");
    assert!(error.contains("limit"), "{error}");
}

#[test]
fn queries_and_checks_expressions() {
    let compiled = compile(SIR);
    let running = start(&compiled);

    let query = json(&sim::query(
        &running,
        "susceptible + infectious + recovered",
    ));
    assert_eq!(query["ok"], true);
    let si_value = query["siValue"].as_f64().expect("si value");
    assert!((si_value - 1000.0).abs() < 1e-9);

    let checked = json(&expressions::parse_expression(
        &compiled,
        "transmission * susceptible * infectious",
        Some("person/day"),
    ));
    assert_eq!(checked["matchesExpected"], true);

    let invalid = json(&expressions::parse_expression(
        &compiled,
        "2[m] + 3[s]",
        None,
    ));
    assert_eq!(invalid["ok"], false);
    assert_eq!(invalid["code"], "E1201");
}

#[test]
fn edits_models_and_recompiles() {
    let compiled = compile(SIR);
    let yaml = edit::edit_model(
        &compiled,
        r#"{"op":"set_eq","section":"flows","name":"infection","eq":"0"}"#,
    )
    .expect("edit");
    let edited = compile(&yaml);
    let meta = meta(&edited);
    assert_eq!(meta["flows"][0]["eq"], "0");
}

#[test]
fn rejects_invalid_edits() {
    let compiled = compile(SIR);
    assert!(edit::edit_model(&compiled, "not json").is_err());
    assert!(
        edit::edit_model(
            &compiled,
            r#"{"op":"set_value","name":"missing","value":1}"#
        )
        .is_err()
    );
}

/// Golden hashes shared with `scripts/check-cross-target.sh` (ADR-0001).
const GOLDEN: &str = include_str!("../../cetus-testkit/golden/hashes.txt");

/// Every sample model, keyed by the path used in `GOLDEN`.
const SAMPLE_MODELS: [(&str, &str); 8] = [
    (
        "content/modules/bio/epidemic-sir/module.yaml",
        include_str!("../../../content/modules/bio/epidemic-sir/module.yaml"),
    ),
    (
        "content/modules/bio/population-logistic/module.yaml",
        include_str!("../../../content/modules/bio/population-logistic/module.yaml"),
    ),
    (
        "content/modules/bio/predator-prey/module.yaml",
        include_str!("../../../content/modules/bio/predator-prey/module.yaml"),
    ),
    (
        "content/modules/energy/solar-battery/module.yaml",
        include_str!("../../../content/modules/energy/solar-battery/module.yaml"),
    ),
    (
        "content/modules/eng/inventory-oscillation/module.yaml",
        include_str!("../../../content/modules/eng/inventory-oscillation/module.yaml"),
    ),
    (
        "content/modules/math/decay/module.yaml",
        include_str!("../../../content/modules/math/decay/module.yaml"),
    ),
    (
        "content/modules/math/logistic-map/module.yaml",
        include_str!("../../../content/modules/math/logistic-map/module.yaml"),
    ),
    (
        "content/modules/phys/rc-circuit/module.yaml",
        include_str!("../../../content/modules/phys/rc-circuit/module.yaml"),
    ),
];

fn golden(key: &str) -> &'static str {
    GOLDEN
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .find_map(|line| {
            let (k, v) = line.split_once(char::is_whitespace)?;
            (k == key).then(|| v.trim())
        })
        .unwrap_or_else(|| panic!("golden/hashes.txt has no entry for `{key}`"))
}

/// Calls `next` until the simulation reports `done`; returns the final state hash.
fn run_in_chunks(source: &str, mut next: impl FnMut(&mut crate::SimHandle) -> Value) -> String {
    let compiled = compile(source);
    let mut running = start(&compiled);
    // Generous bound: the largest sample model has a few thousand ticks.
    for _ in 0..10_000_000 {
        let report = next(&mut running);
        if report["done"] == true {
            return sim::state_hash(&running);
        }
    }
    panic!("model did not reach its horizon");
}

/// The browser advances in small `step(n)` / `runUntil(t)` calls, while the
/// golden hashes come from one `run_until(horizon)`. Chunking must not change
/// a single bit, or what the player sees would differ from what was verified.
#[test]
fn chunked_stepping_matches_the_golden_hashes() {
    for (path, source) in SAMPLE_MODELS {
        let expected = golden(path);
        for chunk in [1_u32, 7, 1000] {
            let hash = run_in_chunks(source, |running| {
                json(&sim::step(running, chunk).expect("step"))
            });
            assert_eq!(hash, expected, "{path}: step({chunk}) chunks");
        }

        // Mixed: alternate single steps with runUntil over a moving display time,
        // as playback does (pause, step, resume).
        let mut turn = 0_u32;
        let hash = run_in_chunks(source, |running| {
            turn += 1;
            if turn.is_multiple_of(2) {
                json(&sim::step(running, 1).expect("step"))
            } else {
                let now = sim::display_time(running);
                let advance = 0.37 * f64::from(turn % 5 + 1);
                json(&sim::run_until(running, now + advance).expect("run until"))
            }
        });
        assert_eq!(hash, expected, "{path}: mixed step/runUntil");
    }
}
