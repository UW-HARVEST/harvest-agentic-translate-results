#!/usr/bin/env bash
# Phase D sweep: rebuild both libraries and run the full differential suite
# under every Cargo feature combination and both profiles.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# the sweep stays correct if features are ever added.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1   # translation/
ROOT="$(cd .. && pwd)"
TIMEOUT=600
rc=0

echo "=== Building the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l "$ROOT/c_src/build/libdriver.so"

# --- enumerate feature combinations -----------------------------------------
# Every name declared under [features], excluding the implicit "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== Cargo.toml declares no [features]; the crate has one configuration ==="
  COMBOS+=("default:")                     # plain build
  COMBOS+=("no-default-features:")         # must be identical, but verify it
else
  echo "=== Features found: ${FEATURES[*]} ==="
  COMBOS+=("default:")
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo+="${FEATURES[i]},"
      fi
    done
    COMBOS+=("no-default-features:${combo%,}")
  done
fi

# --- run the suite for each combination x profile ----------------------------
for profile in dev release; do
  for entry in "${COMBOS[@]}"; do
    kind="${entry%%:*}"
    feats="${entry#*:}"

    flags=()
    label="profile=$profile"
    [ "$profile" = release ] && flags+=("--release")
    if [ "$kind" = "no-default-features" ]; then
      flags+=("--no-default-features")
      label+=" --no-default-features"
      if [ -n "$feats" ]; then
        flags+=("--features" "$feats")
        label+=" --features $feats"
      fi
    else
      label+=" (default features)"
    fi

    echo
    echo "############################################################"
    echo "# $label"
    echo "############################################################"

    # Build the cdylib first: the tests dlopen it, so it must exist and be
    # fresh for this exact configuration.
    if ! timeout "$TIMEOUT" cargo build ${flags[@]+"${flags[@]}"} >/dev/null 2>&1; then
      echo "BUILD FAILED [$label]"; rc=1; continue
    fi
    if ! timeout "$TIMEOUT" cargo test ${flags[@]+"${flags[@]}"} 2>&1 \
         | tee /tmp/sweep_last.log | grep -E '^test result|^error|FAILED'; then
      :
    fi
    if grep -qE 'FAILED|^error' /tmp/sweep_last.log; then
      echo "TESTS FAILED [$label]"; rc=1
    fi
  done
done

echo
if [ "$rc" -eq 0 ]; then
  echo "=== SWEEP PASSED: every configuration matched the C library ==="
else
  echo "=== SWEEP FAILED ==="
fi
exit "$rc"
