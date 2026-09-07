#!/usr/bin/env bash
# Enumerate every feature combination declared in Cargo.toml and run the full
# differential suite under each one, in both debug and release profiles.
#
# Usage: translation/scripts/check_features.sh
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

# ---------------------------------------------------------------------------
# 1. Extract the feature names from the [features] table (excluding "default").
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { inf=1; next }
    /^\[/                { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# ---------------------------------------------------------------------------
# 2. Build the list of combinations: default, no-default, then the full
#    power set of the explicit features on top of --no-default-features.
# ---------------------------------------------------------------------------
COMBOS=()
COMBOS+=("")                            # default feature set
COMBOS+=("--no-default-features")       # empty feature set
COMBOS+=("--all-features")              # everything at once

n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo+="${FEATURES[i]},"; fi
    done
    COMBOS+=("--no-default-features --features ${combo%,}")
  done
fi

# De-duplicate (with 0 features, --all-features == --no-default-features == default).
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

echo "feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  * ${c:-<default>}"; done
echo

# ---------------------------------------------------------------------------
# 3. Make sure the C reference library is built.
# ---------------------------------------------------------------------------
if [[ ! -f ../c_src/build/libdriver.so ]]; then
  echo "building the C reference shared library..."
  ( mkdir -p ../c_src/build && cd ../c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

# ---------------------------------------------------------------------------
# 4. Run: cargo check, build the cdylib, then the full suite, per combo/profile.
# ---------------------------------------------------------------------------
fail=0
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    label="${combo:-<default>} ${profile:-debug}"
    echo "=============================================================="
    echo ">>> $label"
    echo "=============================================================="

    # shellcheck disable=SC2086
    if ! timeout 600 cargo check $combo $profile >/dev/null 2>&1; then
      echo "  cargo check FAILED for [$label]"; fail=1; continue
    fi

    # Build the cdylib first so the tests dlopen a .so matching this combo.
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $combo $profile >/dev/null 2>&1; then
      echo "  cargo build FAILED for [$label]"; fail=1; continue
    fi

    # Point the harness at exactly the .so we just built.
    if [[ -n "$profile" ]]; then
      export RUST_DRIVER_SO="$PWD/target/release/libdriver.so"
    else
      export RUST_DRIVER_SO="$PWD/target/debug/libdriver.so"
    fi
    export C_DRIVER_SO="$PWD/../c_src/build/libdriver.so"

    # shellcheck disable=SC2086
    if timeout 600 cargo test $combo $profile 2>&1 | tail -n 25; then
      echo "  PASS [$label]"
    else
      echo "  TESTS FAILED for [$label]"; fail=1
    fi
    echo
  done
done

echo "=============================================================="
if (( fail )); then
  echo "RESULT: at least one feature combination FAILED"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} feature combination(s) x 2 profiles PASSED"
