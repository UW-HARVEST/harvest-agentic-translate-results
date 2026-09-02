#!/usr/bin/env bash
# Phase D: run the whole differential suite under EVERY cargo feature
# combination. The feature list is extracted from the manifest rather than
# hard-coded, so a newly added feature is picked up automatically.
set -uo pipefail
cd "$(dirname "$0")"

# --- Enumerate features mechanically -------------------------------------
FEATURES=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c '
import json,sys
m=json.load(sys.stdin)
feats=set()
for p in m["packages"]:
    for f in p.get("features",{}):
        if f != "default":
            feats.add(f)
print(" ".join(sorted(feats)))')

read -r -a FEATS <<<"$FEATURES"
echo "features declared: ${#FEATS[@]} (${FEATURES:-none})"

# --- Build the list of combinations to test ------------------------------
# Always include the default build and the no-default-features build, then
# every subset of the declared features.
COMBOS=("--default" "--no-default-features")
n=${#FEATS[@]}
if (( n > 0 )); then
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATS[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${combo}")
    COMBOS+=("--features ${combo}")
  done
fi

# --- Rebuild the C library (ground truth) -------------------------------
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

fail=0
for combo in "${COMBOS[@]}"; do
  flags="$combo"
  [[ "$combo" == "--default" ]] && flags=""
  echo "=============================================================="
  echo "combo: ${combo}"
  echo "=============================================================="

  # The tests dlopen the release .so, so it must be rebuilt for each combo.
  if ! timeout 600 cargo build --release $flags >/tmp/fc_build.log 2>&1; then
    echo "  BUILD FAILED"; tail -20 /tmp/fc_build.log; fail=1; continue
  fi
  if ! timeout 600 cargo clippy --release $flags --all-targets \
        >/tmp/fc_clippy.log 2>&1; then
    echo "  (clippy unavailable or warned — not fatal)"
  fi
  if ! timeout 600 cargo test --release $flags -- --test-threads=1 \
        >/tmp/fc_test.log 2>&1; then
    echo "  TESTS FAILED"; grep -E '^(test |error|failures:)' /tmp/fc_test.log | tail -40; fail=1; continue
  fi
  grep -E '^test result:' /tmp/fc_test.log | sed 's/^/  /'
done

echo "=============================================================="
if (( fail )); then echo "RESULT: FAILURES PRESENT"; exit 1; fi
echo "RESULT: all feature combinations pass"
