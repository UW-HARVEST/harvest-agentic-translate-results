#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so this
# stays correct if features are ever added. With no [features] table the
# combination set is {default, --no-default-features}, which is what this crate
# has today.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE_DIR=$PWD
ROOT=$(cd .. && pwd)

# ---------------------------------------------------------------------------
# Build the C reference library
# ---------------------------------------------------------------------------
if [[ ! -f "$ROOT/c_src/build/libStaticAlias.so" ]]; then
  mkdir -p "$ROOT/c_src/build" || exit 1
  ( cd "$ROOT/c_src/build" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build failed"; exit 1; }
fi
C_SO="$ROOT/c_src/build/libStaticAlias.so"
echo "C reference: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); gsub(/[[:space:]]/, "", $0)
      if ($0 != "default") print $0
    }
  ' Cargo.toml
)

COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 ]]; then
  echo "Cargo.toml declares no [features]; only the default configuration exists."
  COMBOS+=("")                        # default
  COMBOS+=("--no-default-features")   # identical here, verified explicitly
else
  echo "features: ${FEATURES[*]}"
  n=${#FEATURES[@]}
  for (( mask=0; mask < (1 << n); mask++ )); do
    sel=()
    for (( i=0; i < n; i++ )); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[$i]}")
    done
    if [[ ${#sel[@]} -eq 0 ]]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  COMBOS+=("")  # plus the plain default build
fi

# ---------------------------------------------------------------------------
# For each combination: check, build the cdylib, diff symbols, run the suite
# ---------------------------------------------------------------------------
FAIL=0
for combo in "${COMBOS[@]}"; do
  label=${combo:-"(default features)"}
  echo
  echo "==================================================================="
  echo "combination: $label"
  echo "==================================================================="

  # shellcheck disable=SC2086
  if ! timeout 600 cargo check $combo >/dev/null 2>&1; then
    echo "FAIL: cargo check $label"; FAIL=1; continue
  fi

  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release --lib $combo >/dev/null 2>&1; then
    echo "FAIL: cargo build --release --lib $label"; FAIL=1; continue
  fi
  RS_SO="$CRATE_DIR/target/release/libStaticAlias.so"

  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u))
  if [[ -n "$missing" ]]; then
    echo "FAIL: symbols exported by C but not Rust under $label:"; echo "$missing"; FAIL=1
  else
    echo "symbol parity: OK (0 missing)"
  fi

  # Differential suite against the debug cdylib the harness builds itself…
  # shellcheck disable=SC2086
  if STATICALIAS_CARGO_FEATURE_ARGS="$combo" \
       timeout 600 cargo test $combo --test differential -- --test-threads=1 \
       2>&1 | tail -3 | grep -q '^test result: ok'; then
    echo "differential (dev profile): OK"
  else
    echo "FAIL: differential suite (dev profile) under $label"; FAIL=1
  fi

  # …and against the release cdylib, which is the shipped artifact.
  # shellcheck disable=SC2086
  if STATICALIAS_SO="$RS_SO" \
       timeout 600 cargo test $combo --test differential -- --test-threads=1 \
       2>&1 | tail -3 | grep -q '^test result: ok'; then
    echo "differential (release .so): OK"
  else
    echo "FAIL: differential suite (release .so) under $label"; FAIL=1
  fi
done

echo
if [[ $FAIL -eq 0 ]]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit $FAIL
