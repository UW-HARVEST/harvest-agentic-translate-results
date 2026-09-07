#!/usr/bin/env bash
# Full verification driver: builds the C .so, then runs the whole differential
# suite for every feature combination x every Rust artifact profile, and diffs
# nm -D output. Run from anywhere.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- C library
step "Build C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
echo "C  .so: $C_SO"

# ------------------------------------------------------- feature combinations
# Cargo.toml declares no [features] table, so the only combinations are the
# default (empty) set and --no-default-features. Extract them mechanically so
# this keeps working if features are ever added.
step "Enumerate feature combinations from Cargo.toml"
FEATS=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' "$CRATE/Cargo.toml" | grep -v '^default$' | tr '\n' ' ')
echo "declared non-default features: [${FEATS:-none}]"

COMBOS=("--all-features" "--no-default-features" "")
for f in $FEATS; do COMBOS+=("--no-default-features --features $f"); done

# ------------------------------------------------------------------ cargo check
step "cargo check every combination"
for c in "${COMBOS[@]}"; do
  ( cd "$CRATE" && timeout 600 cargo check --all-targets $c >/dev/null 2>&1 ) \
    && echo "  OK    cargo check ${c:-<default>}" \
    || { echo "  FAIL  cargo check ${c:-<default>}"; FAIL=1; }
done

# --------------------------------------------------- build both Rust artifacts
step "Build Rust cdylib (debug + release)"
( cd "$CRATE" && timeout 600 cargo build >/dev/null 2>&1 ) || { echo "debug build FAILED"; exit 1; }
( cd "$CRATE" && timeout 600 cargo build --release >/dev/null 2>&1 ) || { echo "release build FAILED"; exit 1; }
DBG_SO="$CRATE/target/debug/libldexp_q2_lib.so"
REL_SO="$CRATE/target/release/libldexp_q2_lib.so"

# ------------------------------------------------------------------ symbol diff
step "Phase D: nm -D symbol diff"
syms() { nm -D --defined-only "$1" | grep -vE ' [wWuU] ' | awk '{print $NF}' | sort -u; }
for rs in "$DBG_SO" "$REL_SO"; do
  MISSING="$(comm -23 <(syms "$C_SO") <(syms "$rs"))"
  if [ -z "$MISSING" ]; then
    echo "  OK    no C symbol missing from $(basename "$(dirname "$rs")")/$(basename "$rs")"
  else
    echo "  FAIL  missing from $rs:"; echo "$MISSING" | sed 's/^/          /'; FAIL=1
  fi
done

# ------------------------------------------------------- differential test runs
step "Phases B + C: differential tests, every combo x every artifact"
for c in "${COMBOS[@]}"; do
  for rs in "$DBG_SO" "$REL_SO"; do
    label="${c:-<default>} / $(basename "$(dirname "$rs")")"
    if ( cd "$CRATE" && RUST_SO="$rs" timeout 600 cargo test --release $c >/dev/null 2>&1 ); then
      echo "  OK    $label"
    else
      echo "  FAIL  $label"; FAIL=1
    fi
  done
done

# --------------------------------------------------------------------- driver
step "Binary/driver stdout comparison"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" \
   || [ -f "$CRATE/src/main.rs" ] || grep -q '^\[\[bin\]\]' "$CRATE/Cargo.toml"; then
  echo "  A driver exists -- stdout comparison required but not implemented"; FAIL=1
else
  echo "  N/A   neither project builds an executable (library only)"
fi

step "RESULT"
[ "$FAIL" -eq 0 ] && echo "ALL CHECKS PASSED" || echo "SOME CHECKS FAILED"
exit "$FAIL"
