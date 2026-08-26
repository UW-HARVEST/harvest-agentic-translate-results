#!/usr/bin/env bash
# Phase D driver: enumerate every valid feature combination from Cargo.toml and
# run `cargo check` + the full differential suite for each, in both the dev and
# the release profile (the two configurations in which the cdylib can be built).
set -u
cd "$(dirname "$0")" || exit 1
fails=0

# ---- 1. enumerate feature combinations (powerset of [features], minus "default")
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, re, sys
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name != 'default':
                feats.append(name)
print("<none>" if not feats else "", end="")
if feats:
    for r in range(len(feats) + 1):
        for c in itertools.combinations(feats, r):
            print(",".join(c) if c else "<none>")
else:
    print()
PY
)
COMBOS=("${COMBOS[@]/#/}")
echo "=== feature combinations found: ${#COMBOS[@]} -> ${COMBOS[*]}"

# ---- 2. build the C reference shared object
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
echo "=== C .so: $(ls -l c_src/build/libtranslated_rust.so | awk '{print $5" bytes"}')"

for combo in "${COMBOS[@]}"; do
    if [ "$combo" = "<none>" ] || [ -z "$combo" ]; then
        FEATFLAGS=(--no-default-features)
        label="no-default-features"
    else
        FEATFLAGS=(--no-default-features --features "$combo")
        label="$combo"
    fi

    echo
    echo "############ combo: $label"
    if ! timeout 600 cargo check "${FEATFLAGS[@]}" >/dev/null 2>&1; then
        echo "  cargo check FAILED"; fails=$((fails+1)); continue
    fi
    echo "  cargo check ok"

    for profile in dev release; do
        if [ "$profile" = release ]; then
            BUILDFLAGS=(--release); TESTFLAGS=(--release); dir=release
        else
            BUILDFLAGS=(); TESTFLAGS=(); dir=debug
        fi
        if ! timeout 600 cargo build "${FEATFLAGS[@]}" "${BUILDFLAGS[@]}" >/dev/null 2>&1; then
            echo "  [$profile] cargo build FAILED"; fails=$((fails+1)); continue
        fi
        so="target/$dir/libbin2hex_lib.so"
        [ -f "$so" ] || { echo "  [$profile] missing $so"; fails=$((fails+1)); continue; }
        # nm parity for this profile
        missing=$(comm -23 \
            <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort -u) \
            <(nm -D --defined-only "$so" | awk '{print $NF}' | sort -u))
        if [ -n "$missing" ]; then
            echo "  [$profile] MISSING SYMBOLS: $missing"; fails=$((fails+1))
        fi
        out=$(RUST_CDYLIB="$PWD/$so" timeout 600 cargo test "${FEATFLAGS[@]}" "${TESTFLAGS[@]}" 2>&1)
        if [ $? -ne 0 ]; then
            echo "  [$profile] TESTS FAILED"; echo "$out" | grep -E "^test .*FAILED|panicked|test result" | head -20
            fails=$((fails+1))
        else
            echo "  [$profile] tests passed: $(echo "$out" | grep -c '^test .* ok$') cases"
        fi
    done

    # also exercise the harness' rustc fallback path (no cargo artifact at all)
    rm -rf target/test-cdylib
    hidden=$(mktemp -d "${TMPDIR:-/tmp}/hide.XXXXXX")
    mv target/debug/libbin2hex_lib.so "$hidden/" 2>/dev/null
    mv target/release/libbin2hex_lib.so "$hidden/" 2>/dev/null
    if timeout 600 cargo test "${FEATFLAGS[@]}" >/dev/null 2>&1; then
        echo "  [rustc-fallback] tests passed"
    else
        echo "  [rustc-fallback] TESTS FAILED"; fails=$((fails+1))
    fi
    mv "$hidden"/libbin2hex_lib.so target/debug/ 2>/dev/null
    rmdir "$hidden" 2>/dev/null
done

echo
if [ "$fails" -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS AND PROFILES PASSED"
else
    echo "FAILURES: $fails"
fi
exit "$fails"
