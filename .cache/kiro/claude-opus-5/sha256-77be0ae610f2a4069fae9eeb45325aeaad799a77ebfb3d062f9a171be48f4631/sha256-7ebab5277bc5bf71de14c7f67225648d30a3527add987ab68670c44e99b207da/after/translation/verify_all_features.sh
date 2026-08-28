#!/usr/bin/env bash
# Enumerate every valid feature combination from Cargo.toml and run
# `cargo check` + `cargo test` for each one.
#
# Usage: ./verify_all_features.sh [check|test|both]   (default: both)
set -uo pipefail

cd "$(dirname "$0")" || exit 1
MODE="${1:-both}"
LOGDIR="/tmp/xlat-verify"
mkdir -p "$LOGDIR"

# --- enumerate features ----------------------------------------------------
# Read the [features] table from Cargo.toml, ignoring the implicit "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=")
      gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
if (( N == 0 )); then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  COMBOS=("")
else
  echo "Declared features (${N}): ${FEATURES[*]}"
  COMBOS=()
  # Every subset of the feature set (2^N combinations).
  for (( mask = 0; mask < (1 << N); mask++ )); do
    combo=""
    for (( i = 0; i < N; i++ )); do
      if (( (mask >> i) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "Configurations to verify: ${#COMBOS[@]}"

# --- ensure the C reference library exists ---------------------------------
C_BUILD="../c_src/build"
if ! ls "$C_BUILD"/lib*.so >/dev/null 2>&1; then
  echo "Building the C reference shared library..."
  ( mkdir -p "$C_BUILD" && cd "$C_BUILD" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build . ) > "$LOGDIR/c-build.log" 2>&1 \
    || { echo "C build FAILED, see $LOGDIR/c-build.log"; exit 1; }
fi
echo "C reference: $(ls "$C_BUILD"/lib*.so)"

# --- run ------------------------------------------------------------------
fail=0
for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    label="<no features>"
    args=(--no-default-features)
    slug="none"
  else
    label="$combo"
    args=(--no-default-features --features "$combo")
    slug="${combo//,/_}"
  fi

  if [[ "$MODE" == "check" || "$MODE" == "both" ]]; then
    printf 'cargo check  [%s] ... ' "$label"
    if timeout 600 cargo check --all-targets "${args[@]}" \
        > "$LOGDIR/check-$slug.log" 2>&1; then
      echo "ok"
    else
      echo "FAILED (see $LOGDIR/check-$slug.log)"
      tail -n 25 "$LOGDIR/check-$slug.log"
      fail=1
      continue
    fi
  fi

  if [[ "$MODE" == "test" || "$MODE" == "both" ]]; then
    printf 'cargo test   [%s] ... ' "$label"
    if timeout 600 cargo test "${args[@]}" \
        > "$LOGDIR/test-$slug.log" 2>&1; then
      echo "ok ($(grep -c '^test .* ok$' "$LOGDIR/test-$slug.log") tests passed)"
    else
      echo "FAILED (see $LOGDIR/test-$slug.log)"
      grep -E '^(test |failures:|thread )' "$LOGDIR/test-$slug.log" | tail -n 40
      fail=1
    fi
  fi
done

if (( fail )); then
  echo "RESULT: at least one configuration failed."
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} configuration(s) passed."
