//! Snapshot round-trip: restoring at an arbitrary tick and continuing must
//! produce bit-identical state (§5.11, D12).

use std::path::PathBuf;
use std::sync::Arc;

use cetus_compile::compile_model;
use cetus_kernel::Sim;
use cetus_model::Model;
use cetus_sd::SdSubsystem;
use proptest::prelude::*;

fn plan() -> Arc<cetus_compile::CompiledPlan> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/modules/energy/solar-battery/module.yaml");
    let source = std::fs::read_to_string(path).expect("read model");
    let model = Model::from_yaml(&source).expect("parse");
    Arc::new(compile_model(&model).expect("compile"))
}

fn sim(plan: Arc<cetus_compile::CompiledPlan>, seed: u64) -> Sim {
    Sim::new(plan.clone(), seed, Box::new(SdSubsystem::new(plan))).expect("sim")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn snapshot_roundtrip(split in 0u64..600, rest in 1u64..600, seed in any::<u64>()) {
        let plan = plan();

        let mut direct = sim(plan.clone(), seed);
        direct.run_until(split + rest).expect("direct");
        let expected = direct.state_hash();

        let mut interrupted = sim(plan.clone(), seed);
        interrupted.run_until(split).expect("first half");
        let bytes = interrupted.snapshot().expect("snapshot");
        let mut restored = Sim::restore(plan.clone(), Box::new(SdSubsystem::new(plan.clone())), &bytes).expect("restore");
        restored.run_until(split + rest).expect("second half");

        prop_assert_eq!(restored.state_hash(), expected);
    }
}
