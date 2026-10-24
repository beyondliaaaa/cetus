#!/usr/bin/env node
// Runs a model through the browser-facing wasm API — the same calls the
// workbench worker makes (compileModel → start → step … until done) — and
// prints the final state hash. Used by scripts/check-cross-target.sh to check
// that chunked stepping in wasm matches the golden hashes, not just the
// one-shot `run_model_digest` export.
//
// Usage: node scripts/wasm-step-digest.cjs <cetus.js> <module.yaml> <chunk> [seed]
"use strict";

const fs = require("node:fs");

const [wasmJs, modelPath, chunkArg, seedArg = "1"] = process.argv.slice(2);
if (!wasmJs || !modelPath || !chunkArg) {
  console.error("usage: wasm-step-digest.cjs <cetus.js> <module.yaml> <chunk> [seed]");
  process.exit(2);
}
const chunk = Number(chunkArg);
const seed = Number(seedArg);
if (!Number.isInteger(chunk) || chunk < 1 || !Number.isInteger(seed) || seed < 0) {
  console.error(`invalid chunk (${chunkArg}) or seed (${seedArg})`);
  process.exit(2);
}

const mod = require(wasmJs);
const yaml = fs.readFileSync(modelPath, "utf8");

let compiled;
let sim;
try {
  compiled = mod.compileModel(yaml);
  sim = compiled.start(seed);
  // Bounded so a model without a reachable horizon fails instead of hanging.
  for (let calls = 0; ; calls += 1) {
    if (calls > 10_000_000) {
      throw new Error("model did not reach its horizon");
    }
    const status = JSON.parse(sim.step(chunk));
    if (status.done === true) {
      break;
    }
  }
  process.stdout.write(sim.stateHash());
} catch (error) {
  console.error(`${modelPath}: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
} finally {
  sim?.free();
  compiled?.free();
}
