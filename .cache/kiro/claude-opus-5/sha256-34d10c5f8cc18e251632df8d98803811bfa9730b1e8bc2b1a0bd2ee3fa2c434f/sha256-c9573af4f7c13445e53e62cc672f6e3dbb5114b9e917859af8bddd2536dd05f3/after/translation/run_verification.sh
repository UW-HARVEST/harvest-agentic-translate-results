#!/usr/bin/env bash
# Phase D driver: builds both libraries and runs the full differential suite
# under every feature combination and both cargo profiles.
#
# `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact, so the
# explicit `cargo build` before each `cargo test` is load-bearing: without it
# the tests would dlopen a stale .so and pass vacuously.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=== building the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml (there are none declared, so
# this yields the single default configuration; the loop is generic anyway).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== no [features] declared in Cargo.toml -> single default configuration ==="
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")
else
  n=${#FEATURES[@]}
  total=$((1 << n))
  COMBOS+=("DEFAULT")
  for ((mask = 0; mask < total; mask++)); do
    set=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then set="$set,${FEATURES[b]}"; fi
    done
    COMBOS+=("NODEFAULT${set}")
  done
fi

run_case() {
  local label="$1"; shift
  local profile="$1"; shift
  local -a flags=("$@")
  echo
  echo "############################################################"
  echo "# $label   [profile: ${profile:-dev}]"
  echo "############################################################"
  if ! timeout 600 cargo build "${flags[@]}" ${profile:+--release} 2>&1 | tail -3; then
    echo ">>> BUILD FAILED: $label/${profile:-dev}"; FAIL=1; return
  fi
  local log
  log=$(mktemp)
  timeout 600 cargo test "${flags[@]}" ${profile:+--release} >"$log" 2>&1
  local rc=$?
  grep -E "^ *Running tests/|^test result:|^test .* FAILED|^error" "$log"
  local total pass fail
  total=$(grep -c "^test .* \.\.\. ok$" "$log")
  fail=$(grep -oE "[0-9]+ failed" "$log" | awk '{s+=$1} END{print s+0}')
  echo "-- summary: $total tests ok, $fail failed (cargo rc=$rc)"
  if [ "$rc" -ne 0 ] || [ "$fail" -ne 0 ]; then
    echo ">>> TESTS FAILED: $label/${profile:-dev}"
    sed -n '/^failures:/,$p' "$log" | head -60
    FAIL=1
  fi
  rm -f "$log"
}

for combo in "${COMBOS[@]}"; do
  flags=()
  label="$combo"
  case "$combo" in
    DEFAULT) ;;
    NODEFAULT) flags=(--no-default-features); label="--no-default-features" ;;
    NODEFAULT,*) feats="${combo#NODEFAULT,}"
                 flags=(--no-default-features --features "$feats")
                 label="--no-default-features --features $feats" ;;
  esac
  run_case "$label" "release" "${flags[@]}"
  run_case "$label" ""        "${flags[@]}"
done

echo
echo "=== symbol diff (must be empty) ==="
for profile in release debug; do
  RS="$ROOT/translation/target/$profile/libarrayfunc_lib.so"
  [ -f "$RS" ] || continue
  diff <(nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort) \
       <(nm -D --defined-only "$RS"   | awk '$2=="T"{print $3}' | sort) \
    && echo "  $profile: OK (0 differences)" \
    || { echo "  $profile: SYMBOL DIFF NON-EMPTY"; FAIL=1; }
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$FAIL"
