#!/usr/bin/env bash
# Phase D: symbol parity + every feature combination.
#
# Enumerates the cargo features declared in translation/Cargo.toml, runs
# `cargo check` and the whole differential test suite for each combination, and
# diffs `nm -D` between the two shared objects.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$PWD
WORK=$ROOT/.work
mkdir -p "$WORK"

echo "=== 1. Rebuild both shared objects ==="
( cd c_src/build && cmake --build . >"$WORK/cbuild.log" 2>&1 ) || { tail -20 "$WORK/cbuild.log"; exit 1; }
( cd translation && cargo build --release --offline >"$WORK/rbuild.log" 2>&1 ) || { tail -20 "$WORK/rbuild.log"; exit 1; }
ls -l c_src/build/libpng.so translation/target/release/liblibpng.so

echo
echo "=== 2. nm -D symbol parity ==="
nm -D --defined-only c_src/build/libpng.so            | awk '{print $3}' | sort -u > "$WORK/c_syms.txt"
nm -D --defined-only translation/target/release/liblibpng.so | awk '{print $3}' | sort -u > "$WORK/r_syms.txt"
echo "C   defines: $(wc -l < "$WORK/c_syms.txt")"
echo "Rust defines: $(wc -l < "$WORK/r_syms.txt")"
comm -23 "$WORK/c_syms.txt" "$WORK/r_syms.txt" > "$WORK/missing_from_rust.txt"
comm -13 "$WORK/c_syms.txt" "$WORK/r_syms.txt" > "$WORK/extra_in_rust.txt"
echo "missing from Rust: $(wc -l < "$WORK/missing_from_rust.txt")"
echo "extra in Rust    : $(wc -l < "$WORK/extra_in_rust.txt")"
if [ -s "$WORK/missing_from_rust.txt" ]; then
    echo "MISSING SYMBOLS:"; cat "$WORK/missing_from_rust.txt"; exit 1
fi

echo
echo "=== 3. Rust undefined symbols (must be libc/zlib/libm only) ==="
nm -D --undefined-only translation/target/release/liblibpng.so | awk '{print $2}' | sort -u \
  | sed 's/@.*//' > "$WORK/r_undef.txt"
# Anything that looks like a libpng symbol here would be a real dangling reference.
if grep -q '^png_' "$WORK/r_undef.txt"; then
    echo "DANGLING libpng SYMBOLS:"; grep '^png_' "$WORK/r_undef.txt"; exit 1
fi
echo "non-libpng externals: $(wc -l < "$WORK/r_undef.txt") (libc / libm / zlib / unwind)"

echo
echo "=== 4. Feature combinations ==="
FEATS=$(python3 - <<'PY'
import re
s = open('translation/Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', s, re.S | re.M)
if not m:
    print('')
else:
    names = re.findall(r'^\s*([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)
    print(' '.join(n for n in names if n != 'default'))
PY
)
if [ -z "$FEATS" ]; then
    echo "translation/Cargo.toml declares NO [features] section:"
    echo "there is exactly ONE build configuration, the default one."
    echo "(The C side is likewise a single configuration: c_src/include/pnglibconf.h"
    echo " is a fixed, prebuilt config header and CMakeLists.txt defines no options.)"
    COMBOS=("default")
else
    echo "features: $FEATS"
    COMBOS=()
    # power set of the declared features
    python3 - "$FEATS" <<'PY' > "$WORK/combos.txt"
import itertools, sys
f = sys.argv[1].split()
print("--all-features")
print("--no-default-features")
for k in range(len(f) + 1):
    for c in itertools.combinations(f, k):
        print("--no-default-features --features " + ",".join(c) if c else "--no-default-features")
PY
    mapfile -t COMBOS < "$WORK/combos.txt"
fi

fail=0
for combo in "${COMBOS[@]}"; do
    if [ "$combo" = "default" ]; then
        args=()
        label="(default)"
    else
        read -r -a args <<<"$combo"
        label="$combo"
    fi
    echo "--- cargo check $label ---"
    ( cd translation && cargo check --release --offline "${args[@]}" ) >"$WORK/check.log" 2>&1 \
        || { echo "CHECK FAILED for $label"; tail -20 "$WORK/check.log"; fail=1; continue; }
    echo "--- cargo test  $label ---"
    ( cd translation && cargo build --release --offline "${args[@]}" >>"$WORK/check.log" 2>&1 \
      && cargo test --release --offline "${args[@]}" -- --test-threads=1 ) >"$WORK/test.log" 2>&1
    rc=$?
    passed=$(grep -aoE 'test result: ok\. [0-9]+ passed' "$WORK/test.log" | awk '{s+=$4} END{print s+0}')
    failed=$(grep -acE 'test result: FAILED' "$WORK/test.log")
    echo "  tests passed: $passed   binaries failing: $failed   exit: $rc"
    if [ "$rc" -ne 0 ] || [ "$failed" -ne 0 ]; then
        echo "TEST FAILURE for $label"; grep -a -A6 'DIVERGENCE\|panicked' "$WORK/test.log" | head -40; fail=1
    fi
done

echo
if [ "$fail" -eq 0 ]; then
    echo "=== ALL PHASE D CHECKS PASSED ==="
else
    echo "=== PHASE D FAILURES (see above) ==="
fi
exit $fail
