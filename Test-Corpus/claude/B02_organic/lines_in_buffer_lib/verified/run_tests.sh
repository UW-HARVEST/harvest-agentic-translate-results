#!/usr/bin/env bash
# Differential test driver.
#
# MUST be used instead of a bare `cargo test`: `crate-type = ["cdylib"]` means
# `cargo test` does NOT build target/<profile>/libdriver.so, so a bare
# `cargo test` silently exercises a STALE .so and passes unconditionally.
#
# Usage:  ./run_tests.sh [--release] [extra cargo test args...]
set -uo pipefail
cd "$(dirname "$0")"

PROFILE_ARGS=()
PROFILE_DIR=debug
EXTRA=()
for a in "$@"; do
  case "$a" in
    --release) PROFILE_ARGS+=(--release); PROFILE_DIR=release ;;
    *) EXTRA+=("$a") ;;
  esac
done

# ---- 1. build the C reference shared library -------------------------------
if [[ ! -f c_src/build/libdriver.so ]]; then
  echo "== building C reference library =="
  mkdir -p c_src/build
  ( cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

# ---- 2. enumerate feature combinations ------------------------------------
# Cargo.toml has no [features] section, so the only combination is the empty
# one. Detected here rather than hard-coded, so new features are picked up.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml
)

COMBOS=("")
if (( ${#FEATURES[@]} > 0 )); then
  n=${#FEATURES[@]}
  COMBOS=()
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== feature combinations to verify: ${#COMBOS[@]} =="
for c in "${COMBOS[@]}"; do echo "   - '${c:-<none>}'"; done

# ---- 3. build the Rust cdylib + run the suite for each combination --------
RC=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  echo
  echo "============================================================"
  echo "== features: $label   profile: $PROFILE_DIR"
  echo "============================================================"

  FARGS=(--no-default-features)
  [[ -n "$combo" ]] && FARGS+=(--features "$combo")

  # CRITICAL: build the cdylib explicitly. `cargo test` will not do it.
  if ! timeout 600 cargo build "${FARGS[@]}" "${PROFILE_ARGS[@]}"; then
    echo "!! cargo build FAILED for features '$label'"; RC=1; continue
  fi

  export RUST_SO="$PWD/target/$PROFILE_DIR/libdriver.so"
  export C_SO="$PWD/c_src/build/libdriver.so"

  if ! timeout 600 cargo test "${FARGS[@]}" "${PROFILE_ARGS[@]}" "${EXTRA[@]}"; then
    echo "!! cargo test FAILED for features '$label'"; RC=1
  fi
done

echo
if (( RC == 0 )); then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $RC
