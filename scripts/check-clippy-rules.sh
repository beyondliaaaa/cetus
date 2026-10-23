#!/usr/bin/env bash
# Verifies that the determinism rules in clippy.toml actually fire
# (§12 M0, skills/cetus-determinism §4, ADR-0001).
#
# Runs clippy on tools/clippy-probe, which deliberately uses every forbidden
# std item, and fails unless every `std`/`core` rule in clippy.toml produced a
# disallowed_types / disallowed_methods warning. Rules marked
# `allow-invalid = true` (third-party paths) are skipped: they only resolve when
# the crate and feature are present.
#
# A rule that silently stops matching (path syntax change in a clippy upgrade, a
# typo, a renamed std item) is invisible in normal CI, because nothing in the
# real code uses the forbidden item. This script makes it visible.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO="${CARGO:-cargo}"
PYTHON="${PYTHON:-python3}"

# The rule list is read with tomllib, which is Python 3.11+. macOS ships 3.9 as
# /usr/bin/python3, so say so plainly instead of failing with an ImportError.
if ! "$PYTHON" -c 'import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)' 2>/dev/null; then
  if command -v "$PYTHON" >/dev/null 2>&1; then
    found="$("$PYTHON" --version 2>&1)"
  else
    found="no '$PYTHON' on PATH"
  fi
  echo "error: scripts/check-clippy-rules.sh needs Python 3.11+ (for tomllib); found: $found" >&2
  echo "       install a newer Python, or point PYTHON at one: PYTHON=python3.12 $0" >&2
  exit 2
fi

OUT="$(mktemp)"
trap 'rm -f "$OUT"' EXIT

# The probe is supposed to produce warnings: do not let a global
# `RUSTFLAGS=-D warnings` turn them into a failed build.
env -u RUSTFLAGS \
  CLIPPY_CONF_DIR="$ROOT" \
  CARGO_TARGET_DIR="$ROOT/target/clippy-probe" \
  "$CARGO" clippy --quiet --manifest-path tools/clippy-probe/Cargo.toml \
  --message-format=json >"$OUT"

"$PYTHON" - "$ROOT/clippy.toml" "$OUT" <<'PY'
import json, re, sys, tomllib

config_path, output_path = sys.argv[1], sys.argv[2]
with open(config_path, "rb") as f:
    config = tomllib.load(f)

expected = {}
for kind in ("disallowed-types", "disallowed-methods"):
    for entry in config.get(kind, []):
        if entry.get("allow-invalid", False):
            continue
        expected[entry["path"]] = kind

hits = set()
pattern = re.compile(r"use of a disallowed (?:type|method) `([^`]+)`")
with open(output_path, encoding="utf-8") as f:
    for line in f:
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if record.get("reason") != "compiler-message":
            continue
        message = record["message"]
        code = (message.get("code") or {}).get("code", "")
        if code not in ("clippy::disallowed_types", "clippy::disallowed_methods"):
            continue
        match = pattern.search(message.get("message", ""))
        if match:
            hits.add(match.group(1))

missing = sorted(set(expected) - hits)
if missing:
    print("FAIL: these clippy.toml rules did not fire on tools/clippy-probe:", file=sys.stderr)
    for path in missing:
        print(f"  {path}  ({expected[path]})", file=sys.stderr)
    print(
        "Either the path no longer resolves (check the clippy docs for the current "
        "syntax and update ADR-0001), or the probe does not use it yet.",
        file=sys.stderr,
    )
    sys.exit(1)

print(f"OK: all {len(expected)} std determinism rules in clippy.toml fire")
PY
