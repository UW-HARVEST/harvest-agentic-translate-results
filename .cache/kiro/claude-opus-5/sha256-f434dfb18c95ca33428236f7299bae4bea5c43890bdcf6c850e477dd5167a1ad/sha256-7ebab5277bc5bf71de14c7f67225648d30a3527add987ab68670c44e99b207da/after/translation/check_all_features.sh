#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and run `cargo check` (and
# optionally `cargo test`) for each.
#
# Usage: ./check_all_features.sh [check|test]
set -uo pipefail

cd "$(dirname "$0")"
MODE="${1:-check}"

# Extract feature names from the [features] table of Cargo.toml, ignoring the
# implicit `default` feature (handled separately via --no-default-features).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "Declared features (excluding \`default\`): ${N}"
if [ "$N" -gt 0 ]; then
  printf '  - %s\n' "${FEATURES[@]}"
fi

# Build the list of combinations to exercise: the default configuration, plus
# every subset of the declared features with default features disabled.
COMBOS=()
COMBOS+=("<default>")
COMBOS+=("<none>")
if [ "$N" -gt 0 ]; then
  total=$((1 << N))
  for ((mask = 1; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "Combinations to ${MODE}: ${#COMBOS[@]}"
echo

# The differential tests dlopen the C shared library; build it once up front.
if [ "$MODE" = "test" ]; then
  C_SO="../c_src/build/libdriver.so"
  if [ ! -f "$C_SO" ]; then
    echo "Building the C shared library..."
    (cd ../c_src && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
      && cmake --build . > /dev/null) || { echo "C build failed"; exit 1; }
  fi
  echo "C shared library: $(readlink -f "$C_SO")"
  echo
fi

fail=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    "<default>")
      args=()
      unset DRIVER_TEST_NO_DEFAULT_FEATURES DRIVER_TEST_FEATURES
      ;;
    "<none>")
      args=(--no-default-features)
      export DRIVER_TEST_NO_DEFAULT_FEATURES=1
      unset DRIVER_TEST_FEATURES
      ;;
    *)
      args=(--no-default-features --features "$combo")
      export DRIVER_TEST_NO_DEFAULT_FEATURES=1
      export DRIVER_TEST_FEATURES="$combo"
      ;;
  esac

  printf '=== cargo %s %s ===\n' "$MODE" "${args[*]:-(default features)}"
  # The harness rebuilds the cdylib under test itself; clear its cache so each
  # combination is definitely built with that combination's features.
  rm -rf target/so-under-test
  if timeout 600 cargo "$MODE" "${args[@]}" > "/tmp/feat_${MODE}.log" 2>&1; then
    if [ "$MODE" = "test" ]; then
      grep -E '^test result:' "/tmp/feat_${MODE}.log" | sed 's/^/  /'
    fi
    echo "  PASS"
  else
    echo "  FAIL"
    tail -n 40 "/tmp/feat_${MODE}.log"
    fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED (${MODE})"
else
  echo "SOME COMBINATIONS FAILED (${MODE})"
fi
exit "$fail"
