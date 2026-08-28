#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and run cargo check + the
# differential test suite for each, in both the dev and release profiles.
#
# Usage: ./verify_all_features.sh        (run from translation/)
set -uo pipefail

cd "$(dirname "$0")"

LOG_DIR=/tmp/goto-verify
mkdir -p "$LOG_DIR"

# --- build the C ground truth ------------------------------------------------
C_DIR="$(cd .. && pwd)/c_src"
if [[ ! -f "$C_DIR/build/libdriver.so" ]]; then
  echo "== building the C shared library =="
  (mkdir -p "$C_DIR/build" && cd "$C_DIR/build" &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .) \
    >"$LOG_DIR/cmake.log" 2>&1 || { tail -30 "$LOG_DIR/cmake.log"; exit 1; }
fi

# --- enumerate feature combinations -----------------------------------------
# Every subset of the declared [features] (excluding the implicit "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if ((mask & (1 << i))); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
  done
  COMBOS+=("$combo")
done
# `cargo check` with the default feature set is also a distinct configuration.
COMBOS+=("__default__")

echo "== ${#FEATURES[@]} declared feature(s): ${FEATURES[*]:-<none>}"
echo "== ${#COMBOS[@]} configuration(s) to verify"

fail=0
for combo in "${COMBOS[@]}"; do
  if [[ $combo == "__default__" ]]; then
    label="default-features"
    flags=()
  elif [[ -z $combo ]]; then
    label="no-default-features"
    flags=(--no-default-features)
  else
    label="no-default-features+$combo"
    flags=(--no-default-features --features "$combo")
  fi

  for profile in dev release; do
    prof_flags=()
    [[ $profile == release ]] && prof_flags=(--release)
    tag="${label//[^A-Za-z0-9]/_}-$profile"

    for step in check test; do
      log="$LOG_DIR/$tag-$step.log"
      # The differential harness rebuilds the cdylib itself; make sure it uses
      # the same feature selection.
      if DRIVER_CARGO_FLAGS="${flags[*]}" \
        timeout 600 cargo "$step" "${flags[@]}" "${prof_flags[@]}" >"$log" 2>&1; then
        printf '  ok   %-45s %s\n' "$label/$profile" "$step"
      else
        printf '  FAIL %-45s %s  (see %s)\n' "$label/$profile" "$step" "$log"
        tail -40 "$log"
        fail=1
      fi
    done
  done
done

if ((fail)); then
  echo "== FAILURES present =="
  exit 1
fi
echo "== all configurations verified =="
