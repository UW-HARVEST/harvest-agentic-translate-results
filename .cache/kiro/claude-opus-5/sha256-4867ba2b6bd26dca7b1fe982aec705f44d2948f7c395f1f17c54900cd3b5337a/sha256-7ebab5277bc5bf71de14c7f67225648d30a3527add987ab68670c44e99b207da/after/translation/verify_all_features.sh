#!/usr/bin/env bash
# Enumerates every feature combination declared in Cargo.toml and runs
# `cargo check` + `cargo test` for each. `driver` currently declares no
# [features], so the only valid combination is the empty set, but the loop is
# driven off Cargo.toml so it keeps working if features are added later.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- ensure the C reference library exists -----------------------------------
C_BUILD=../c_src/build
if [[ ! -f $C_BUILD/libdriver.so ]]; then
  mkdir -p $C_BUILD
  (cd $C_BUILD && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null) || exit 1
fi

# --- enumerate features ------------------------------------------------------
# Feature names are the `name = [...]` keys in the [features] table, minus
# `default`, which is covered by the no-default-features baseline plus combos.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblk = 1; next }
    /^\[/           { inblk = 0 }
    inblk && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "Declared features (${N}): ${FEATURES[*]:-<none>}"

COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if ((mask & (1 << i))); then combo+="${combo:+,}${FEATURES[i]}"; fi
  done
  COMBOS+=("$combo")
done

echo "Feature combinations to verify: ${#COMBOS[@]}"

# --- check + test each combination -------------------------------------------
fail=0
for combo in "${COMBOS[@]}"; do
  label=${combo:-"(no features)"}
  echo "=============================================================="
  echo ">>> $label"

  if ! timeout 600 cargo check --no-default-features --features "$combo" 2>&1 | tail -20; then
    echo "!!! cargo check FAILED for $label"; fail=1; continue
  fi
  # Build the cdylib the tests dlopen, then run the comparison suite.
  if ! timeout 600 cargo build --no-default-features --features "$combo" 2>&1 | tail -20; then
    echo "!!! cargo build FAILED for $label"; fail=1; continue
  fi
  if ! timeout 600 cargo test --no-default-features --features "$combo" 2>&1 | grep -E "^test |test result|^error"; then
    echo "!!! cargo test FAILED for $label"; fail=1; continue
  fi
done

# Also cover the crate's own default feature set.
echo "=============================================================="
echo ">>> (default features)"
timeout 600 cargo build 2>&1 | tail -5
timeout 600 cargo test 2>&1 | grep -E "^test |test result|^error" || fail=1

echo "=============================================================="
if ((fail)); then echo "RESULT: FAILURES"; exit 1; else echo "RESULT: all combinations OK"; fi
