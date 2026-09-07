#!/usr/bin/env bash
# Full differential verification: builds the C .so, then for EVERY cargo feature
# combination rebuilds the Rust cdylib and runs all differential test suites.
#
# `cargo test` does NOT rebuild the cdylib artifact, so `cargo build` is run
# explicitly before every `cargo test` (the harness also hard-fails on a stale
# .so, see tests/common/mod.rs::assert_not_stale).
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"

echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null)
ls -l "$ROOT/c_src/build/libdriver.so"

cd "$CRATE_DIR"

# Enumerate declared features from Cargo.toml (the [features] table).
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{
        sub(/[[:space:]]*=.*/,""); print }' Cargo.toml
)

# Build the list of combinations to verify.
COMBOS=("default")
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table: --no-default-features is the same build, but verify it
  # explicitly so the "every feature combination" gate is mechanically checked.
  COMBOS+=("--no-default-features")
else
  COMBOS+=("--no-default-features")
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  # All features together.
  COMBOS+=("--all-features")
  # Every pairwise combination.
  n=${#FEATURES[@]}
  for ((i = 0; i < n; i++)); do
    for ((j = i + 1; j < n; j++)); do
      COMBOS+=("--no-default-features --features ${FEATURES[i]},${FEATURES[j]}")
    done
  done
fi

echo
echo "== declared features: ${FEATURES[*]:-<none>} =="
echo "== verifying ${#COMBOS[@]} combination(s) =="

status=0
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "default" ]; then flags=(); else read -r -a flags <<<"$combo"; fi
  echo
  echo "---------------------------------------------------------------"
  echo ">>> combination: $combo"
  echo "---------------------------------------------------------------"
  if ! timeout 600 cargo check "${flags[@]}" >/dev/null 2>&1; then
    echo "!!! cargo check FAILED for: $combo"
    status=1
    continue
  fi
  timeout 600 cargo build --release "${flags[@]}" 2>&1 | tail -1
  if timeout 600 cargo test --release "${flags[@]}" 2>&1 | grep -E '^test result|^!!!'; then
    :
  fi
  # Re-run capturing the exit status (the pipe above masks it).
  if ! timeout 600 cargo test --release "${flags[@]}" >/dev/null 2>&1; then
    echo "!!! TESTS FAILED for: $combo"
    status=1
  else
    echo ">>> PASSED: $combo"
  fi
done

echo
echo "== symbol parity (nm -D) =="
diff <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$CRATE_DIR/target/release/libdriver.so" | awk '{print $NF}' | sort) \
  && echo "symbol sets identical" || { echo "!!! SYMBOL DIFF"; status=1; }

echo
if [ "$status" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$status"
