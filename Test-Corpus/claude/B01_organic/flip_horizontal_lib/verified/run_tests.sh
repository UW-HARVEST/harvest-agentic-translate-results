#!/usr/bin/env bash
# Differential test driver.
#
# Enumerates every valid Cargo feature combination, and for each one:
#   1. cargo check
#   2. builds the cdylib in BOTH profiles (`cargo test` does not build a cdylib,
#      because the integration tests dlopen it instead of linking it)
#   3. runs the Phase B + Phase C differential test suites
#
# The C reference .so is built once up front.
set -uo pipefail
cd "$(dirname "$0")"

RED=$'\033[31m'; GRN=$'\033[32m'; YLW=$'\033[33m'; RST=$'\033[0m'
fail=0

# --------------------------------------------------------------------------
# Build the C reference shared object.
# --------------------------------------------------------------------------
echo "${YLW}== building C reference .so ==${RST}"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "${RED}C build FAILED${RST}"; exit 1; }
ls -l c_src/build/libtranslated_rust.so

# --------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); if ($0 != "default") print $0
    }
  ' Cargo.toml
)

declare -a COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table at all -> exactly one configuration.
  COMBOS=("")
else
  n=${#FEATURES[@]}
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( bit = 0; bit < n; bit++ )); do
      if (( mask & (1 << bit) )); then
        combo="${combo:+$combo,}${FEATURES[$bit]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo
echo "${YLW}== ${#COMBOS[@]} feature combination(s) to verify ==${RST}"
for c in "${COMBOS[@]}"; do echo "   --no-default-features --features '${c}'"; done

# --------------------------------------------------------------------------
# Verify each combination.
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none (no-default-features)>}"
  echo
  echo "${YLW}=================================================================${RST}"
  echo "${YLW}== FEATURES: ${label}${RST}"
  echo "${YLW}=================================================================${RST}"

  args=(--no-default-features)
  [ -n "$combo" ] && args+=(--features "$combo")

  echo "-- cargo check"
  if ! cargo check "${args[@]}" --all-targets 2>&1 | tail -n 5; then
    echo "${RED}CHECK FAILED: ${label}${RST}"; fail=1; continue
  fi

  # The tests dlopen the cdylib, so it must be built explicitly, in the same
  # profile the test binary runs in (debug) -- and in release, so the
  # optimised, panic=abort artifact is covered too.
  for profile in debug release; do
    echo "-- cargo build (${profile}) cdylib"
    pargs=("${args[@]}")
    [ "$profile" = release ] && pargs+=(--release)
    if ! cargo build "${pargs[@]}" 2>&1 | tail -n 3; then
      echo "${RED}BUILD FAILED: ${label} / ${profile}${RST}"; fail=1
    fi
  done

  for suite in differential error_paths; do
    echo "-- cargo test --test ${suite}"
    if ! timeout 600 cargo test "${args[@]}" --test "$suite" 2>&1 | tail -n 8; then
      echo "${RED}TEST FAILED: ${label} / ${suite}${RST}"; fail=1
    fi
  done

  # Re-run the suites against the RELEASE cdylib by pointing the loader at it.
  echo "-- re-running suites against the release cdylib"
  for suite in differential error_paths; do
    if ! timeout 600 env DIFFTEST_RUST_SO="$PWD/target/release/libflip_horizontal_lib.so" \
        cargo test "${args[@]}" --test "$suite" 2>&1 | tail -n 5; then
      echo "${RED}RELEASE TEST FAILED: ${label} / ${suite}${RST}"; fail=1
    fi
  done
done

# --------------------------------------------------------------------------
# Symbol parity.
# --------------------------------------------------------------------------
echo
echo "${YLW}== symbol parity (nm -D) ==${RST}"
./check_symbols.sh || fail=1

echo
if [ "$fail" -eq 0 ]; then
  echo "${GRN}ALL CONFIGURATIONS PASSED${RST}"
else
  echo "${RED}FAILURES PRESENT${RST}"
fi
exit "$fail"
