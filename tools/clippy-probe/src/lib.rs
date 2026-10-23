//! Probe for the determinism rules in the root `clippy.toml`.
//!
//! Every function below uses something the kernel must never use. Clippy has to
//! flag each one; `scripts/check-clippy-rules.sh` checks that it does. When you
//! add a rule to `clippy.toml`, add a use of it here.

#![allow(dead_code, clippy::pedantic)]

use std::collections::{HashMap, HashSet};

pub fn unordered_containers() -> usize {
    let map: HashMap<u8, u8> = HashMap::new();
    let set: HashSet<u8> = HashSet::new();
    map.len() + set.len()
}

pub fn f64_transcendentals(x: f64) -> f64 {
    let (s, c) = x.sin_cos();
    x.sin()
        + x.cos()
        + x.tan()
        + s
        + c
        + x.asin()
        + x.acos()
        + x.atan()
        + x.atan2(1.0)
        + x.sinh()
        + x.cosh()
        + x.tanh()
        + x.asinh()
        + x.acosh()
        + x.atanh()
        + x.exp()
        + x.exp2()
        + x.exp_m1()
        + x.ln()
        + x.ln_1p()
        + x.log(3.0)
        + x.log10()
        + x.log2()
        + x.powf(1.5)
        + x.powi(3)
        + x.sqrt()
        + x.cbrt()
        + x.hypot(2.0)
        + x.mul_add(2.0, 1.0)
}

pub fn f32_transcendentals(x: f32) -> f32 {
    let (s, c) = x.sin_cos();
    x.sin()
        + x.cos()
        + x.tan()
        + s
        + c
        + x.asin()
        + x.acos()
        + x.atan()
        + x.atan2(1.0)
        + x.sinh()
        + x.cosh()
        + x.tanh()
        + x.asinh()
        + x.acosh()
        + x.atanh()
        + x.exp()
        + x.exp2()
        + x.exp_m1()
        + x.ln()
        + x.ln_1p()
        + x.log(3.0)
        + x.log10()
        + x.log2()
        + x.powf(1.5)
        + x.powi(3)
        + x.sqrt()
        + x.cbrt()
        + x.hypot(2.0)
        + x.mul_add(2.0, 1.0)
}

pub fn wall_clock() {
    let _ = std::time::Instant::now();
    let _ = std::time::SystemTime::now();
}
