#!/usr/bin/env bash
# Fails if an OS randomness source is in the dependency tree of a shipped
# artifact (D5, ADR-0001 "rand 的默认特性被关闭").
#
# The workspace builds `rand` without its OS-randomness features. Cargo unifies
# features across everything it builds together, so a single crate asking for
# `rand/thread_rng` (or depending on something that does) would quietly bring
# `getrandom` back. clippy.toml catches *calls* to `rand::rng()` and friends;
# this catches the *dependency* before anyone calls it.
#
# Scope: only the artifacts we ship or validate with, normal edges only.
# The workspace as a whole legitimately contains getrandom: proptest (used by
# cetus-testkit and dev-dependencies) and tempfile depend on it. Checking the
# whole workspace would always fail.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO="${CARGO:-cargo}"
FORBIDDEN="getrandom"

# package  target ("host" = the machine running the check)
ARTIFACTS=(
  "cetus-wasm wasm32-unknown-unknown"
  "cetus-cli host"
)

failures=0
for artifact in "${ARTIFACTS[@]}"; do
  read -r package target <<<"$artifact"
  # No --target means the host graph; only pass it for cross targets.
  target_args=()
  if [[ "$target" != "host" ]]; then
    target_args=(--target "$target")
  fi
  # `cargo tree -p X` resolves features exactly as `cargo build -p X` does,
  # which is how scripts/build-wasm.sh and check-cross-target.sh build them.
  tree="$("$CARGO" tree --locked -p "$package" ${target_args[@]+"${target_args[@]}"} -e normal \
    --prefix none --format '{p}')"
  if grep -qE "^${FORBIDDEN} v" <<<"$tree"; then
    echo "FAIL $package ($target): $FORBIDDEN is in the normal dependency tree" >&2
    echo "     which features pulled it in:" >&2
    "$CARGO" tree --locked -p "$package" ${target_args[@]+"${target_args[@]}"} -e normal,features \
      -i "$FORBIDDEN" >&2 || true
    failures=$((failures + 1))
  else
    echo "OK   $package ($target): no $FORBIDDEN"
  fi
done

if ((failures > 0)); then
  echo "FAIL: OS randomness reachable from $failures shipped artifact(s) (D5)." >&2
  echo "Use derived ChaCha8 substreams (cetus_kernel::rng) instead; enabling it needs an ADR." >&2
  exit 1
fi
