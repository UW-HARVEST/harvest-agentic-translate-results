#!/usr/bin/env bash
# Phase D driver: run the full differential suite under every feature
# combination and both build profiles.
#
# The Rust cdylib is rebuilt for each configuration before the tests run,
# because the tests dlopen the .so for the profile they were built in — debug
# (opt-level 0) and release (opt-level 3) are genuinely different codegen and
# both must match the C library bit-for-bit.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- ensure the C ground-truth library exists ------------------------------
if ! ls ../c_src/build/lib*.so >/dev/null 2>&1; then
  echo "== building C reference library =="
  ( cd ../c_src && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
echo "C .so: $(ls ../c_src/build/lib*.so)"

# --- enumerate feature combinations from Cargo.toml ------------------------
# Collect the feature names declared in the [features] table (excluding
# "default"), then build the powerset.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features] -> the only configuration is the default one."
  COMBOS+=("DEFAULT")
  COMBOS+=("NONE")   # --no-default-features (identical here, verified explicitly)
else
  echo "features found: ${FEATURES[*]}"
  COMBOS+=("DEFAULT")
  COMBOS+=("NONE")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

# --- run every (combo x profile) ------------------------------------------
FAILED=0
declare -a RESULTS

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT) FEATFLAGS=() ; label="default-features" ;;
    NONE)    FEATFLAGS=(--no-default-features) ; label="--no-default-features" ;;
    *)       FEATFLAGS=(--no-default-features --features "$combo") ; label="features=$combo" ;;
  esac

  for profile in debug release; do
    RELFLAG=()
    [ "$profile" = release ] && RELFLAG=(--release)

    echo
    echo "=================================================================="
    echo "== $label | profile=$profile"
    echo "=================================================================="

    if ! timeout 600 cargo build "${RELFLAG[@]}" "${FEATFLAGS[@]}" >/dev/null 2>&1; then
      echo "BUILD FAILED"
      RESULTS+=("FAIL(build) $label profile=$profile")
      FAILED=1
      continue
    fi

    LOG="${TMPDIR:-.}/verify_all.$$.log"
    timeout 600 cargo test "${RELFLAG[@]}" "${FEATFLAGS[@]}" -- --test-threads=4 >"$LOG" 2>&1
    rc=$?
    grep -E '^test result:|FAILED|panicked' "$LOG"

    if [ ! -s "$LOG" ]; then
      echo "NO TEST OUTPUT CAPTURED (harness problem)"
      RESULTS+=("FAIL(no-output) $label profile=$profile")
      FAILED=1
    elif [ "$rc" -ne 0 ] || grep -qE '^test result: FAILED|panicked at' "$LOG"; then
      RESULTS+=("FAIL(test) $label profile=$profile")
      FAILED=1
    else
      passed=$(grep -oE '^test result: ok\. [0-9]+' "$LOG" | awk '{s+=$4} END{print s+0}')
      if [ "$passed" -eq 0 ]; then
        echo "ZERO TESTS RAN (harness problem)"
        RESULTS+=("FAIL(zero-tests) $label profile=$profile")
        FAILED=1
      else
        RESULTS+=("ok   $label profile=$profile ($passed tests passed)")
      fi
    fi
    rm -f "$LOG"
  done
done

echo
echo "================== PHASE D SUMMARY =================="
for line in "${RESULTS[@]}"; do echo "  $line"; done
if [ "$FAILED" -ne 0 ]; then
  echo "RESULT: FAILURES PRESENT"
  exit 1
fi
echo "RESULT: all configurations pass"
