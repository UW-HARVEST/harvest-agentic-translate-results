#!/usr/bin/env bash
# Differential test driver.
#
# `cargo test` alone does NOT rebuild a cdylib-only lib target (an integration
# test cannot link a cdylib, so cargo sees no dependency). This script builds
# the C .so and the Rust .so first, then runs the differential tests against
# BOTH the debug and the release Rust artifact, for EVERY feature combination.

set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
CARGO="cargo --offline"
fail=0

echo "=== building C shared library ==="
( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Feature combinations. Cargo.toml has no [features] table, so the only
# configuration is the default; the loop is written generically anyway so it
# keeps working if features are added later.
# ---------------------------------------------------------------------------
FEATS=$(python3 - <<'PY'
import re, itertools, sys
try:
    s = open('Cargo.toml').read()
except OSError:
    print(''); sys.exit()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
combos = []
for r in range(len(names) + 1):
    for c in itertools.combinations(names, r):
        combos.append(','.join(c))
print('\n'.join(combos) if combos else '')
PY
)

run_combo() {
    local featflags="$1" label="$2"
    for profile in debug release; do
        echo
        echo "=== [$label / $profile] building Rust cdylib ==="
        if [ "$profile" = release ]; then
            PROFFLAG="--release"
            $CARGO build --release $featflags || { echo "rust build FAILED"; fail=1; continue; }
            RS_SO="target/release/libhex2bin_lib.so"
        else
            PROFFLAG=""
            $CARGO build $featflags || { echo "rust build FAILED"; fail=1; continue; }
            RS_SO="target/debug/libhex2bin_lib.so"
        fi
        if [ ! -f "$RS_SO" ]; then
            echo "MISSING Rust .so at $RS_SO"; fail=1; continue
        fi
        echo "Rust .so: $RS_SO"

        echo "--- symbol parity ---"
        diff <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort) \
             <(nm -D --defined-only "$RS_SO" | awk '$2 ~ /^[TtBbDdRr]$/ {print $3}' | sort) \
             && echo "symbol diff EMPTY (ok)" \
             || { echo "SYMBOL PARITY FAILURE"; fail=1; }

        echo "--- differential tests ---"
        HEX2BIN_RUST_SO="$(pwd)/$RS_SO" timeout 600 $CARGO test $PROFFLAG $featflags -- --test-threads=4
        if [ $? -ne 0 ]; then echo "TESTS FAILED [$label / $profile]"; fail=1; fi
    done
}

if [ -z "$FEATS" ]; then
    run_combo "" "default (no [features] in Cargo.toml)"
else
    while IFS= read -r combo; do
        [ -z "${combo+x}" ] && continue
        if [ -z "$combo" ]; then
            run_combo "--no-default-features" "no-default-features"
        else
            run_combo "--no-default-features --features $combo" "features=$combo"
        fi
    done <<< "$FEATS"
    run_combo "" "default features"
fi

echo
if [ "$fail" -eq 0 ]; then
    echo "########## ALL DIFFERENTIAL TESTS PASSED ##########"
else
    echo "########## FAILURES PRESENT ##########"
fi
exit $fail
