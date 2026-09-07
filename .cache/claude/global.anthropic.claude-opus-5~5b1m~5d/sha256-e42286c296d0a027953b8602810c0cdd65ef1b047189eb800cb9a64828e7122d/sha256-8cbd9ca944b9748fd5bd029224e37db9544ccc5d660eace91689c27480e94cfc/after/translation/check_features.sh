#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination
# and under both cargo profiles.
#
# Feature combinations are extracted mechanically from Cargo.toml rather than
# hard-coded, so adding a feature automatically widens the matrix.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

CARGO_FLAGS=(--offline)
FAIL=0

# ---------------------------------------------------------------------------
# Extract the feature names from the [features] table of Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

echo "== declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# ---------------------------------------------------------------------------
# Build the combination list: default, no-default-features, then the power set
# of the declared features (capped so the matrix stays tractable).
# ---------------------------------------------------------------------------
COMBOS=()
COMBOS+=("__default__")
COMBOS+=("__nodefault__")

n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
elif (( n > 12 )); then
  echo "!! more than 12 features; testing each feature individually + all"
  for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
  COMBOS+=("$(IFS=,; echo "${FEATURES[*]}")")
fi

# ---------------------------------------------------------------------------
# Make sure the C .so exists before any timing-sensitive test runs.
# ---------------------------------------------------------------------------
if ! ls ../c_src/build/lib*.so >/dev/null 2>&1; then
  echo "== building the C shared library"
  ( mkdir -p ../c_src/build && cd ../c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "!! C build failed"; exit 1; }
fi

run_combo() {
  local profile="$1" combo="$2"
  local -a feat_args=()
  local label="$combo"
  case "$combo" in
    __default__)   label="(default features)" ;;
    __nodefault__) feat_args=(--no-default-features); label="(--no-default-features)" ;;
    *)             feat_args=(--no-default-features --features "$combo") ;;
  esac

  local -a prof_args=()
  [[ "$profile" == "release" ]] && prof_args=(--release)

  # Tell the test harness which feature set to use if it has to rebuild the
  # cdylib itself (see tests/common/mod.rs::rust_so_path).
  case "$combo" in
    __default__)   export HARNESS_CARGO_FEATURES="" ;;
    __nodefault__) export HARNESS_CARGO_FEATURES="__nodefault__" ;;
    *)             export HARNESS_CARGO_FEATURES="$combo" ;;
  esac

  echo
  echo "############################################################"
  echo "## profile=$profile features=$label"
  echo "############################################################"

  # `cargo test` needs the cdylib itself present for libloading; build it first
  # under the same profile/feature set.
  if ! cargo build "${CARGO_FLAGS[@]}" "${prof_args[@]}" "${feat_args[@]}"; then
    echo "!! cargo build FAILED (profile=$profile features=$label)"
    FAIL=1
    return
  fi
  if ! cargo test "${CARGO_FLAGS[@]}" "${prof_args[@]}" "${feat_args[@]}"; then
    echo "!! cargo test FAILED (profile=$profile features=$label)"
    FAIL=1
    return
  fi
  echo "== OK profile=$profile features=$label"
}

for profile in debug release; do
  for combo in "${COMBOS[@]}"; do
    run_combo "$profile" "$combo"
  done
done

echo
if (( FAIL )); then
  echo "RESULT: FAILURES PRESENT"
  exit 1
fi
echo "RESULT: ALL FEATURE COMBINATIONS x PROFILES PASSED"
