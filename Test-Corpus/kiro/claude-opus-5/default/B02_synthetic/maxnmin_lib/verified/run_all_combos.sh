#!/usr/bin/env bash
# Phase D — run Phases B and C under EVERY cargo feature combination, for both
# the release and the debug Rust .so, and gate on symbol parity each time.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"
FAIL=0

# --- enumerate feature combinations from Cargo.toml -------------------------
mapfile -t FEATURES < <(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys; [print(f) for f in json.load(sys.stdin)["packages"][0]["features"]]')

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the combination list: default, --no-default-features, and every subset
# of the declared features. With zero declared features that collapses to the
# two configurations below, which is the complete set for this crate.
COMBOS=("default" "nodefault:")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("nodefault:$(IFS=,; echo "${sel[*]}")")
  done
fi

run_combo() {
  local combo="$1" profile="$2"
  local -a flags=()
  [ "$profile" = "release" ] && flags+=(--release)
  local label="$combo/$profile"
  if [ "$combo" != "default" ]; then
    flags+=(--no-default-features)
    local feats="${combo#nodefault:}"
    [ -n "$feats" ] && flags+=(--features "$feats")
  fi

  echo "=============================================================="
  echo "COMBO: $label   (cargo ${flags[*]})"
  echo "=============================================================="

  if ! timeout 600 cargo build "${flags[@]}" >/tmp/build_$$.log 2>&1; then
    echo "BUILD FAILED ($label)"; tail -20 /tmp/build_$$.log; FAIL=1; return
  fi

  local so="target/$profile/libmaxnmin_lib.so"
  if ! ./check_symbols.sh "$(pwd)/$so"; then
    echo "SYMBOL PARITY FAILED ($label)"; FAIL=1
  fi

  # Point the tests at exactly the .so we just built for this combination.
  if ! RUST_SO="$(pwd)/$so" timeout 600 cargo test "${flags[@]}" >/tmp/test_$$.log 2>&1; then
    echo "TESTS FAILED ($label)"; grep -E 'FAILED|panicked|assertion|test result' /tmp/test_$$.log | head -30; FAIL=1
  else
    grep -E 'test result' /tmp/test_$$.log
  fi
}

for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    run_combo "$combo" "$profile"
  done
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit "$FAIL"
