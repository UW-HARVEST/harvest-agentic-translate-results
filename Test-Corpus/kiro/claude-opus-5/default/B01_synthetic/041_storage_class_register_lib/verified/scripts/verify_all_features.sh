#!/usr/bin/env bash
# Enumerates every valid feature combination declared in translation/Cargo.toml
# and runs `cargo check` + `cargo test` for each, in both dev and release
# profiles. Also (re)builds the C shared library that the tests compare against.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"
STATUS=0

echo "=== building C shared library ==="
(cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }

# Additional C build types, kept out of c_src so that tree stays untouched.
# `driver` relies on signed overflow, so it is worth confirming the Rust
# translation matches the C compiler at every optimisation level.
C_SOS=("$ROOT/c_src/build/libdriver.so")
for bt in Debug Release MinSizeRel RelWithDebInfo; do
  bdir="$PWD/target/c-build-$bt"
  if cmake -S "$ROOT/c_src" -B "$bdir" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
       -DCMAKE_BUILD_TYPE="$bt" >/dev/null 2>&1 \
     && cmake --build "$bdir" >/dev/null 2>&1; then
    C_SOS+=("$bdir/libdriver.so")
  else
    echo "warning: could not build C variant $bt"
  fi
done

# Extract feature names from the [features] table, skipping "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

# All 2^n subsets of the feature set (n == 0 yields the single empty combo).
COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (( mask & (1 << i) )); then
      combo="${combo:+$combo,}${FEATURES[i]}"
    fi
  done
  COMBOS+=("$combo")
done

echo "=== ${#FEATURES[@]} feature(s): ${FEATURES[*]:-<none>} ==="
echo "=== ${#COMBOS[@]} combination(s) ==="

run() {
  local label="$1"; shift
  echo "--- $label: $* ---"
  if timeout 600 "$@" >/tmp/driver-verify.log 2>&1; then
    grep -E "^test result|^error" /tmp/driver-verify.log || tail -n 3 /tmp/driver-verify.log
  else
    STATUS=1
    echo "FAILED: $*"
    tail -n 40 /tmp/driver-verify.log
  fi
}

# Runs cargo test once per C build variant.
test_all_c_variants() {
  local label="$1"; shift
  for so in "${C_SOS[@]}"; do
    DRIVER_C_SO="$so" run "test    $label c='$(basename "$(dirname "$so")")'" "$@"
  done
}

for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    base=(--no-default-features)
    [[ -n $combo ]] && base+=(--features "$combo")
    [[ -n $profile ]] && base+=("$profile")
    label="combo='${combo:-<none>}' profile='${profile:-dev}'"
    # The integration tests dlopen the cdylib, so it must exist before the test
    # binary runs; cargo does not treat it as a test dependency.
    run "check   $label" cargo check "${base[@]}"
    run "cdylib  $label" cargo build "${base[@]}"
    test_all_c_variants "$label" cargo test "${base[@]}"
  done
done

# Also cover the default feature set explicitly.
for profile in "" "--release"; do
  base=()
  [[ -n $profile ]] && base+=("$profile")
  label="combo='<default>' profile='${profile:-dev}'"
  run "check   $label" cargo check "${base[@]}"
  run "cdylib  $label" cargo build "${base[@]}"
  test_all_c_variants "$label" cargo test "${base[@]}"
done

echo "=== overall: $([[ $STATUS -eq 0 ]] && echo PASS || echo FAIL) ==="
exit $STATUS
