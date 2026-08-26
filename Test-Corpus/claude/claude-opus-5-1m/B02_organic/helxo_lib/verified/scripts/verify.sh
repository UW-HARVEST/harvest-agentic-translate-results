#!/bin/bash
# Full verification run: builds the C .so and the Rust .so, checks symbol
# parity and runs the whole differential suite for EVERY feature combination.
#
# `Cargo.toml` has no [features] table and c_src/CMakeLists.txt has no build
# options, so there is exactly one configuration; the loop below is generated
# from Cargo.toml so it stays correct if features are ever added.
set -euo pipefail
cd "$(dirname "$0")/.."

CARGO_FLAGS="--offline"

echo "=== 1. building the C shared library ==="
(mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
    && cmake --build . > /dev/null)
ls -l c_src/build/libtranslated_rust.so

echo
echo "=== 2. enumerating feature combinations ==="
FEATURES=$(python3 - <<'PY'
import re,sys
txt=open('Cargo.toml').read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M|re.S)
feats=[]
if m:
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if not line or '=' not in line: continue
        name=line.split('=')[0].strip()
        if name!='default': feats.append(name)
print(' '.join(feats))
PY
)
if [ -z "$FEATURES" ]; then
    COMBOS=("")
    echo "no [features] -> 1 combination (default/empty)"
else
    COMBOS=()
    n=$(echo "$FEATURES" | wc -w)
    arr=($FEATURES)
    for ((mask=0; mask<(1<<n); mask++)); do
        c=""
        for ((i=0; i<n; i++)); do
            if (( mask & (1<<i) )); then c="$c,${arr[$i]}"; fi
        done
        COMBOS+=("${c#,}")
    done
    echo "features: $FEATURES -> ${#COMBOS[@]} combinations"
fi

for combo in "${COMBOS[@]}"; do
    echo
    echo "=== 3. combination: '${combo:-<none>}' ==="
    echo "--- cargo check ---"
    cargo check $CARGO_FLAGS --no-default-features ${combo:+--features "$combo"} --all-targets
    echo "--- cargo build (cdylib) ---"
    cargo build $CARGO_FLAGS --no-default-features ${combo:+--features "$combo"}
    echo "--- symbol parity ---"
    ./scripts/check_symbols.sh
    echo "--- differential test suite (dev profile: debug assertions + overflow checks) ---"
    cargo test $CARGO_FLAGS --no-default-features ${combo:+--features "$combo"}
    echo "--- differential test suite, single threaded ---"
    cargo test $CARGO_FLAGS --no-default-features ${combo:+--features "$combo"} -- --test-threads=1
    echo "--- release profile (optimized, panic=abort cdylib) ---"
    cargo build $CARGO_FLAGS --release --no-default-features ${combo:+--features "$combo"}
    ./scripts/check_symbols.sh target/release/libhelxo_lib.so
    cargo test $CARGO_FLAGS --release --no-default-features ${combo:+--features "$combo"}
done

echo
echo "=== ALL COMBINATIONS VERIFIED ==="
