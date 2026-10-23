#!/usr/bin/env bash
# Build the Cetus wasm bindings and run wasm-bindgen + wasm-opt on them.
#
# Usage:
#   scripts/build-wasm.sh [web|nodejs] [--release|--debug] [--out-dir DIR]
#
# Defaults: target nodejs, release profile, output target/wasm-pkg/<target>.
#
# Determinism notes (ADR-0001, §5.12):
#   * wasm-bindgen CLI must match the crate version exactly.
#   * wasm-opt must NOT be given --fast-math or any flag that changes
#     floating-point semantics.
#   * do not build with -C target-cpu=native.
set -euo pipefail

TARGET_KIND="nodejs"
PROFILE="release"
OUT_DIR=""

while (($#)); do
  case "$1" in
    web|nodejs) TARGET_KIND="$1" ;;
    --release) PROFILE="release" ;;
    --debug) PROFILE="debug" ;;
    --out-dir) OUT_DIR="$2"; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO="${CARGO:-cargo}"
WASM_BINDGEN="${WASM_BINDGEN:-wasm-bindgen}"
WASM_OPT="${WASM_OPT:-wasm-opt}"

if ! command -v "$WASM_BINDGEN" >/dev/null 2>&1; then
  echo "error: wasm-bindgen not found on PATH (install the CLI matching the crate version)" >&2
  exit 2
fi

crate_version="$(python3 - "$ROOT/Cargo.lock" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
m = re.search(r'name = "wasm-bindgen"\nversion = "([^"]+)"', text)
print(m.group(1) if m else "")
PY
)"
cli_version="$("$WASM_BINDGEN" --version | awk '{print $2}')"

if [[ -z "$crate_version" ]]; then
  echo "error: could not read wasm-bindgen version from Cargo.lock (run cargo fetch first)" >&2
  exit 2
fi
if [[ "$crate_version" != "$cli_version" ]]; then
  echo "error: wasm-bindgen crate $crate_version != CLI $cli_version" >&2
  exit 2
fi

if [[ "$PROFILE" == "release" ]]; then
  PROFILE_FLAG="--release"
  WASM_DIR="release"
else
  PROFILE_FLAG=""
  WASM_DIR="debug"
fi

"$CARGO" build -p cetus-wasm --target wasm32-unknown-unknown $PROFILE_FLAG

OUT_DIR="${OUT_DIR:-$ROOT/target/wasm-pkg/$TARGET_KIND}"
mkdir -p "$OUT_DIR"

"$WASM_BINDGEN" \
  --target "$TARGET_KIND" \
  --out-dir "$OUT_DIR" \
  --out-name cetus \
  "$ROOT/target/wasm32-unknown-unknown/$WASM_DIR/cetus_wasm.wasm"

if command -v "$WASM_OPT" >/dev/null 2>&1; then
  # -O2 only: never enable flags that relax floating-point semantics.
  "$WASM_OPT" -O2 "$OUT_DIR/cetus_bg.wasm" -o "$OUT_DIR/cetus_bg.opt.wasm"
  mv "$OUT_DIR/cetus_bg.opt.wasm" "$OUT_DIR/cetus_bg.wasm"
else
  echo "warning: wasm-opt not found; skipping size optimisation" >&2
fi

echo "wasm bundle written to $OUT_DIR"
