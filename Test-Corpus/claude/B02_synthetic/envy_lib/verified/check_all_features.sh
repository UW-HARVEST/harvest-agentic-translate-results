#!/usr/bin/env bash
# Phase A/D automation: enumerate every valid feature combination from
# Cargo.toml, `cargo check` each one, then build the cdylib and run the whole
# differential suite (phases B, C and D) against it - in the dev profile *and*
# in the release profile (panic = "abort", optimisations on).
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"

# --- enumerate the declared features (excluding "default") ------------------
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /=/   {split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "default" && a[1] != "") print a[1]}
' Cargo.toml)

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- powerset of the feature list -------------------------------------------
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "feature combinations to verify: ${#COMBOS[@]}"

rc=0
run() {
  echo "+ $*"
  if ! timeout 600 "$@"; then
    echo "FAILED: $*"
    rc=1
  fi
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    FEAT_ARGS=(--no-default-features)
    label="<no features>"
  else
    FEAT_ARGS=(--no-default-features --features "$combo")
    label="$combo"
  fi
  echo
  echo "================ feature combination: $label ================"

  run cargo check $CARGO_FLAGS "${FEAT_ARGS[@]}"
  run cargo check $CARGO_FLAGS "${FEAT_ARGS[@]}" --tests

  # dev profile
  run cargo build $CARGO_FLAGS "${FEAT_ARGS[@]}"
  ENVY_RUST_SO="$PWD/target/debug/libenvy_lib.so" \
    run cargo test $CARGO_FLAGS "${FEAT_ARGS[@]}"

  # release profile (panic = abort, optimised)
  run cargo build $CARGO_FLAGS --release "${FEAT_ARGS[@]}"
  echo "+ ENVY_RUST_SO=target/release/libenvy_lib.so cargo test ${FEAT_ARGS[*]}"
  if ! ENVY_RUST_SO="$PWD/target/release/libenvy_lib.so" \
      timeout 600 cargo test $CARGO_FLAGS "${FEAT_ARGS[@]}"; then
    echo "FAILED: release-profile cdylib for $label"
    rc=1
  fi
done

# the default configuration as a plain user would build it
echo
echo "================ default configuration ================"
run cargo build $CARGO_FLAGS
run cargo test $CARGO_FLAGS

echo
if [ "$rc" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASS"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$rc"
