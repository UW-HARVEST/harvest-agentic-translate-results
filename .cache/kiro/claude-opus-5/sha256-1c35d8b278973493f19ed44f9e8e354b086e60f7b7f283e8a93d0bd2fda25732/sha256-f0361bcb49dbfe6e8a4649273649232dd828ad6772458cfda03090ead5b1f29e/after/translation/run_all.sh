#!/usr/bin/env bash
# Full verification run: builds both libraries and executes the differential
# suite against every Rust cdylib profile and every feature combination.
#
#   cd translation && ./run_all.sh
#
# Every cargo invocation is wrapped in `timeout 600`.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
FAILED=0

hdr() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C reference shared library.
# ---------------------------------------------------------------------------
hdr "Building C reference library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
[[ -f "$C_SO" ]] || { echo "missing $C_SO"; exit 1; }

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml.
#
# The crate declares no [features] table, so the set is just the default build
# plus --no-default-features / --all-features (which are all the same build
# here). Extracted rather than hard-coded so this keeps working if features are
# added later.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); if ($0 != "default") print $0
    }
  ' "$HERE/Cargo.toml"
)
hdr "Feature combinations"
if (( ${#FEATURES[@]} == 0 )); then
  echo "Cargo.toml declares no [features]; only the default configuration exists."
  COMBOS=("" "--no-default-features" "--all-features")
else
  echo "declared features: ${FEATURES[*]}"
  COMBOS=("" "--no-default-features" "--all-features")
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  # Full powerset of the declared features.
  n=${#FEATURES[@]}
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && combo="${combo:+$combo,}${FEATURES[$i]}"
    done
    COMBOS+=("--no-default-features --features $combo")
  done
fi

# ---------------------------------------------------------------------------
# 3. cargo check every combination first (fast failure).
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  hdr "cargo check ${combo:-<default>}"
  # shellcheck disable=SC2086
  ( cd "$HERE" && timeout 600 cargo check --all-targets $combo ) \
    || { echo "CHECK FAILED: ${combo:-<default>}"; FAILED=1; }
done

# ---------------------------------------------------------------------------
# 4. Build the Rust cdylib in both profiles and run the suite against each.
#    DRIVER_RUST_SO pins which .so the harness dlopen()s, so both the debug and
#    the (shipping) release export wrapper get differentially tested.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    relflag=""; [[ $profile == release ]] && relflag="--release"

    hdr "build cdylib [$profile] ${combo:-<default>}"
    # shellcheck disable=SC2086
    ( cd "$HERE" && timeout 600 cargo build $relflag $combo ) \
      || { echo "BUILD FAILED: $profile ${combo:-<default>}"; FAILED=1; continue; }

    RUST_SO="$HERE/target/$profile/libdriver.so"
    [[ -f "$RUST_SO" ]] || { echo "missing $RUST_SO"; FAILED=1; continue; }

    hdr "symbol parity [$profile] ${combo:-<default>}"
    diff <(nm -D --defined-only --format=posix "$C_SO"    | awk '$2 ~ /^[TDBRWV]$/ {print $1}' | sort) \
         <(nm -D --defined-only --format=posix "$RUST_SO" | awk '$2 ~ /^[TDBRWV]$/ {print $1}' | sort) \
      && echo "symbol diff EMPTY (parity OK)" \
      || { echo "SYMBOL DIFF NON-EMPTY"; FAILED=1; }

    hdr "cargo test [$profile cdylib] ${combo:-<default>}"
    # Run the full suite (heavy sweeps included) once per profile for the
    # default feature set; the remaining combos compile identical code, so
    # DRIVER_FAST=1 skips the multi-minute sweeps there.
    fast=""
    if [[ -n "$combo" ]]; then fast="1"; fi
    # shellcheck disable=SC2086
    ( cd "$HERE" && DRIVER_RUST_SO="$RUST_SO" DRIVER_FAST="$fast" \
        timeout 600 cargo test $combo ) \
      || { echo "TEST FAILED: $profile ${combo:-<default>}"; FAILED=1; }
  done
done

hdr "RESULT"
if (( FAILED )); then
  echo "VERIFICATION FAILED"
  exit 1
fi
echo "ALL CONFIGURATIONS PASSED"
