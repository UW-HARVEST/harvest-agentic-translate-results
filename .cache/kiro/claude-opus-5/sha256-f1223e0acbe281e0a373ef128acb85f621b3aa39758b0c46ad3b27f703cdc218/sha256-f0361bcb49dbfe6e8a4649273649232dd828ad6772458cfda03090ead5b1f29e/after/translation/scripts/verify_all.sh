#!/usr/bin/env bash
# Full verification driver: builds both libraries, diffs the exported symbol
# sets, and runs the whole differential suite under EVERY cargo feature
# combination (enumerated from Cargo.toml, not hard-coded).
#
#   ./scripts/verify_all.sh
#
# Every command is wrapped in `timeout` so no single step can hang the run.

set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
C_DIR="$ROOT/c_src"
TIMEOUT=600
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf '!!! FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
step "Building the C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$C_DIR/build"
( cd "$C_DIR/build" \
  && timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout $TIMEOUT cmake --build . >/dev/null ) \
  || { fail "C build"; exit 1; }

C_SO="$(find "$C_DIR/build" -maxdepth 1 -name 'lib*.so' | head -n1)"
[ -n "$C_SO" ] || { fail "no C .so produced"; exit 1; }
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
step "Enumerating cargo feature combinations"
# ---------------------------------------------------------------------------
# Features are read from Cargo.toml's [features] table (excluding `default`).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblk=1; next }
    /^\[/           { inblk=0 }
    inblk && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE_DIR/Cargo.toml"
)

N=${#FEATURES[@]}
echo "optional features found: $N ${FEATURES[*]:-（none）}"

# Build the list of combinations to test. With no features the only
# configuration is the default build.
COMBOS=()
if [ "$N" -eq 0 ]; then
  COMBOS+=("default")
else
  COMBOS+=("default")
  COMBOS+=("--no-default-features")
  # full power set of the optional features
  for ((mask = 1; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("--no-default-features --features $combo")
    COMBOS+=("--features $combo")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "default" ]; then FLAGS=(); else read -r -a FLAGS <<<"$combo"; fi

  step "Configuration: $combo -- cargo check"
  ( cd "$CRATE_DIR" && timeout $TIMEOUT cargo check --release "${FLAGS[@]}" 2>&1 | tail -5 ) \
    || fail "cargo check [$combo]"

  step "Configuration: $combo -- building the Rust cdylib"
  ( cd "$CRATE_DIR" && timeout $TIMEOUT cargo build --release "${FLAGS[@]}" 2>&1 | tail -3 ) \
    || fail "cargo build [$combo]"

  R_SO="$CRATE_DIR/target/release/libmaxnmin_lib.so"
  [ -f "$R_SO" ] || { fail "no Rust .so [$combo]"; continue; }

  step "Configuration: $combo -- symbol diff (nm -D)"
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort >/tmp/hv_c.syms
  nm -D --defined-only "$R_SO" | awk '{print $3}' | grep -v '^_ZN' | sort >/tmp/hv_r.syms
  MISSING="$(comm -23 /tmp/hv_c.syms /tmp/hv_r.syms)"
  printf 'C exports:    %s\nRust exports: %s\n' \
    "$(wc -l </tmp/hv_c.syms)" "$(wc -l </tmp/hv_r.syms)"
  if [ -n "$MISSING" ]; then
    fail "symbols exported by C but MISSING from Rust [$combo]:"
    echo "$MISSING"
  else
    echo "symbol diff: EMPTY (all C symbols present in Rust)"
  fi

  step "Configuration: $combo -- differential test suite"
  ( cd "$CRATE_DIR" && timeout $TIMEOUT cargo test --release "${FLAGS[@]}" 2>&1 | tail -25 ) \
    || fail "cargo test [$combo]"
done

# ---------------------------------------------------------------------------
step "Binary / driver executable comparison"
# ---------------------------------------------------------------------------
if grep -q '^\[\[bin\]\]' "$CRATE_DIR/Cargo.toml" \
   || [ -d "$CRATE_DIR/src/bin" ] \
   || grep -q 'add_executable' "$C_DIR/CMakeLists.txt"; then
  fail "a binary target appeared -- stdout comparison must be added here"
else
  echo "no binary target on either side (no add_executable, no [[bin]], no src/bin)"
  echo "=> nothing to compare; requirement not applicable"
fi

# ---------------------------------------------------------------------------
step "Summary"
# ---------------------------------------------------------------------------
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT (see !!! lines above)"
fi
exit $FAILED
