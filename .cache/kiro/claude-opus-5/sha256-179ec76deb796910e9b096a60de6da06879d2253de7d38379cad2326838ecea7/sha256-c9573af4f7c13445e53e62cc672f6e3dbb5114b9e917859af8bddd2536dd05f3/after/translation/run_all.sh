#!/usr/bin/env bash
# Full verification sweep: build both libraries, diff the exported symbols, and
# run every differential test under every cargo feature combination.
#
# Usage: ./run_all.sh
set -uo pipefail

cd "$(dirname "$0")"
CRATE="$PWD"
CSRC="$CRATE/../c_src"
FAIL=0
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

step() { printf '\n=== %s ===\n' "$1"; }

# --------------------------------------------------------------------------
step "Build the C shared library"
mkdir -p "$CSRC/build"
( cd "$CSRC/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
CSO=$(find "$CSRC/build" -maxdepth 1 -name '*.so' | head -1)
echo "C .so: $CSO"

# --------------------------------------------------------------------------
step "Enumerate cargo feature combinations"
# Every declared feature; the powerset is walked below. A crate with no
# [features] section yields a single (empty) combination.
FEATURES=$(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name != 'default':
                feats.append(name)
print(' '.join(feats))
PY
)
if [ -z "$FEATURES" ]; then
    COMBOS=("default")
    echo "no [features] declared -> single default configuration"
else
    COMBOS=("default")
    # powerset of the declared features, driven with --no-default-features
    read -ra FARR <<<"$FEATURES"
    n=${#FARR[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then combo="$combo,${FARR[i]}"; fi
        done
        COMBOS+=("${combo#,}")
    done
    printf 'features: %s\n' "$FEATURES"
fi

# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    if [ "$combo" = "default" ]; then
        FLAGS=()
        LABEL="default features"
    else
        FLAGS=(--no-default-features)
        [ -n "$combo" ] && FLAGS+=(--features "$combo")
        LABEL="--no-default-features --features '${combo}'"
    fi

    step "cargo check   [$LABEL]"
    cargo check "${FLAGS[@]}" 2>&1 | tail -3 || FAIL=1

    step "cargo build --release   [$LABEL]"
    cargo build --release "${FLAGS[@]}" 2>&1 | tail -3 || FAIL=1
    RSO="$CRATE/target/release/libomni_collide_lib.so"

    step "Symbol parity   [$LABEL]"
    nm -D --defined-only "$CSO" | awk '$2=="T"||$2=="W"{print $3}' | sort >"$TMP/c_syms.txt"
    nm -D --defined-only "$RSO" | awk '$2=="T"||$2=="W"{print $3}' | sort >"$TMP/r_syms.txt"
    echo "C exports:    $(wc -l <"$TMP/c_syms.txt")"
    echo "Rust exports: $(wc -l <"$TMP/r_syms.txt")"
    MISSING=$(comm -23 "$TMP/c_syms.txt" "$TMP/r_syms.txt")
    if [ -n "$MISSING" ]; then
        echo "MISSING FROM RUST:"; echo "$MISSING"; FAIL=1
    else
        echo "missing from Rust: none"
    fi
    UNDEF=$(nm -D --undefined-only "$RSO" | awk '{print $NF}' \
            | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^__cxa_|^_Unwind_|^statx$|^gettid$')
    if [ -n "$UNDEF" ]; then
        echo "UNDEFINED NON-LIBC SYMBOLS:"; echo "$UNDEF"; FAIL=1
    else
        echo "undefined non-libc symbols: none"
    fi

    step "Differential tests   [$LABEL]"
    timeout 600 cargo test --release "${FLAGS[@]}" -- --test-threads=4 2>&1 \
        | grep -E 'running|test result|FAILED|panicked' || FAIL=1
    # cargo test's exit status is what actually matters
    timeout 600 cargo test --release "${FLAGS[@]}" >/dev/null 2>&1 || FAIL=1

    step "Same again against the debug-profile cdylib   [$LABEL]"
    cargo build "${FLAGS[@]}" 2>&1 | tail -2 || FAIL=1
    DSO="$CRATE/target/debug/libomni_collide_lib.so"
    nm -D --defined-only "$DSO" | awk '$2=="T"||$2=="W"{print $3}' | sort >"$TMP/d_syms.txt"
    echo "debug exports: $(wc -l <"$TMP/d_syms.txt")"
    MISSING=$(comm -23 "$TMP/c_syms.txt" "$TMP/d_syms.txt")
    if [ -n "$MISSING" ]; then
        echo "MISSING FROM DEBUG RUST .so:"; echo "$MISSING"; FAIL=1
    else
        echo "missing from Rust (debug): none"
    fi
    RUST_SO="$DSO" timeout 600 cargo test --release "${FLAGS[@]}" 2>&1 \
        | grep -E 'test result|FAILED' || FAIL=1
    RUST_SO="$DSO" timeout 600 cargo test --release "${FLAGS[@]}" >/dev/null 2>&1 || FAIL=1
done

step "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
    echo "ALL CHECKS PASSED"
else
    echo "FAILURES DETECTED"
fi
exit "$FAIL"
