#!/usr/bin/env bash
# Runs the full differential suite (Phases B, C, D) across every Cargo feature
# combination and both optimisation profiles.
#
# Feature combinations are extracted from Cargo.toml, never hard-coded, so this
# keeps working if a [features] table is added later.
#
# Usage:  ./run_all_features.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"

# --------------------------------------------------------------------------
# 1. Build the C reference library.
# --------------------------------------------------------------------------
echo "=== building C reference library ==="
mkdir -p "$ROOT/c_src/build" || exit 1
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
[ -f "$C_SO" ] || { echo "missing $C_SO"; exit 1; }

# --------------------------------------------------------------------------
# 2. Enumerate the feature power set from Cargo.toml.
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { in_f = 1; next }
    /^\[/                { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
COMBOS+=("--no-default-features")   # empty feature set
COMBOS+=("")                       # default feature set

n=${#FEATURES[@]}
if (( n > 0 )); then
  echo "declared features: ${FEATURES[*]}"
  # Full power set of the non-default features.
  for (( mask = 1; mask < (1 << n); mask++ )); do
    sel=()
    for (( i = 0; i < n; i++ )); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[$i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--no-default-features --features $joined")
    COMBOS+=("--features $joined")
  done
else
  echo "declared features: (none) -- Cargo.toml has no [features] table,"
  echo "so the default and empty feature sets are the only configurations."
fi

# --------------------------------------------------------------------------
# 3. Run the suite for every combination x profile.
# --------------------------------------------------------------------------
FAILED=0
PASSES=()
for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo test ${profile_flag} ${combo:-<default features>}"
    echo
    echo "=== $label ==="
    # shellcheck disable=SC2086
    if timeout 600 cargo test $profile_flag $combo 2>&1 | tail -n 25; then
      PASSES+=("PASS  $label")
    else
      PASSES+=("FAIL  $label")
      FAILED=1
    fi
  done
done

# --------------------------------------------------------------------------
# 4. Symbol diff, printed explicitly (must be empty).
# --------------------------------------------------------------------------
echo
echo "=== symbol diff (C .so vs Rust .so) — must be empty ==="
for prof in debug release; do
  RUST_SO="target/$prof/libdriver.so"
  [ -f "$RUST_SO" ] || continue
  echo "--- $prof ---"
  diff <(nm -D --defined-only --format=posix "$C_SO"    | awk '$2=="T"{print $1}' | sort) \
       <(nm -D --defined-only --format=posix "$RUST_SO" | awk '$2=="T"{print $1}' | sort) \
       | grep '^<' && { echo "MISSING SYMBOLS IN RUST ($prof)"; FAILED=1; } \
       || echo "no C symbol missing from the Rust .so"
done

echo
echo "=== summary ==="
printf '%s\n' "${PASSES[@]}"
if (( FAILED )); then
  echo "OVERALL: FAILED"
  exit 1
fi
echo "OVERALL: ALL CONFIGURATIONS PASS"
