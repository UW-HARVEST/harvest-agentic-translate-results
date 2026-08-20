#!/usr/bin/env bash
# Enumerate every [features] combination from Cargo.toml and run the whole
# verification suite under each one. This crate declares no [features], so the
# complete set is the single empty combination -- but the loop is written
# generically so that adding a feature automatically extends the sweep.
set -uo pipefail
cd "$(dirname "$0")"

# --- enumerate features -----------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[ \t]*=/ {sub(/[ \t]*=.*/,""); print}
  ' Cargo.toml
)
N=${#FEATURES[@]}
echo "Declared features (${N}): ${FEATURES[*]:-<none>}"

COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((b = 0; b < N; b++)); do
    if (( mask & (1 << b) )); then combo+="${FEATURES[b]},"; fi
  done
  COMBOS+=("${combo%,}")
done
echo "Feature combinations to verify: ${#COMBOS[@]}"

# --- build the C reference once --------------------------------------------
if [ ! -f c_src/build/libtranslated_rust.so ]; then
  echo "== building the C reference shared object =="
  ( mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || exit 1
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  label=${combo:-"(no features)"}
  echo
  echo "############################################################"
  echo "## feature combination: $label"
  echo "############################################################"

  if ! cargo check --offline --no-default-features --features "$combo" 2>&1 | tail -3; then
    echo "!! cargo check FAILED for $label"; FAIL=1; continue
  fi
  # The tests dlopen target/debug/libomni_manifold_lib.so, so build it first.
  if ! cargo build --offline --no-default-features --features "$combo" 2>&1 | tail -3; then
    echo "!! cargo build FAILED for $label"; FAIL=1; continue
  fi
  if ! timeout 900 cargo test --offline --no-default-features --features "$combo" \
        -- --test-threads=1 2>&1 | grep -E 'running|test result|^error|FAILED|panicked'; then
    echo "!! cargo test FAILED for $label"; FAIL=1
  fi
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} total)"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $FAIL
