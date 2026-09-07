#!/usr/bin/env bash
# Runs the differential suite across every build/feature configuration.
#
# `translation/Cargo.toml` declares no `[features]` table, so the feature
# cross-product is a single element (default == --no-default-features == --all-features).
# The loop is written generically anyway, and additionally sweeps both build
# profiles of the Rust cdylib, since only the profile can change codegen here.
set -uo pipefail
cd "$(dirname "$0")"

FEATURES=$(python3 - <<'PY'
import re,sys
src=open('Cargo.toml').read()
m=re.search(r'^\[features\](.*?)(^\[|\Z)', src, re.S|re.M)
if not m:
    print(''); sys.exit()
print(' '.join(re.findall(r'^\s*([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)))
PY
)
echo "features declared in Cargo.toml: '${FEATURES:-<none>}'"

if [ -z "$FEATURES" ]; then
  COMBOS=("--no-default-features")
else
  COMBOS=("--no-default-features" "--all-features")
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
fi

# Make sure the C reference is built.
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

rc=0
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"
    echo "=============================================================="
    echo "combo='${combo}' profile=${profile}"
    # shellcheck disable=SC2086
    timeout 600 cargo build $relflag $combo >/dev/null 2>&1 \
      || { echo "  BUILD FAILED"; rc=1; continue; }
    so="target/${profile}/libdriver.so"
    echo "  nm -D --defined-only ${so}:"
    nm -D --defined-only "$so" | grep -vE ' _| rust_' | sed 's/^/    /'
    # shellcheck disable=SC2086
    DRIVER_RUST_SO="$PWD/$so" timeout 600 cargo test $relflag $combo -- --nocapture 2>&1 \
      | grep -E 'phase|test result|DIVERGENCE|panicked' | sed 's/^/    /'
    # shellcheck disable=SC2086
    DRIVER_RUST_SO="$PWD/$so" timeout 600 cargo test $relflag $combo >/dev/null 2>&1 \
      || { echo "  TESTS FAILED"; rc=1; }
  done
done
echo "=============================================================="
[ $rc -eq 0 ] && echo "ALL CONFIGURATIONS PASSED" || echo "SOME CONFIGURATIONS FAILED"
exit $rc
