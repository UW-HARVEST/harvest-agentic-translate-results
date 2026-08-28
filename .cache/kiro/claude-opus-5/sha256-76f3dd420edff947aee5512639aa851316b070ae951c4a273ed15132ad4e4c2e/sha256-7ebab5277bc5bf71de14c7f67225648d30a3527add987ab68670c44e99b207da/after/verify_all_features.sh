#!/usr/bin/env bash
# Enumerates every valid feature combination declared in translation/Cargo.toml
# and runs `cargo check` + `cargo test` for each one.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# --- 0. Make sure the C reference library exists ---------------------------
if ! ls "$ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
  echo "Building the C reference library..."
  (mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .) >/tmp/c_build.log 2>&1 ||
    { echo "C build failed, see /tmp/c_build.log"; exit 1; }
fi
echo "C reference library: $(ls "$ROOT"/c_src/build/lib*.so)"

cd "$ROOT/translation" || exit 1

# --- 1. Extract feature names from the [features] table -------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "Declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- 2. Build the list of combinations to test ----------------------------
# With N features there are 2^N subsets; N==0 leaves just the empty set.
COMBOS=()
N=${#FEATURES[@]}
if [ "$N" -eq 0 ]; then
  COMBOS+=("")
else
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "Feature combinations to verify: ${#COMBOS[@]}"

FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  echo
  echo "=============================================================="
  echo "  combination: $label"
  echo "=============================================================="

  args=(--no-default-features)
  [ -n "$combo" ] && args+=(--features "$combo")
  export MATHOP_TEST_FEATURES="$combo"

  log="/tmp/fc_check_${combo//,/_}.log"
  if timeout 600 cargo check "${args[@]}" --all-targets >"$log" 2>&1; then
    echo "  cargo check : OK"
  else
    echo "  cargo check : FAILED (see $log)"; tail -30 "$log"; FAIL=1; continue
  fi

  for profile in dev release; do
    pargs=("${args[@]}")
    [ "$profile" = release ] && pargs+=(--release)
    log="/tmp/fc_test_${profile}_${combo//,/_}.log"
    if timeout 600 cargo test "${pargs[@]}" -- --test-threads=1 >"$log" 2>&1; then
      echo "  cargo test ($profile) : OK"
      grep -E '^test result:' "$log" | sed 's/^/    /'
    else
      echo "  cargo test ($profile) : FAILED (see $log)"; tail -40 "$log"; FAIL=1
    fi
  done

  # Symbol parity for this combination's cdylib.
  for profile_dir in debug release; do
    rso="target/$profile_dir/libmathop_lib.so"
    [ -f "$rso" ] || continue
    missing=$(comm -23 \
      <(nm -D --defined-only --format=posix "$ROOT"/c_src/build/lib*.so | awk '{print $1}' | sort -u) \
      <(nm -D --defined-only --format=posix "$rso" | awk '{print $1}' | sort -u))
    if [ -z "$missing" ]; then
      echo "  nm parity ($profile_dir) : OK"
    else
      echo "  nm parity ($profile_dir) : MISSING -> $missing"; FAIL=1
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$FAIL"
