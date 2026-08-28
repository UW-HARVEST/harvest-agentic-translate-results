#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and run `cargo check` plus the
# full differential test suite against each one.
#
# Usage: ./check_all_features.sh        (run from translation/)
set -uo pipefail
cd "$(dirname "$0")"

# Pull the declared features straight out of the manifest, excluding "default".
mapfile -t FEATURES < <(
  cargo metadata --no-deps --format-version 1 \
    | python3 -c '
import json,sys
md = json.load(sys.stdin)
pkg = md["packages"][0]
for f in sorted(pkg["features"]):
    if f != "default":
        print(f)
'
)

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of combinations: the powerset of the declared features, always
# with --no-default-features, plus the default configuration on its own.
COMBOS=()
COMBOS+=("--default")                       # sentinel: plain default build
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((bit = 0; bit < n; bit++)); do
    if ((mask & (1 << bit))); then
      combo+="${combo:+,}${FEATURES[bit]}"
    fi
  done
  COMBOS+=("$combo")
done

fail=0
for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "--default" ]]; then
    label="default features"
    args=()
  elif [[ -z "$combo" ]]; then
    label="no-default-features (empty feature set)"
    args=(--no-default-features)
  else
    label="no-default-features + $combo"
    args=(--no-default-features --features "$combo")
  fi

  echo "=============================================================="
  echo "## $label"
  echo "=============================================================="

  if ! timeout 600 cargo check "${args[@]}" > /tmp/check.log 2>&1; then
    echo "CHECK FAILED"; tail -30 /tmp/check.log; fail=1; continue
  fi
  echo "cargo check: OK"

  if ! timeout 600 cargo build --release "${args[@]}" > /tmp/build.log 2>&1; then
    echo "BUILD FAILED"; tail -30 /tmp/build.log; fail=1; continue
  fi
  echo "cargo build --release: OK"

  if ! timeout 600 cargo test "${args[@]}" > /tmp/test.log 2>&1; then
    echo "TESTS FAILED"; grep -E "^test |panicked|mismatch|missing" /tmp/test.log | head -40; fail=1; continue
  fi
  grep -E "^test result:" /tmp/test.log
done

echo "=============================================================="
if ((fail)); then
  echo "RESULT: at least one configuration failed"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} configuration(s) checked, built and tested clean"
