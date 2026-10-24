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
#   * wasm-opt must be binaryen version_123 (WASM_OPT_VERSION below).
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
# Must match the binaryen release installed in .github/workflows/ci.yml.
WASM_OPT_VERSION="123"

# rustup installs its proxies in ~/.cargo/bin but only interactive login shells
# source ~/.cargo/env. Fall back to the default location so `npm run dev` works
# from IDEs and non-login shells too.
if ! command -v "$CARGO" >/dev/null 2>&1; then
  if [[ -x "$HOME/.cargo/bin/cargo" ]]; then
    CARGO="$HOME/.cargo/bin/cargo"
  else
    echo "error: cargo not found on PATH (install Rust via rustup)" >&2
    echo "       or run: export PATH=\"\$HOME/.cargo/bin:\$PATH\"" >&2
    exit 2
  fi
fi

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

# wasm-opt is pinned too (ADR-0001): its table handling changed between
# binaryen releases, and an older binaryen breaks the wasm-bindgen externref
# table at load time. Skipping it would also ship a different bundle from CI.
# Set CETUS_ALLOW_MISSING_WASM_OPT=1 to build an unoptimised bundle anyway
# (local debugging only; never in CI).
if command -v "$WASM_OPT" >/dev/null 2>&1; then
  wasm_opt_version="$("$WASM_OPT" --version | grep -oE 'version [0-9]+' | head -1 | awk '{print $2}' || true)"
  if [[ "$wasm_opt_version" != "$WASM_OPT_VERSION" ]]; then
    echo "error: wasm-opt version '${wasm_opt_version:-unknown}' != pinned $WASM_OPT_VERSION (binaryen version_$WASM_OPT_VERSION, ADR-0001)" >&2
    exit 2
  fi
  RUN_WASM_OPT=1
elif [[ "${CETUS_ALLOW_MISSING_WASM_OPT:-0}" == "1" ]]; then
  echo "warning: wasm-opt not found; CETUS_ALLOW_MISSING_WASM_OPT=1, skipping optimisation" >&2
  RUN_WASM_OPT=0
else
  echo "error: wasm-opt not found on PATH (install binaryen version_$WASM_OPT_VERSION, ADR-0001)" >&2
  echo "       set CETUS_ALLOW_MISSING_WASM_OPT=1 to build an unoptimised bundle for local debugging" >&2
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

if [[ "$RUN_WASM_OPT" == "1" ]]; then
  # -O2 only: never enable flags that relax floating-point semantics.
  "$WASM_OPT" -O2 "$OUT_DIR/cetus_bg.wasm" -o "$OUT_DIR/cetus_bg.opt.wasm"
  mv "$OUT_DIR/cetus_bg.opt.wasm" "$OUT_DIR/cetus_bg.wasm"
fi

echo "wasm bundle written to $OUT_DIR"
