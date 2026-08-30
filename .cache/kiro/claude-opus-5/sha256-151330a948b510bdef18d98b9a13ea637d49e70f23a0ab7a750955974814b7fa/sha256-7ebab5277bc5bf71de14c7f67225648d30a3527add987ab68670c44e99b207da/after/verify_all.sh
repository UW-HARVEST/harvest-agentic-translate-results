#!/usr/bin/env bash
# Verify the translation across every build-time configuration.
#
# Cargo.toml declares no [features] and CMakeLists.txt declares no options, so
# the only build-time axes are the cargo feature *modes* (all equivalent to the
# empty feature set here) and the cargo profile. This script enumerates them
# from Cargo.toml rather than hard-coding, so a future [features] section is
# picked up automatically.
set -uo pipefail

cd "$(dirname "$0")/translation" || exit 1
ROOT="$(cd .. && pwd)"

# ---- enumerate every valid feature combination -----------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default one."
  COMBOS=("--no-default-features" "" "--all-features")
else
  # Power set of the optional features, with and without default features.
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    joined=$(
      IFS=,
      echo "${sel[*]}"
    )
    if [ -z "$joined" ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $joined")
    fi
  done
  COMBOS+=("" "--all-features")
fi

echo "== ${#COMBOS[@]} configuration(s) to verify =="
for c in "${COMBOS[@]}"; do echo "   cargo <cmd> ${c:-<default>}"; done
echo

# ---- build the C reference library ----------------------------------------
echo "== building the C reference library =="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null
) || {
  echo "FAIL: C build"
  exit 1
}
nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print "   C exports: " $NF}'
echo

# ---- check / build / test every combination, in both profiles -------------
fails=0
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    tag="${combo:-<default>} ${profile:-<debug>}"

    printf '== cargo check   %s\n' "$tag"
    if ! timeout 600 cargo check $combo $profile --all-targets >/tmp/dv.log 2>&1; then
      echo "   FAIL"
      tail -30 /tmp/dv.log
      fails=$((fails + 1))
      continue
    fi

    printf '== cargo build   %s\n' "$tag"
    if ! timeout 600 cargo build $combo $profile >/tmp/dv.log 2>&1; then
      echo "   FAIL"
      tail -30 /tmp/dv.log
      fails=$((fails + 1))
      continue
    fi

    printf '== cargo test    %s\n' "$tag"
    if ! timeout 600 cargo test $combo $profile -- --test-threads=1 >/tmp/dv.log 2>&1; then
      echo "   FAIL"
      grep -E "^(test|failures:|error|  DIFF|  FLAKY)|panicked|mismatch" /tmp/dv.log | head -40
      fails=$((fails + 1))
      continue
    fi
    grep -E "^test result:" /tmp/dv.log | sed 's/^/   /'
  done
done

echo
if [ "$fails" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "$fails configuration(s) FAILED"
fi
exit "$fails"
