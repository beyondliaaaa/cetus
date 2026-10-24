#!/usr/bin/env node
// M2 exit criterion "10 万步运行时 UI 不卡顿" (§12 M2), the reproducible half.
//
// The UI cannot stall on the simulation by construction: wasm runs only in the
// worker (§10.2). What can regress is how long the worker takes to produce
// 100,000 macro steps and hand the probes back, which bounds how stale the
// chart gets. This benchmark measures exactly that through the browser-facing
// API, the same way apps/web/src/worker/sim.worker.ts drives it.
//
// Model: the SIR sample with dt = 0.001 day (horizon 160 day ⇒ 160,000 ticks,
// so 100,000 steps stay inside the horizon). Every observable has a probe, as
// the worker registers them.
//
// The budget is deliberately loose (CI runners are shared and slow): it exists
// to catch order-of-magnitude regressions, not to track milliseconds.
//
// Usage: node scripts/bench-wasm.cjs <cetus.js> [--budget-ms N]
//        CETUS_BENCH_BUDGET_MS=N also sets the budget.
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { performance } = require("node:perf_hooks");

const STEPS = 100_000;
const CHUNK = 1_000;
const DEFAULT_BUDGET_MS = 1_500;

const args = process.argv.slice(2);
const wasmJs = args[0] ? path.resolve(args[0]) : null;
if (!wasmJs) {
  console.error("usage: bench-wasm.cjs <cetus.js> [--budget-ms N]");
  process.exit(2);
}
const flag = args.indexOf("--budget-ms");
const budgetMs = Number(
  flag >= 0 ? args[flag + 1] : (process.env.CETUS_BENCH_BUDGET_MS ?? DEFAULT_BUDGET_MS),
);
if (!Number.isFinite(budgetMs) || budgetMs <= 0) {
  console.error("budget must be a positive number of milliseconds");
  process.exit(2);
}

const root = path.join(__dirname, "..");
const source = fs.readFileSync(
  path.join(root, "content/modules/bio/epidemic-sir/module.yaml"),
  "utf8",
);
const yaml = source.replace(/(\btime:\s*\{[^}]*\bdt:\s*)0\.05\b/, (_, prefix) => `${prefix}0.001`);
if (yaml === source) {
  console.error("could not set dt = 0.001 in the SIR model (its time line changed?)");
  process.exit(2);
}

const mod = require(wasmJs);
const compiled = mod.compileModel(yaml);
const meta = JSON.parse(compiled.metaJson());
const sim = compiled.start(1);
const probes = meta.observables.map((observable) => observable.name);
for (const name of probes) {
  sim.addProbe(name);
}

const t0 = performance.now();
let status;
for (let done = 0; done < STEPS; done += CHUNK) {
  status = JSON.parse(sim.step(CHUNK));
}
const t1 = performance.now();
let points = 0;
for (const name of probes) {
  points += sim.readProbe(name, 0).length / 2;
}
const t2 = performance.now();

sim.free();
compiled.free();

if (status.tick !== STEPS) {
  console.error(`expected to reach tick ${STEPS}, reached ${status.tick}`);
  process.exit(1);
}

const stepMs = t1 - t0;
const probeMs = t2 - t1;
const totalMs = t2 - t0;
console.log(
  `wasm bench: ${STEPS} RK4 steps in ${stepMs.toFixed(1)} ms ` +
    `(${CHUNK}/call), ${points} probe points read in ${probeMs.toFixed(1)} ms; ` +
    `total ${totalMs.toFixed(1)} ms, budget ${budgetMs} ms`,
);
if (totalMs > budgetMs) {
  console.error(`FAIL: over budget by ${(totalMs - budgetMs).toFixed(1)} ms`);
  process.exit(1);
}
