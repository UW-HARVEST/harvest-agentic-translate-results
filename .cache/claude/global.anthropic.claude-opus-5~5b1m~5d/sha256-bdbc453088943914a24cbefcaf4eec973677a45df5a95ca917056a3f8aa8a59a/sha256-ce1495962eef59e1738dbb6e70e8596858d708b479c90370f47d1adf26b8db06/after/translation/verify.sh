#!/usr/bin/env bash
# Full differential verification: builds the C .so, then runs every phase of
# the test suite against every feature combination and both Rust build
# profiles.
#
# Usage: ./verify.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE_DIR="$PWD"
WORK_DIR="$(cd .. && pwd)"

pass=0
fail=0
FAILED_RUNS=()

note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; pass=$((pass + 1)); }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; fail=$((fail + 1)); FAILED_RUNS+=("$*"); }

# ---------------------------------------------------------------------------
# 1. Build the C shared library (ground truth)
# ---------------------------------------------------------------------------
note "Building the C shared library"
(
  cd "$WORK_DIR/c_src" \
    && mkdir -p build \
    && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
C_SO="$WORK_DIR/c_src/build/libdriver.so"
[ -f "$C_SO" ] || { echo "missing $C_SO"; exit 1; }
echo "   $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
# Extract the [features] section keys, if any.
FEATURES=$(awk '
  /^\[features\]/ { inf = 1; next }
  /^\[/           { inf = 0 }
  inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "default") print a[1] }
' Cargo.toml)

# Build the list of cargo feature flag sets to test.
declare -a COMBOS
COMBOS=("" "--no-default-features")
if [ -n "$FEATURES" ]; then
  # Power set of the declared features, each with --no-default-features.
  feats=($FEATURES)
  n=${#feats[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then sel="$sel,${feats[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
  done
  COMBOS+=("--all-features")
fi

note "Feature combinations to verify (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do echo "   cargo test ${c:-<default>}"; done
if [ -z "$FEATURES" ]; then
  echo "   (Cargo.toml declares no [features]; default == --no-default-features)"
fi

# ---------------------------------------------------------------------------
# 3. cargo check every combination first
# ---------------------------------------------------------------------------
note "cargo check --tests for every combination"
for c in "${COMBOS[@]}"; do
  if timeout 600 cargo check --tests $c >/dev/null 2>&1; then
    ok "check ${c:-<default>}"
  else
    bad "check ${c:-<default>}"
  fi
done

# ---------------------------------------------------------------------------
# 4. Run the full suite: every combination x {debug, release} Rust artifact
# ---------------------------------------------------------------------------
timeout 600 cargo build          >/dev/null 2>&1 || { echo "debug build failed"; exit 1; }
timeout 600 cargo build --release >/dev/null 2>&1 || { echo "release build failed"; exit 1; }

for profile in debug release; do
  SO="$CRATE_DIR/target/$profile/libdriver.so"
  [ -f "$SO" ] || { bad "missing $SO"; continue; }
  for c in "${COMBOS[@]}"; do
    note "Suite: Rust=$profile  features=${c:-<default>}"
    if DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$SO" \
       timeout 600 cargo test $c -- --test-threads=1 2>&1 | tee "${TMPDIR:-/tmp}/.driver_verify.log" \
       | grep -E '^test result:|^running|^error'; then
      if grep -q "^test result: FAILED" "${TMPDIR:-/tmp}/.driver_verify.log" \
         || grep -qE '^error' "${TMPDIR:-/tmp}/.driver_verify.log"; then
        bad "suite $profile ${c:-<default>}"
      else
        ok "suite $profile ${c:-<default>}"
      fi
    else
      bad "suite $profile ${c:-<default>} (runner error)"
    fi
  done
done

# ---------------------------------------------------------------------------
# 5. Symbol parity diff, printed explicitly
# ---------------------------------------------------------------------------
note "Symbol parity (nm -D --defined-only)"
for profile in debug release; do
  SO="$CRATE_DIR/target/$profile/libdriver.so"
  d=$(diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
           <(nm -D --defined-only "$SO"   | awk '{print $3}' | sort) \
      | grep '^<')
  if [ -z "$d" ]; then
    ok "no C symbol missing from Rust ($profile)"
  else
    bad "Rust ($profile) is missing: $d"
  fi
done

# ---------------------------------------------------------------------------
note "Summary"
printf '   passed: %d\n   failed: %d\n' "$pass" "$fail"
if [ "$fail" -ne 0 ]; then
  printf '   failing runs:\n'
  for f in "${FAILED_RUNS[@]}"; do printf '     - %s\n' "$f"; done
  exit 1
fi
echo "   ALL CHECKS PASSED"
