#!/usr/bin/env bash
# Cross-target determinism check (P2, §5.12, skills/cetus-determinism §3.1).
#
# 1. Compares the M0 hello digest between native and wasm.
# 2. Runs every sample model on both targets and compares the state hash, in
#    wasm both one-shot (run_model_digest) and through the workbench API in
#    chunks (scripts/wasm-step-digest.cjs).
# 3. Compares every hash against crates/cetus-testkit/golden/hashes.txt, so a
#    change that moves native and wasm *together* (toolchain or dependency
#    bump) also fails.
#
# Native runs the release profile, the same profile the wasm bundle ships with.
#
# Any mismatch is a P2 violation and must fail CI.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO="${CARGO:-cargo}"
GOLDEN_FILE="$ROOT/crates/cetus-testkit/golden/hashes.txt"

# Prints the golden hash for a key, or nothing if the key is not recorded.
golden() {
  awk -v key="$1" '$0 !~ /^[[:space:]]*(#|$)/ && $1 == key { print $2; exit }' "$GOLDEN_FILE"
}

failures=0

# Reports one result line and counts failures.
#   compare <label> <golden-key> <native> <wasm>
compare() {
  local label="$1" key="$2" native="$3" wasm="$4" expected
  expected="$(golden "$key")"
  if [[ -z "$expected" ]]; then
    printf "FAIL %-34s no golden entry for %s in %s\n" "$label" "$key" "$GOLDEN_FILE" >&2
    printf "     native: %s\n     wasm:   %s\n" "$native" "$wasm" >&2
    failures=$((failures + 1))
  elif [[ "$native" != "$wasm" ]]; then
    printf "FAIL %-34s native and wasm differ\n     native: %s\n     wasm:   %s\n" \
      "$label" "$native" "$wasm" >&2
    failures=$((failures + 1))
  elif [[ "$native" != "$expected" ]]; then
    printf "FAIL %-34s both targets agree but differ from the golden\n     got:    %s\n     golden: %s\n" \
      "$label" "$native" "$expected" >&2
    failures=$((failures + 1))
  else
    printf "OK   %-34s %s\n" "$label" "$native"
  fi
}

"$CARGO" build -q --release -p cetus-cli
CETUS="$ROOT/target/release/cetus"

"$ROOT/scripts/build-wasm.sh" nodejs --release >/dev/null
WASM_JS="$ROOT/target/wasm-pkg/nodejs/cetus.js"

native_digest="$("$CETUS" hello | tail -1)"
wasm_digest="$(node -e '
const mod = require(process.argv[1]);
process.stdout.write(mod.hello_digest());
' "$WASM_JS")"
compare "hello" "hello" "$native_digest" "$wasm_digest"

while IFS= read -r model; do
  native_hash="$("$CETUS" run "$model" --hash --seed 1 | tail -1)"
  wasm_hash="$(node -e '
const fs = require("fs");
const mod = require(process.argv[1]);
const yaml = fs.readFileSync(process.argv[2], "utf8");
process.stdout.write(mod.run_model_digest(yaml, 1));
' "$WASM_JS" "$model")"
  name="$(basename "$(dirname "$model")")"
  compare "$name" "$model" "$native_hash" "$wasm_hash"

  # The workbench never calls run_model_digest: it drives compileModel/start/
  # step in small chunks. That path must land on the same bits. 997 is prime,
  # so chunk boundaries fall at varying offsets within each model's ticks.
  wasm_step_hash="$(node "$ROOT/scripts/wasm-step-digest.cjs" "$WASM_JS" "$model" 997 1)"
  compare "$name (wasm step)" "$model" "$native_hash" "$wasm_step_hash"
done < <(find content/modules -name module.yaml | sort)

if ((failures > 0)); then
  echo "FAIL: $failures determinism check(s) failed (P2 violated)" >&2
  echo "If a hash moved on purpose, update $GOLDEN_FILE and explain why in the PR (ADR-0001)." >&2
  exit 1
fi

echo "OK: native, wasm and golden agree on the hello digest and every sample model"
