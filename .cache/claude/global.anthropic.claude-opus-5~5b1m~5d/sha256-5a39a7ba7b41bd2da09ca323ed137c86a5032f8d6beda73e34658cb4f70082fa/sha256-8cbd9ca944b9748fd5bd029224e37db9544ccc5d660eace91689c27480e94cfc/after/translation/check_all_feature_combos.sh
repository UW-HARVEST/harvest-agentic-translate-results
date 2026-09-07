#!/usr/bin/env bash
# Phase D automation: enumerate every feature combination declared in
# Cargo.toml and run `cargo check` + the full differential test suite for each.
#
# Usage: ./check_all_feature_combos.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- extract feature names from the [features] section of Cargo.toml ---------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[a-zA-Z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "== declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the C reference library ------------------------------------------
echo "== building C reference library"
( mkdir -p ../c_src/build \
  && cd ../c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "FAIL: C build"; exit 1; }
C_SO=$(find ../c_src/build -maxdepth 1 -name '*.so' | head -1)
echo "   C  .so: $C_SO"

# --- enumerate the power set of features ------------------------------------
COMBOS=()
n=${#FEATURES[@]}
total=$((1 << n))
for ((mask = 0; mask < total; mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (( (mask >> i) & 1 )); then
      combo="${combo:+$combo,}${FEATURES[i]}"
    fi
  done
  COMBOS+=("$combo")
done
# Always include the plain default build as well.
COMBOS+=("__default__")

rc=0
for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "__default__" ]]; then
    label="(default features)"
    args=()
  elif [[ -z "$combo" ]]; then
    label="(no-default-features, no features)"
    args=(--no-default-features)
  else
    label="(no-default-features, features=$combo)"
    args=(--no-default-features --features "$combo")
  fi

  echo
  echo "=================================================================="
  echo "== combo $label"
  echo "=================================================================="

  echo "-- cargo check"
  if ! timeout 600 cargo check --offline "${args[@]}" 2>&1 | tail -5; then
    echo "FAIL: cargo check $label"; rc=1; continue
  fi

  echo "-- cargo build --release --lib (produce the cdylib under test)"
  if ! timeout 600 cargo build --offline --release --lib "${args[@]}" 2>&1 | tail -5; then
    echo "FAIL: cargo build $label"; rc=1; continue
  fi

  RS_SO=target/release/libfloat2half_lib.so
  echo "-- symbol diff (C exports missing from Rust)"
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u))
  if [[ -n "$missing" ]]; then
    echo "FAIL: symbols missing from Rust .so under $label:"; echo "$missing"; rc=1
  else
    echo "   OK: symbol diff empty"
  fi

  echo "-- cargo test --release"
  if ! timeout 600 cargo test --offline --release "${args[@]}" 2>&1 | tail -25; then
    echo "FAIL: cargo test $label"; rc=1; continue
  fi
done

echo
if [[ $rc -eq 0 ]]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} combos)"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $rc
