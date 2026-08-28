#!/usr/bin/env bash
# Full verification sweep: enumerates every feature combination declared in
# Cargo.toml and runs cargo check + the differential test suite for each, then
# sweeps the harness build matrix (C optimization level x Rust profile) and
# compares exported symbols.
#
# Usage: ./verify.sh          (from translation/)
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
LOGDIR="${TMPDIR:-/tmp}/jumpnode-verify"
mkdir -p "$LOGDIR"
TIMEOUT=600
rc=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; rc=1; }

# --- 1. Enumerate feature combinations ------------------------------------
# Read the [features] table from Cargo.toml and build the powerset. Optional
# dependencies (implicit features) are not used by this crate.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /=/   { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "" && a[1] !~ /^#/) print a[1] }
  ' Cargo.toml
)

step "Declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=("")   # always include the empty combination
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
step "Feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - ${c:-<no features>}"; done

# --- 2. Build the C shared library ----------------------------------------
step "Building C shared library"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
) > "$LOGDIR/cmake.log" 2>&1 || { fail "C build (see $LOGDIR/cmake.log)"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "C library: $C_SO"

# --- 3. cargo check + test per feature combination ------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-no-features}"
  safe="${label//,/_}"

  step "cargo check --no-default-features --features '$combo'  [$label]"
  if timeout $TIMEOUT cargo check --no-default-features ${combo:+--features "$combo"} \
       > "$LOGDIR/check-$safe.log" 2>&1; then
    echo "  check OK"
  else
    fail "cargo check [$label]"; tail -30 "$LOGDIR/check-$safe.log"; continue
  fi

  # Build both profiles so the differential test exercises the debug and the
  # release cdylib, then run the suite under each profile.
  for prof in dev release; do
    step "cargo test ($prof) --no-default-features --features '$combo'  [$label]"
    profflag=(--profile "$prof"); [[ $prof == dev ]] && profflag=()
    if timeout $TIMEOUT cargo build "${profflag[@]}" --no-default-features \
         ${combo:+--features "$combo"} > "$LOGDIR/build-$safe-$prof.log" 2>&1; then
      :
    else
      fail "cargo build $prof [$label]"; tail -30 "$LOGDIR/build-$safe-$prof.log"; continue
    fi

    testflag=(); [[ $prof == release ]] && testflag=(--release)
    if timeout $TIMEOUT cargo test "${testflag[@]}" --no-default-features \
         ${combo:+--features "$combo"} > "$LOGDIR/test-$safe-$prof.log" 2>&1; then
      grep -E '^test result:' "$LOGDIR/test-$safe-$prof.log" | sed 's/^/  /'
    else
      fail "cargo test $prof [$label]"
      grep -E 'panicked|FAILED|^test result:|error' "$LOGDIR/test-$safe-$prof.log" | head -30
    fi
  done

  # --- 4. Symbol comparison ----------------------------------------------
  step "Symbol comparison [$label]"
  for prof in debug release; do
    R_SO="target/$prof/libjumpnode_lib.so"
    [[ -f $R_SO ]] || continue
    missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TtDBRWVGS]$/ {print $3}' | sort -u) \
      <(nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TtDBRWVGS]$/ {print $3}' | sort -u))"
    if [[ -n $missing ]]; then
      fail "symbols missing from $R_SO: $(echo "$missing" | tr '\n' ' ')"
    else
      echo "  $R_SO exports every C symbol"
    fi
  done
done

# --- 5. Harness build matrix ----------------------------------------------
# The internals suite compares the file-local C functions against their Rust
# counterparts. Sweep C optimization levels and Rust profiles, since UB and
# floating-point codegen can be optimization-sensitive.
for copt in -O0 -O1 -O2 -O3 -Os; do
  for rprof in release dev; do
    step "internals matrix: C $copt / Rust $rprof"
    if HARNESS_C_OPT="$copt" HARNESS_RUST_PROFILE="$rprof" \
       timeout $TIMEOUT cargo test --release --test internals \
       > "$LOGDIR/internals$copt-$rprof.log" 2>&1; then
      grep -E '^test result:' "$LOGDIR/internals$copt-$rprof.log" | sed 's/^/  /'
    else
      fail "internals matrix C $copt / Rust $rprof"
      grep -E 'panicked|assertion|FAILED' "$LOGDIR/internals$copt-$rprof.log" | head -20
    fi
  done
done

step "RESULT"
if (( rc == 0 )); then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT (logs in $LOGDIR)"
fi
exit $rc
