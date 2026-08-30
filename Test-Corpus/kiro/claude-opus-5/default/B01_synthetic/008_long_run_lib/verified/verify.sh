#!/usr/bin/env bash
# Full verification sweep: build the C reference, then cargo check + cargo test
# the Rust translation under every valid feature combination.
#
# Usage: ./verify.sh [check-only]
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
fail=0

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations declared in Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' "$here/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table: the crate has exactly one build configuration.
  COMBOS=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then combo="${combo:+$combo,}${FEATURES[$b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== feature combinations (${#COMBOS[@]}) =="
for c in "${COMBOS[@]}"; do echo "   '--no-default-features --features ${c:-<none>}'"; done

# ---------------------------------------------------------------------------
# 2. Build the C reference shared object.
# ---------------------------------------------------------------------------
echo "== building C reference =="
(
  cd "$root/c_src" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null
) || { echo "FAIL: C build"; exit 1; }
ls -l "$root/c_src/build/liblong.so"

# ---------------------------------------------------------------------------
# 3. cargo check for every combination.
# ---------------------------------------------------------------------------
cd "$here"
for combo in "${COMBOS[@]}"; do
  echo "== cargo check --no-default-features --features '${combo:-<none>}' =="
  if timeout 600 cargo check --all-targets --no-default-features \
    ${combo:+--features "$combo"} 2>&1 | tail -3; then :; else fail=1; fi
done

[ "${1:-}" = "check-only" ] && { echo "check-only: done (fail=$fail)"; exit $fail; }

# ---------------------------------------------------------------------------
# 4. Build the Rust cdylib (release) and run the differential tests for every
#    combination. `long_exec` needs optimised code to finish in minutes.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  echo "== build+test --no-default-features --features '${combo:-<none>}' =="
  if ! timeout 600 cargo build --release --no-default-features \
    ${combo:+--features "$combo"} 2>&1 | tail -3; then
    echo "FAIL: build for '${combo:-<none>}'"
    fail=1
    continue
  fi
  nm -D -S --defined-only target/release/liblong.so

  # Fast suites first, then the ~8 minute end-to-end comparison.
  if ! timeout 600 cargo test --release --no-default-features ${combo:+--features "$combo"} \
    --lib --test kernel_compare --test symbol_parity 2>&1 | tail -20; then
    echo "FAIL: fast tests for '${combo:-<none>}'"
    fail=1
  fi
  if ! timeout 600 cargo test --release --no-default-features ${combo:+--features "$combo"} \
    --test long_exec_compare 2>&1 | tail -10; then
    echo "FAIL: long_exec test for '${combo:-<none>}'"
    fail=1
  fi
done

echo "== verify.sh finished (fail=$fail) =="
exit $fail
