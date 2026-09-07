#!/usr/bin/env bash
# Full C-vs-Rust differential verification gate.
#
# Rebuilds both libraries, enumerates every cargo feature combination declared in
# Cargo.toml, and runs the whole differential suite for each combination against
# BOTH Rust cdylib profiles (release sets panic = "abort", so it is a different
# artifact than debug).
#
# Usage:  ./verify.sh [--fast]
#           --fast   skip the multi-gigabyte large-trip-count tests
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
C_DIR="$CRATE_DIR/../c_src"

FAST=0
[[ "${1:-}" == "--fast" ]] && FAST=1

FAILURES=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAILURES=$((FAILURES+1)); }

# ---------------------------------------------------------------- build the C
step "Building the C shared library"
mkdir -p "$C_DIR/build"
if ( cd "$C_DIR/build" \
     && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/verify_cmake.log 2>&1 \
     && timeout 600 cmake --build . >>/tmp/verify_cmake.log 2>&1 ); then
  ok "C library built"
else
  bad "C build failed (see /tmp/verify_cmake.log)"; tail -20 /tmp/verify_cmake.log; exit 1
fi
C_SO=$(find "$C_DIR/build" -maxdepth 1 -name '*.so' | head -1)
[[ -f "$C_SO" ]] && ok "C .so: $C_SO" || { bad "no C .so produced"; exit 1; }

# --------------------------------------------------- enumerate feature combos
step "Enumerating cargo feature combinations"
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /=/      {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] !~ /^#/ && a[1]!="") print a[1]}
' Cargo.toml)

if [[ ${#FEATURES[@]} -eq 0 ]]; then
  echo "  Cargo.toml declares no [features] -> the default build is the only configuration."
  COMBOS=("<default>")
else
  echo "  declared features: ${FEATURES[*]}"
  # Full power set of the declared features.
  COMBOS=("<default>" "<none>")
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && combo+="${FEATURES[$i]},"
    done
    COMBOS+=("${combo%,}")
  done
fi
echo "  combinations to verify: ${#COMBOS[@]}"

feature_args() {
  case "$1" in
    "<default>") echo "" ;;
    "<none>")    echo "--no-default-features" ;;
    *)           echo "--no-default-features --features $1" ;;
  esac
}

# --------------------------------------------------------- cargo check matrix
step "cargo check across every feature combination"
for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2046
  if timeout 600 cargo check --quiet $(feature_args "$combo") >/tmp/verify_check.log 2>&1; then
    ok "cargo check $combo"
  else
    bad "cargo check $combo"; tail -20 /tmp/verify_check.log
  fi
done

# ---------------------------------------------------------- build both cdylibs
step "Building the Rust cdylib (debug + release)"
for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2046
  timeout 600 cargo build --quiet $(feature_args "$combo") >/tmp/verify_build.log 2>&1 \
    && ok "debug build $combo" || { bad "debug build $combo"; tail -20 /tmp/verify_build.log; }
  # shellcheck disable=SC2046
  timeout 600 cargo build --quiet --release $(feature_args "$combo") >/tmp/verify_build.log 2>&1 \
    && ok "release build $combo" || { bad "release build $combo"; tail -20 /tmp/verify_build.log; }
done

# ------------------------------------------------------------- symbol parity
step "Symbol parity: nm -D on the C .so vs the Rust .so"
for profile in debug release; do
  RUST_SO="$CRATE_DIR/target/$profile/libpremultiply_lib.so"
  [[ -f "$RUST_SO" ]] || { bad "missing $RUST_SO"; continue; }
  MISSING=$(comm -23 \
    <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u))
  if [[ -z "$MISSING" ]]; then
    ok "$profile: 0 C symbols missing from the Rust .so"
  else
    bad "$profile: Rust .so is missing: $(echo "$MISSING" | tr '\n' ' ')"
  fi
done

# ------------------------------------------------------------ the test matrix
step "Differential test suite: every combination x both cdylib profiles"
FAST_TESTS=(--test controls --test feature_matrix --test phase_b_valid --test phase_c_errors)
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    RUST_SO="$CRATE_DIR/target/$profile/libpremultiply_lib.so"
    export DIFF_RUST_SO="$RUST_SO"
    label="$combo / $profile cdylib"

    # shellcheck disable=SC2046
    if timeout 600 cargo test --quiet $(feature_args "$combo") "${FAST_TESTS[@]}" \
         >/tmp/verify_test.log 2>&1; then
      ok "fast suite  [$label]"
    else
      bad "fast suite  [$label]"; tail -40 /tmp/verify_test.log
    fi

    if [[ $FAST -eq 0 ]]; then
      # shellcheck disable=SC2046
      if timeout 600 cargo test --quiet $(feature_args "$combo") --test phase_c_large \
           -- --test-threads=1 >/tmp/verify_large.log 2>&1; then
        ok "large suite [$label]"
      else
        bad "large suite [$label]"; tail -40 /tmp/verify_large.log
      fi
    fi
    unset DIFF_RUST_SO
  done
done

# ------------------------------------------------------------------- verdict
step "Verdict"
if [[ $FAILURES -eq 0 ]]; then
  printf '  \033[32mALL CHECKS PASSED\033[0m\n'
  exit 0
else
  printf '  \033[31m%d CHECK(S) FAILED\033[0m\n' "$FAILURES"
  exit 1
fi
