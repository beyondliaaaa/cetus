#!/usr/bin/env bash
# Cross-target determinism check (P2, §5.12, skills/cetus-determinism §3.1).
#
# 1. Compares the M0 hello digest between native and wasm.
# 2. Runs every sample model on both targets and compares the 64-bit state hash.
#
# Any mismatch is a P2 violation and must fail CI.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO="${CARGO:-cargo}"

native_digest="$("$CARGO" run -q -p cetus-cli -- hello | tail -1)"
echo "native hello: $native_digest"

"$ROOT/scripts/build-wasm.sh" nodejs --release >/dev/null

wasm_digest="$(node -e '
const mod = require(process.argv[1]);
process.stdout.write(mod.hello_digest());
' "$ROOT/target/wasm-pkg/nodejs/cetus.js")"
echo "wasm   hello: $wasm_digest"

if [[ "$native_digest" != "$wasm_digest" ]]; then
  echo "FAIL: native and wasm hello digests differ (P2 violated)" >&2
  exit 1
fi

failures=0
while IFS= read -r model; do
  native_hash="$("$CARGO" run -q -p cetus-cli -- run "$model" --hash --seed 1 | tail -1)"
  wasm_hash="$(node -e '
const fs = require("fs");
const mod = require(process.argv[1]);
const yaml = fs.readFileSync(process.argv[2], "utf8");
process.stdout.write(mod.run_model_digest(yaml, 1));
' "$ROOT/target/wasm-pkg/nodejs/cetus.js" "$model")"

  if [[ "$native_hash" == "$wasm_hash" ]]; then
    printf "OK   %-55s %s\n" "$(basename "$(dirname "$model")")" "$native_hash"
  else
    printf "FAIL %-55s\n  native: %s\n  wasm:   %s\n" "$model" "$native_hash" "$wasm_hash" >&2
    failures=$((failures + 1))
  fi
done < <(find content/modules -name module.yaml | sort)

if (( failures > 0 )); then
  echo "FAIL: $failures model(s) diverged between native and wasm" >&2
  exit 1
fi

echo "OK: native and wasm agree on every sample model"
