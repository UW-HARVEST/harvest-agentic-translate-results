#!/usr/bin/env bash
# Phase D driver: rebuild the C reference, then run the full differential suite
# under every feature combination x build profile.
set -uo pipefail
cd "$(dirname "$0")"

# ---- 1. C reference .so -----------------------------------------------------
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# ---- 2. Enumerate feature combinations from Cargo.toml ---------------------
# Read the [features] table (excluding "default"); build the power set.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0,a,"="); gsub(/[[:space:]]/,"",a[1]); if (a[1]!="default") print a[1] }' Cargo.toml
)
N=${#FEATURES[@]}
echo "declared non-default features: $N ${FEATURES[*]:-(none)}"

COMBOS=("default")                       # plain `cargo test`
COMBOS+=("__nodefault__")                # --no-default-features
if (( N > 0 )); then
  for (( mask=1; mask < (1<<N); mask++ )); do
    combo=""
    for (( i=0; i<N; i++ )); do
      (( mask & (1<<i) )) && combo="${combo:+$combo,}${FEATURES[$i]}"
    done
    COMBOS+=("$combo")
  done
fi

# ---- 3. Run the suite for each combination x profile ------------------------
FAIL=0
for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    flags=()
    [[ $profile == release ]] && flags+=(--release)
    case "$combo" in
      default)        label="default features" ;;
      __nodefault__)  label="--no-default-features"; flags+=(--no-default-features) ;;
      *)              label="features=$combo"; flags+=(--no-default-features --features "$combo") ;;
    esac

    echo
    echo "=============================================================="
    echo "  $label   [$profile profile]"
    echo "=============================================================="

    # Build the cdylib for this configuration and point the tests at it, so the
    # .so under test always matches the configuration being exercised.
    cargo build "${flags[@]}" >/dev/null 2>&1 || { echo "  BUILD FAILED"; FAIL=1; continue; }
    so="target/$profile/libdriver.so"
    [[ -f $so ]] || { echo "  missing $so"; FAIL=1; continue; }

    if DRIVER_RUST_SO="$PWD/$so" timeout 600 cargo test "${flags[@]}" 2>&1 \
         | grep -E 'test result|^test |FAILED|DIVERGENCE|panicked'; then :; fi
    # shellcheck disable=SC2181
    if ! DRIVER_RUST_SO="$PWD/$so" timeout 600 cargo test "${flags[@]}" >/dev/null 2>&1; then
      echo "  >>> FAILED: $label [$profile]"
      FAIL=1
    else
      echo "  >>> PASSED: $label [$profile]"
    fi
  done
done

echo
if (( FAIL )); then echo "SOME CONFIGURATIONS FAILED"; exit 1; fi
echo "ALL CONFIGURATIONS PASSED"
