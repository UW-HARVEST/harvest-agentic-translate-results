#!/usr/bin/env bash
# Phase D driver: symbol parity + the full Phase B/C suite under every
# feature combination and both codegen profiles.
#
# Usage: ./verify_all.sh        (run from the crate root, i.e. translation/)
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
FAIL=0
step() { printf '\n=== %s ===\n' "$1"; }

# ---------------------------------------------------------------------------
# Enumerate feature combinations declared in Cargo.toml (powerset).
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features]; the default build is the only configuration."
  COMBOS=("__default__")
else
  # shellcheck disable=SC2206
  FARR=($FEATURES)
  n=${#FARR[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("${combo:-__none__}")
  done
  COMBOS+=("__default__")
fi

# ---------------------------------------------------------------------------
run_combo() {
  local combo="$1" profile="$2"
  local featflags=() profflags=() target_dir

  case "$combo" in
    __default__) featflags=() ;;
    __none__) featflags=(--no-default-features) ;;
    *) featflags=(--no-default-features --features "$combo") ;;
  esac
  case "$profile" in
    release) profflags=(--release); target_dir=target/release ;;
    debug) profflags=(); target_dir=target/debug ;;
  esac

  step "build [features=$combo profile=$profile]"
  if ! timeout 600 cargo build "${profflags[@]}" "${featflags[@]}" >/tmp/build.log 2>&1; then
    tail -20 /tmp/build.log
    echo "BUILD FAILED [features=$combo profile=$profile]"
    FAIL=1
    return
  fi

  local rust_so="$PWD/$target_dir/libconfusion_lib.so"
  step "symbol parity [features=$combo profile=$profile]"
  local diff_out
  diff_out=$(diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
                  <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort))
  if [ -n "$diff_out" ]; then
    echo "SYMBOL DIFF NOT EMPTY [features=$combo profile=$profile]:"
    echo "$diff_out"
    FAIL=1
  else
    echo "symbol diff empty ($(nm -D --defined-only "$C_SO" | wc -l) symbols)"
  fi

  step "tests [features=$combo profile=$profile]"
  # Tests always link against the release-built test harness for speed, but
  # load the .so under test via RUST_SO_PATH so the profile really varies.
  if ! RUST_SO_PATH="$rust_so" timeout 600 cargo test --release --tests \
      >/tmp/test.log 2>&1; then
    grep -E "^test .* FAILED|panicked at|test result" /tmp/test.log | head -40
    echo "TESTS FAILED [features=$combo profile=$profile]"
    FAIL=1
  else
    grep -E "test result" /tmp/test.log
  fi
}

for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    run_combo "$combo" "$profile"
  done
done

printf '\n===================================\n'
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$FAIL"
