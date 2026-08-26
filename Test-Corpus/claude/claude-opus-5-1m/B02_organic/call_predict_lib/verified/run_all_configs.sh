#!/usr/bin/env bash
# Runs the differential suite for EVERY build-time configuration.
#
# Feature combinations are extracted mechanically from Cargo.toml: the file has
# no [features] section, so the only valid combination is the empty one, which
# is exercised three ways (default, --no-default-features, --all-features) and
# in both the debug and the release profile.
set -uo pipefail
cd "$(dirname "$0")"

echo "=== features declared in Cargo.toml ==="
awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {print $1}' Cargo.toml | tee /dev/stderr > "${TMPDIR:-/tmp}/features.txt"
NFEAT=$(wc -l < "${TMPDIR:-/tmp}/features.txt")
echo "feature count: $NFEAT  ->  $((1 << NFEAT)) combination(s)"
echo

# enumerate the power set of the declared features (empty set when NFEAT == 0)
mapfile -t FEATS < "${TMPDIR:-/tmp}/features.txt"
COMBOS=("")
for ((i = 0; i < NFEAT; i++)); do
    NEW=()
    for c in "${COMBOS[@]}"; do
        NEW+=("$c")
        if [ -z "$c" ]; then NEW+=("${FEATS[$i]}"); else NEW+=("$c,${FEATS[$i]}"); fi
    done
    COMBOS=("${NEW[@]}")
done

FAIL=0
for combo in "${COMBOS[@]}"; do
    for prof in "" "--release"; do
        label="features='${combo}' ${prof:-debug}"
        echo "--------------------------------------------------------------"
        echo ">>> cargo check --no-default-features --features '${combo}' ${prof}"
        if ! timeout 600 cargo check --offline --tests --no-default-features --features "${combo}" ${prof} 2>&1 | tail -3; then
            echo "CHECK FAILED: ${label}"; FAIL=1
        fi
        # rebuild the shipped cdylib for this configuration, then test against it
        timeout 600 cargo build --offline --no-default-features --features "${combo}" ${prof} 2>&1 | tail -2
        echo ">>> cargo test  --no-default-features --features '${combo}' ${prof}"
        if ! timeout 600 cargo test --offline --no-default-features --features "${combo}" ${prof} 2>&1 | tail -4; then
            echo "TEST FAILED: ${label}"; FAIL=1
        fi
    done
done

echo "--------------------------------------------------------------"
echo ">>> default features, both profiles"
for prof in "" "--release"; do
    timeout 600 cargo build --offline ${prof} 2>&1 | tail -1
    if ! timeout 600 cargo test --offline ${prof} 2>&1 | tail -4; then
        echo "TEST FAILED: default ${prof:-debug}"; FAIL=1
    fi
done
echo ">>> --all-features, release"
timeout 600 cargo build --offline --all-features --release 2>&1 | tail -1
if ! timeout 600 cargo test --offline --all-features --release 2>&1 | tail -4; then
    echo "TEST FAILED: --all-features"; FAIL=1
fi

echo "=============================================================="
echo ">>> symbol parity (C .so vs Rust .so)"
C_SO=c_src/build/libtranslated_rust.so
R_SO=target/release/libcall_predict_lib.so
diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort) \
    && echo "SYMBOL PARITY: identical" || { echo "SYMBOL PARITY: DIFFERENT"; FAIL=1; }

echo
if [ "$FAIL" = 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $FAIL
