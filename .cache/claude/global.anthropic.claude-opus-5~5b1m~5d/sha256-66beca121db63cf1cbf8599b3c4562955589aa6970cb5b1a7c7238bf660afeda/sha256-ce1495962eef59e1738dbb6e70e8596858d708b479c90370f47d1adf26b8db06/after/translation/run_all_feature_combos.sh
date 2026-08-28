#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY cargo feature
# combination declared in Cargo.toml.
#
# Usage: ./run_all_feature_combos.sh
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
CARGO_FLAGS="--offline"
FAIL=0

echo "=== [0] building the C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' "$HERE/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; combinations = {default} and {--no-default-features}"
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")
else
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("NODEFAULT:$combo")
    COMBOS+=("DEFAULT:$combo")
  done
fi

run_combo() {
  local spec="$1"
  local args=()
  case "$spec" in
    DEFAULT) label="(default features)" ;;
    NODEFAULT) label="--no-default-features"; args+=(--no-default-features) ;;
    NODEFAULT:*) label="--no-default-features --features ${spec#NODEFAULT:}"
                 args+=(--no-default-features --features "${spec#NODEFAULT:}") ;;
    DEFAULT:*)   label="--features ${spec#DEFAULT:}"
                 args+=(--features "${spec#DEFAULT:}") ;;
  esac

  echo
  echo "=================================================================="
  echo "=== feature combo: $label"
  echo "=================================================================="

  ( cd "$HERE" \
    && cargo build $CARGO_FLAGS "${args[@]}" >/dev/null 2>&1 \
    && cargo build $CARGO_FLAGS --release "${args[@]}" >/dev/null 2>&1 ) \
    || { echo "BUILD FAILED for $label"; FAIL=1; return; }

  echo "--- nm -D symbol parity ---"
  ( cd "$HERE" && ./check_symbols.sh ) || FAIL=1

  ( cd "$HERE" && cargo test $CARGO_FLAGS "${args[@]}" -- --test-threads=4 ) \
    || { echo "TESTS FAILED for $label"; FAIL=1; }
}

for spec in "${COMBOS[@]}"; do
  run_combo "$spec"
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} combos)"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$FAIL"
