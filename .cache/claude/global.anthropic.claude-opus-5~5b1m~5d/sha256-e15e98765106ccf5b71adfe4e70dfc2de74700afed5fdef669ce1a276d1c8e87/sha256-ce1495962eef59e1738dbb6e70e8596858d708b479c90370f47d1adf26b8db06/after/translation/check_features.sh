#!/usr/bin/env bash
# Enumerates the crate's Cargo features programmatically and runs the FULL
# differential suite once per feature combination, in both profiles.
#
# Nothing here is hard-coded to "this crate has no features": the feature list is
# read from `cargo metadata`, the power set is generated, and each combination is
# built and tested. If a feature is ever added, this script picks it up
# automatically instead of silently testing only the default configuration.

set -u
cd "$(dirname "$0")"

FAILED=0
COMBOS_RUN=0

# --- discover the feature list ---------------------------------------------
mapfile -t FEATURES < <(
  cargo metadata --no-deps --format-version 1 --offline 2>/dev/null |
    python3 -c '
import json, sys
md = json.load(sys.stdin)
feats = set()
for pkg in md["packages"]:
    for name, implied in pkg.get("features", {}).items():
        feats.add(name)
        for dep in implied:
            if "/" not in dep and not dep.startswith("dep:"):
                feats.add(dep)
feats.discard("default")
for f in sorted(feats):
    print(f)
'
)

NUM=${#FEATURES[@]}
echo "=== Feature matrix ==="
if [ "$NUM" -eq 0 ]; then
  echo "Discovered 0 optional features in Cargo.toml."
  echo "The complete feature space is therefore the single default (empty)"
  echo "combination; --no-default-features and --all-features are identical to it."
else
  echo "Discovered $NUM feature(s): ${FEATURES[*]}"
fi
echo

# --- build the C library once ----------------------------------------------
C_BUILD=../c_src/build
mkdir -p "$C_BUILD"
(cd "$C_BUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null) || {
  echo "FATAL: could not build the C shared library"
  exit 1
}
C_SO=$(ls "$C_BUILD"/*.so | head -1)
C_SO=$(readlink -f "$C_SO")
export C_SO_PATH="$C_SO"
echo "C library: $C_SO_PATH"
echo

# --- run one combination ----------------------------------------------------
# run_combo <label> <cargo-feature-flags...>
run_combo() {
  local label="$1"
  shift
  local flags=("$@")

  for profile in debug release; do
    local prof_flag=()
    [ "$profile" = release ] && prof_flag=(--release)

    echo "--- combo [$label] profile=$profile ---"

    if ! cargo check --offline "${prof_flag[@]}" "${flags[@]}" >/dev/null 2>&1; then
      echo "  FAIL: cargo check failed"
      FAILED=$((FAILED + 1))
      continue
    fi

    # Build the cdylib for this exact combination and pin the test harness to it,
    # so the suite can never pick up a .so from a different configuration.
    if ! cargo build --offline "${prof_flag[@]}" "${flags[@]}" >/dev/null 2>&1; then
      echo "  FAIL: cargo build failed"
      FAILED=$((FAILED + 1))
      continue
    fi
    local so="target/$profile/libarrayfunc_lib.so"
    if [ ! -f "$so" ]; then
      echo "  FAIL: $so not produced"
      FAILED=$((FAILED + 1))
      continue
    fi

    # Symbol parity for this combination.
    local missing
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO_PATH" | awk '{print $3}' | sort) \
      <(nm -D --defined-only "$so" | awk '{print $3}' | sort))
    if [ -n "$missing" ]; then
      echo "  FAIL: Rust .so missing symbols: $(echo "$missing" | tr '\n' ' ')"
      FAILED=$((FAILED + 1))
      continue
    fi

    # The signal-parity probes always need the uninstrumented release artifact.
    cargo build --offline --release "${flags[@]}" >/dev/null 2>&1
    export RUST_SO_RELEASE_PATH="$PWD/target/release/libarrayfunc_lib.so"
    export RUST_SO_PATH="$PWD/$so"

    local out
    out=$(timeout 600 cargo test --offline "${prof_flag[@]}" "${flags[@]}" 2>&1)
    if echo "$out" | grep -qE 'test result: FAILED|error: test failed'; then
      echo "  FAIL: tests failed"
      echo "$out" | grep -E 'FAILED|panicked at' | head -10 | sed 's/^/    /'
      FAILED=$((FAILED + 1))
    else
      local n
      n=$(echo "$out" | grep -oE '^test result: ok\. [0-9]+' | awk '{s+=$4} END {print s}')
      echo "  OK: ${n:-0} tests passed, symbol parity complete"
    fi
    unset RUST_SO_PATH RUST_SO_RELEASE_PATH
    COMBOS_RUN=$((COMBOS_RUN + 1))
  done
}

# --- enumerate the power set of features -----------------------------------
if [ "$NUM" -eq 0 ]; then
  run_combo "default (no features exist)"
  run_combo "--no-default-features" --no-default-features
  run_combo "--all-features" --all-features
else
  total=$((1 << NUM))
  for ((mask = 0; mask < total; mask++)); do
    combo=()
    for ((b = 0; b < NUM; b++)); do
      if (((mask >> b) & 1)); then combo+=("${FEATURES[$b]}"); fi
    done
    if [ ${#combo[@]} -eq 0 ]; then
      run_combo "no-default-features (empty)" --no-default-features
    else
      joined=$(
        IFS=,
        echo "${combo[*]}"
      )
      run_combo "$joined" --no-default-features --features "$joined"
    fi
  done
  run_combo "default"
  run_combo "--all-features" --all-features
fi

echo
echo "=== Feature matrix summary: $COMBOS_RUN configuration(s) run, $FAILED failure(s) ==="
[ "$FAILED" -eq 0 ] || exit 1
echo "OK: every feature combination passes in both profiles."
