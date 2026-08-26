#!/usr/bin/env bash
# Phase A + D driver: enumerate every valid feature combination from Cargo.toml
# and run `cargo check` and the full differential test suite for each one.
#
# Usage: ./verify_all.sh [--offline]
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT=$PWD
EXTRA=${1:-}
FAILED=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
say "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations (power set of [features], minus "default")
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z_][A-Za-z0-9_-]*[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
say "Found $N optional feature(s): ${FEATURES[*]:-<none>}"

COMBOS=()
if [ "$N" -eq 0 ]; then
  COMBOS=("")            # the empty set is the one and only configuration
else
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "Feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. cargo check + cargo test for every combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label=${combo:-"(no features)"}

  say "cargo check --no-default-features --features '$combo'  ->  $label"
  if ! timeout 600 cargo check $EXTRA --no-default-features --features "$combo" \
        --all-targets 2>&1 | tail -5; then
    echo "CHECK FAILED for $label"; FAILED=1; continue
  fi

  # The differential tests dlopen target/<profile>/libarrayfunc_lib.so, so the
  # cdylib for this combination has to exist before the tests run.
  say "cargo build --no-default-features --features '$combo'  ->  $label"
  if ! timeout 600 cargo build $EXTRA --no-default-features --features "$combo" 2>&1 | tail -3; then
    echo "BUILD FAILED for $label"; FAILED=1; continue
  fi

  say "cargo test --no-default-features --features '$combo'  ->  $label"
  if ! timeout 600 cargo test $EXTRA --no-default-features --features "$combo" 2>&1 \
        | grep -E "^(test result|error|failures:|test .* FAILED)"; then
    echo "TEST RUN produced no summary for $label"; FAILED=1; continue
  fi
  if ! timeout 600 cargo test $EXTRA --no-default-features --features "$combo" >/dev/null 2>&1; then
    echo "TESTS FAILED for $label"; FAILED=1
  fi
done

# ---------------------------------------------------------------------------
# 4. Symbol diff (must be empty)
# ---------------------------------------------------------------------------
say "Symbol diff: C .so vs Rust .so"
RUST_SO=$ROOT/target/debug/libarrayfunc_lib.so
if ! command -v nm >/dev/null; then
  echo "nm unavailable -- CANNOT VERIFY SYMBOL PARITY"; FAILED=1
elif [ ! -f "$RUST_SO" ]; then
  echo "Rust .so missing at $RUST_SO -- CANNOT VERIFY SYMBOL PARITY"; FAILED=1
else
  # No temp files (the sandbox may have a read-only /tmp): use process
  # substitution so a failure to produce either list cannot look like success.
  C_COUNT=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u | wc -l)
  R_COUNT=$(nm -D --defined-only "$RUST_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u | wc -l)
  if [ "$C_COUNT" -eq 0 ] || [ "$R_COUNT" -eq 0 ]; then
    echo "nm produced no symbols (C=$C_COUNT, Rust=$R_COUNT) -- CANNOT VERIFY"; FAILED=1
  else
    MISSING=$(comm -23 \
      <(nm -D --defined-only "$C_SO"    | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u) \
      | grep -vE '^(_ITM_|__cxa_|__gmon_|_init$|_fini$|__bss_start|_edata$|_end$)')
    echo "C exports: $C_COUNT   Rust exports: $R_COUNT"
    if [ -n "$MISSING" ]; then
      echo "MISSING FROM RUST .so:"; echo "$MISSING"; FAILED=1
    else
      echo "symbol diff empty (all $C_COUNT C exports present in the Rust .so)"
    fi
  fi
fi

say "RESULT"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit $FAILED
