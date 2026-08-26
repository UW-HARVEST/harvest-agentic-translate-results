#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination and runs the full
# verification (cargo check -> cargo build -> Phase B/C/D differential tests)
# for each one, plus an extra pass against the optimised (release) .so.
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS=(--offline)

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml (power set of [features]).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/,""); if ($0 != "default") print
    }
  ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  # No [features] section at all -> exactly one configuration.
  COMBOS=("")
else
  for (( m=0; m < (1<<n); m++ )); do
    combo=""
    for (( b=0; b<n; b++ )); do
      if (( m & (1<<b) )); then combo="${combo:+$combo,}${FEATURES[$b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "=========================================================="
echo "Cargo features found : ${FEATURES[*]:-<none>}"
echo "Combinations to test : ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<no features>}'"; done
echo "=========================================================="
echo

# ---------------------------------------------------------------------------
# 2. Build the C reference shared library (single CMake configuration: the
#    CMakeLists has no option()/target_compile_definitions, so there is only
#    one C build configuration).
# ---------------------------------------------------------------------------
if [[ ! -f c_src/build/libtranslated_rust.so ]]; then
  echo "### building C reference .so"
  ( mkdir -p c_src/build && cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 2; }
fi
ls -l c_src/build/libtranslated_rust.so
echo

rc=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  FEAT_ARGS=(--no-default-features)
  [[ -n "$combo" ]] && FEAT_ARGS+=(--features "$combo")

  echo "##########################################################"
  echo "### FEATURES: $label"
  echo "##########################################################"

  echo "--- cargo check ${FEAT_ARGS[*]}"
  timeout 600 cargo check "${CARGO_FLAGS[@]}" "${FEAT_ARGS[@]}" --all-targets \
    || { echo "CHECK FAILED for $label"; rc=1; continue; }

  echo "--- cargo build (debug cdylib) ${FEAT_ARGS[*]}"
  timeout 600 cargo build "${CARGO_FLAGS[@]}" "${FEAT_ARGS[@]}" \
    || { echo "BUILD FAILED for $label"; rc=1; continue; }

  echo "--- symbol parity (debug)"
  RUST_SO_PATH=target/debug/libgotomach_lib.so ./symbol_parity.sh \
    || { echo "SYMBOL PARITY FAILED for $label"; rc=1; }

  echo "--- Phase B/C/D differential tests (debug .so)"
  RUST_SO_PATH=target/debug/libgotomach_lib.so \
    timeout 600 cargo test "${CARGO_FLAGS[@]}" "${FEAT_ARGS[@]}" -- --test-threads=1 \
    || { echo "TESTS FAILED (debug) for $label"; rc=1; }

  echo "--- cargo build --release (optimised cdylib) ${FEAT_ARGS[*]}"
  timeout 600 cargo build --release "${CARGO_FLAGS[@]}" "${FEAT_ARGS[@]}" \
    || { echo "RELEASE BUILD FAILED for $label"; rc=1; continue; }

  echo "--- symbol parity (release)"
  RUST_SO_PATH=target/release/libgotomach_lib.so ./symbol_parity.sh \
    || { echo "SYMBOL PARITY FAILED (release) for $label"; rc=1; }

  echo "--- Phase B/C/D differential tests against the RELEASE .so"
  RUST_SO_PATH=$PWD/target/release/libgotomach_lib.so \
    timeout 600 cargo test "${CARGO_FLAGS[@]}" "${FEAT_ARGS[@]}" -- --test-threads=1 \
    || { echo "TESTS FAILED (release .so) for $label"; rc=1; }

  echo
done

echo "=========================================================="
if (( rc == 0 )); then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES DETECTED"; fi
echo "=========================================================="
exit $rc
