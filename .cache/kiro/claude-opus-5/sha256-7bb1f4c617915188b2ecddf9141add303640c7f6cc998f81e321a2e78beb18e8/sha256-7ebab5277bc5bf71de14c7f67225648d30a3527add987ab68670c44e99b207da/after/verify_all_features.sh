#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every valid
# build-time feature combination declared in Cargo.toml.
#
# Usage: ./verify_all_features.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
CSRC="$ROOT/c_src"
LOGDIR="/tmp/half2float-verify"
mkdir -p "$LOGDIR"

# ---------------------------------------------------------------- build the C .so
echo "== building C shared library =="
(
  mkdir -p "$CSRC/build" &&
  cd "$CSRC/build" &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
  cmake --build .
) >"$LOGDIR/cmake.log" 2>&1 || { tail -30 "$LOGDIR/cmake.log"; exit 1; }
ls "$CSRC"/build/lib*.so

# ------------------------------------------------- enumerate feature combinations
# Features are read from the [features] table in Cargo.toml. Every subset of the
# non-"default" features is a valid combination; the empty subset is
# --no-default-features.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  COMBOS=("")
else
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== ${#COMBOS[@]} feature combination(s): ${COMBOS[*]@Q} =="

# ------------------------------------------------------------ check / build / test
fail=0
for combo in "${COMBOS[@]}"; do
  if [[ -n "$combo" ]]; then
    args=(--no-default-features --features "$combo")
    label="$combo"
  else
    args=(--no-default-features)
    label="<none>"
  fi
  slug="${label//[^A-Za-z0-9]/_}"

  for phase in check build test; do
    case "$phase" in
      check) cmd=(cargo check "${args[@]}") ;;
      build) cmd=(cargo build --release "${args[@]}") ;;
      test)  cmd=(cargo test "${args[@]}") ;;
    esac
    log="$LOGDIR/${phase}_${slug}.log"
    if (cd "$CRATE" && TRANSLATION_TEST_FEATURE_ARGS="${args[*]}" \
        timeout 600 "${cmd[@]}") >"$log" 2>&1; then
      echo "PASS  $phase  features=$label"
    else
      echo "FAIL  $phase  features=$label  (see $log)"
      tail -30 "$log"
      fail=1
    fi
  done

  # Re-run the differential suite against the release cdylib built above, so the
  # optimised artifact and its exported symbols are verified too.
  rel_so="$CRATE/target/release/libhalf2float_lib.so"
  log="$LOGDIR/test_release_${slug}.log"
  if (cd "$CRATE" && TRANSLATION_RUST_SO="$rel_so" \
      TRANSLATION_TEST_FEATURE_ARGS="${args[*]}" \
      timeout 600 cargo test "${args[@]}") >"$log" 2>&1; then
    echo "PASS  test(release .so)  features=$label"
  else
    echo "FAIL  test(release .so)  features=$label  (see $log)"
    tail -30 "$log"
    fail=1
  fi
done

exit "$fail"
