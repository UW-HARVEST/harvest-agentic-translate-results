#!/usr/bin/env bash
# Runs the C-vs-Rust differential suite for every build configuration.
#
# `cargo test` does NOT rebuild a cdylib-only lib target, so the `cargo build`
# before each `cargo test` is mandatory: without it the suite silently tests a
# stale libdriver.so. (tests/differential.rs also asserts freshness.)
#
# Usage: ./run_tests.sh [extra args passed to `cargo test`]
set -uo pipefail
cd "$(dirname "$0")"

# ---------------------------------------------------------------------------
# Feature combinations. Cargo.toml declares no [features] table, so the only
# valid configuration is the empty (default == no-default) one. This is derived
# mechanically rather than hard-coded, so it keeps working if features appear.
# ---------------------------------------------------------------------------
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [ -z "${FEATURES}" ]; then
  COMBOS=("")
else
  # power set of the declared features
  COMBOS=()
  feats=(${FEATURES})
  n=${#feats[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${feats[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "=== feature combinations to verify: ${#COMBOS[@]} ==="
for c in "${COMBOS[@]}"; do echo "  --no-default-features --features '${c}'"; done

# ---------------------------------------------------------------------------
# The C reference shared object.
# ---------------------------------------------------------------------------
if [ ! -f c_src/build/libdriver.so ]; then
  echo "=== building the C reference library ==="
  mkdir -p c_src/build
  (cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null) || exit 1
fi

rc=0
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    label="features='${combo}' profile=${profile}"
    flags=(--no-default-features)
    [ -n "${combo}" ] && flags+=(--features "${combo}")
    [ "${profile}" = release ] && flags+=(--release)

    echo
    echo "############################################################"
    echo "### cargo check   ${label}"
    echo "############################################################"
    timeout 600 cargo check "${flags[@]}" --all-targets 2>&1 | tail -5 || rc=1

    echo "### cargo build   ${label}   (refreshes the cdylib under test)"
    timeout 600 cargo build "${flags[@]}" 2>&1 | tail -3 || rc=1

    echo "### cargo test    ${label}"
    timeout 600 cargo test "${flags[@]}" --test differential -- --test-threads=1 "$@" 2>&1 | tail -25
    if [ ${PIPESTATUS[0]} -ne 0 ]; then
      echo "!!! FAILED: ${label}"
      rc=1
    fi
  done
done

echo
if [ ${rc} -eq 0 ]; then
  echo "=== ALL CONFIGURATIONS PASSED ==="
else
  echo "=== FAILURES PRESENT (rc=${rc}) ==="
fi
exit ${rc}
