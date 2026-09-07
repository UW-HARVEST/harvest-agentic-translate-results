#!/usr/bin/env bash
# Build the C .so, build the Rust cdylib, then run the differential suite
# across every feature combination and both profiles.
#
# `cargo build` is MANDATORY before `cargo test`: cargo does not refresh cdylib
# artifacts as part of `cargo test`, so without it the suite can load a stale
# .so. tests/differential.rs also guards against this.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE="$ROOT/translation"

echo "=== building C shared library ==="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
ls -l "$ROOT/c_src/build/libdriver.so"

cd "$CRATE"

# Feature combinations. The crate declares no [features], so the complete set of
# combinations is the single default build; this is derived, not assumed.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml || true)
if [ -n "$FEATURES" ]; then
  echo "=== declared features ==="; echo "$FEATURES"
else
  echo "=== no [features] in Cargo.toml: 1 feature combination (default) ==="
fi

FLAG_SETS=("" "--no-default-features" "--all-features")
for prof in "" "--release"; do
  for flags in "${FLAG_SETS[@]}"; do
    echo
    echo "=== cargo build ${prof} ${flags} ==="
    timeout 600 cargo build ${prof} ${flags}
    echo "=== cargo test  ${prof} ${flags} ==="
    timeout 600 cargo test ${prof} ${flags} 2>&1 | tail -n 40
  done
done

echo
echo "=== symbol parity (Phase D) ==="
diff <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$CRATE/target/release/libdriver.so" | awk '{print $NF}' | sort) \
  && echo "symbol diff EMPTY: Rust .so exports every symbol the C .so exports"
