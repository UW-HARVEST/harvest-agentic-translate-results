#!/usr/bin/env bash
# Phase D automation: enumerate every feature combination declared in Cargo.toml
# and run `cargo check` + the full differential test suite for each one.
#
# Usage: scripts/check_all_features.sh
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE_DIR="$PWD"
ROOT="$(dirname "$CRATE_DIR")"

FAIL=0
# Scratch dir: honour TMPDIR (a bare /tmp may be read-only).
TMPD="${TMPDIR:-/tmp}/encode_quant_featurecheck.$$"
mkdir -p "$TMPD" || { echo "cannot create scratch dir $TMPD"; exit 1; }
trap 'rm -rf "$TMPD"' EXIT
note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 0. Make sure the C ground-truth .so exists.
# ---------------------------------------------------------------------------
note "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 2 -name '*.so' | head -1)
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 1. Extract the feature list from Cargo.toml ([features] section keys).
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ { inf = 1; next }
  /^\[/           { inf = 0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); print $0
  }
' Cargo.toml | grep -v '^default$' | sort -u)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares NO [features]; the only configurations are the"
  echo "default build and --no-default-features (which are identical here)."
  COMBOS=("" "--no-default-features")
else
  echo "Declared features: $(echo "$FEATURES" | tr '\n' ' ')"
  # Full power set of the declared features.
  mapfile -t FARR <<<"$FEATURES"
  N=${#FARR[@]}
  COMBOS=("" "--no-default-features")
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FARR[$i]}"
      fi
    done
    COMBOS+=("--no-default-features --features $combo")
    COMBOS+=("--features $combo")
  done
fi

# ---------------------------------------------------------------------------
# 2. cargo check + full test run for every combination, debug AND release.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"

  note "cargo check $label"
  # shellcheck disable=SC2086
  if ! timeout 300 cargo check --all-targets $combo 2>&1 | tail -5; then
    echo "CHECK FAILED: $label"; FAIL=1
  fi

  for profile in "" "--release"; do
    plabel="${profile:-debug}"
    note "cargo test $label $plabel"
    log="$TMPD/test${plabel}$(echo "$label" | tr -c 'A-Za-z0-9' '_').log"
    # Run ONCE, capture everything, then judge from the captured output.
    # shellcheck disable=SC2086
    timeout 590 cargo test $profile $combo >"$log" 2>&1
    rc=$?
    # Print every "test result:" line so nothing is hidden by truncation.
    grep -E "^test result:" "$log" | sed 's/^/    /'
    nsuites=$(grep -c "^test result:" "$log")
    npassed=$(grep -c "^test result: ok" "$log")
    ntests=$(awk '/^test result: ok/ { s += $4 } END { print s + 0 }' "$log")
    echo "    -> exit=$rc suites=$nsuites ok=$npassed tests_passed=$ntests"
    if [ "$rc" -ne 0 ] || grep -q "^test result: FAILED" "$log" \
       || [ "$nsuites" -eq 0 ] || [ "$nsuites" -ne "$npassed" ]; then
      echo "TEST FAILED: $label $plabel (see $log)"
      grep -E "panicked|DIVERGENCE|^error" "$log" | head -20
      FAIL=1
    fi
    # Guard against a vacuous pass: the suite must actually run many tests.
    if [ "$ntests" -lt 60 ]; then
      echo "TEST SUITE TOO SMALL ($ntests passing tests): $label $plabel"
      FAIL=1
    fi
  done
done

# ---------------------------------------------------------------------------
# 3. Symbol parity re-check, independent of the test harness.
# ---------------------------------------------------------------------------
note "Symbol parity (nm -D)"
cargo build --release >/dev/null 2>&1
R_SO="$CRATE_DIR/target/release/libencode_quant_lib.so"
CS="$TMPD/c_syms.txt"; RS="$TMPD/r_syms.txt"
nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TWDBRi]$/ {print $3}' | sort > "$CS" \
  || { echo "nm on C .so FAILED"; FAIL=1; }
nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TWDBRi]$/ {print $3}' | sort > "$RS" \
  || { echo "nm on Rust .so FAILED"; FAIL=1; }
NC=$(wc -l < "$CS"); NR=$(wc -l < "$RS")
echo "C exports:    $NC"; cat "$CS" | sed 's/^/    C:    /'
echo "Rust exports: $NR"
# A zero-symbol C .so would make the diff vacuously empty - refuse that.
if [ "$NC" -lt 1 ]; then
  echo "REFUSING VACUOUS PASS: C .so exported $NC symbols"; FAIL=1
fi
if ! grep -qx "encode_quant" "$CS"; then
  echo "C .so is missing encode_quant"; FAIL=1
fi
if ! grep -qx "encode_quant" "$RS"; then
  echo "Rust .so is missing encode_quant"; FAIL=1
fi
MISSING=$(comm -23 "$CS" "$RS")
if [ -n "$MISSING" ]; then
  echo "MISSING FROM RUST:"; echo "$MISSING"; FAIL=1
else
  echo "Symbol diff: EMPTY ($NC/$NC C exports present in Rust)"
fi

note "RESULT"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$FAIL"
