#!/usr/bin/env bash
# Runs the full differential test suite for EVERY cargo feature combination and
# for both the dev and release profiles.
#
# Feature names are extracted from Cargo.toml (the [features] table) rather than
# hard-coded, so a future feature is picked up automatically.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CARGO_FLAGS=(--offline)   # crates.io is unreachable in this sandbox

# --- build the C reference library ------------------------------------------
( mkdir -p ../c_src/build && cd ../c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# --- enumerate features -----------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in=1; next }
    /^\[/           { in=0 }
    in && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the only configurations are the"
  echo "default build and --no-default-features (equivalent here)."
  COMBOS+=("")                       # default
  COMBOS+=("--no-default-features")  # explicit empty feature set
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo+="${FEATURES[$i]},"; fi
    done
    COMBOS+=("--no-default-features --features ${combo%,}")
  done
  COMBOS+=("")  # plus the default feature set
fi

FAIL=0
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="profile='${profile:-dev}' features='${combo:-<default>}'"
    echo "=============================================================="
    echo ">>> cargo check   ${label}"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo check "${CARGO_FLAGS[@]}" $profile $combo --all-targets \
         >/dev/null 2>&1; then
      echo "!!! cargo check FAILED for ${label}"; FAIL=1; continue
    fi
    echo ">>> cargo test    ${label}"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test "${CARGO_FLAGS[@]}" $profile $combo 2>&1 \
         | grep -E '^(test |result:|error)' ; then
      : # grep found nothing interesting
    fi
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test "${CARGO_FLAGS[@]}" $profile $combo >/dev/null 2>&1; then
      echo "!!! cargo test FAILED for ${label}"; FAIL=1
    else
      echo "--- OK: ${label}"
    fi
  done
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
