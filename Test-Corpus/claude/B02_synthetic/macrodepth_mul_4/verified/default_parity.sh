#!/bin/bash
# CONFIGS.md row 22 — the build-time DEFAULT fallback.
#
# C: compiled with NEITHER -DOP NOR -DREPEAT, so mdmacros.h:26-31 supplies
#    `add` / `5` itself.
# Rust: built with --no-default-features and no feature at all, so the `else`
#    arms of the OP / REPEAT constants supply `add` / `5`.
set -eu
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/artifacts/defaults"
mkdir -p "$OUT"
cd "$ROOT"

# no -DOP / -DREPEAT on the command line at all
gcc -shared -fPIC -I c_src/src -o "$OUT/libcdriver.so" c_src/src/mdcore.c
gcc -I c_src/src -o "$OUT/cdriver" c_src/src/mdcore.c c_src/src/mdmain.c

cargo build --quiet --no-default-features
cp target/debug/libdriver.so "$OUT/librdriver.so"
cp target/debug/driver       "$OUT/rdriver"

echo "--- symbol parity (implicit-default build) ---"
diff <(nm -D --defined-only "$OUT/libcdriver.so" | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$OUT/librdriver.so" | awk '{print $NF}' | sort) \
  && echo "symbol diff empty"

echo "--- differential tests (expecting the add/5 defaults) ---"
HARVEST_ARTIFACTS="$OUT" HARVEST_OP=add HARVEST_REPEAT=5 \
  timeout 600 cargo test --no-default-features -- --test-threads=1
