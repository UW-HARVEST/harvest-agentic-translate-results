#!/bin/bash
# Phase D: run the whole verification under EVERY feature combination.
#
# The crate declares exactly one optional feature (`debug-stats`) and no default
# features, so the complete power set is {} and {debug-stats}.  The combinations
# are extracted from Cargo.toml rather than hard-coded.
set -uo pipefail
cd "$(dirname "$0")/../translation"

C_SO="$(cd .. && pwd)/c_src/build/liblong.so"
export LONG_C_SO="$C_SO"

# --- enumerate the feature power set from Cargo.toml -------------------------
mapfile -t FEATS < <(awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+ *=/ { sub(/ *=.*/, ""); print }
' Cargo.toml)

combos=("")
for f in "${FEATS[@]}"; do
    new=()
    for c in "${combos[@]}"; do
        new+=("$c")
        if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    combos=("${new[@]}")
done

echo "features declared : ${FEATS[*]:-<none>}"
echo "combinations      : ${#combos[@]}"
echo

fail=0
for combo in "${combos[@]}"; do
    if [ -z "$combo" ]; then
        label="<no features>"
        args=(--no-default-features)
    else
        label="$combo"
        args=(--no-default-features --features "$combo")
    fi
    echo "==================================================================="
    echo "### feature combination: $label"
    echo "==================================================================="

    # separate target dir per combo so the cdylib under test always matches
    export CARGO_TARGET_DIR="target/fc-$(echo "${combo:-none}" | tr ',' '_')"

    echo "--- cargo check ---"
    cargo check "${args[@]}" 2>&1 | tail -3 || fail=1

    echo "--- cargo build --release (cdylib under test) ---"
    cargo build --release "${args[@]}" 2>&1 | tail -3 || fail=1

    RUST_SO="$CARGO_TARGET_DIR/release/liblong.so"
    export LONG_RUST_SO="$(cd "$(dirname "$RUST_SO")" && pwd)/liblong.so"

    echo "--- nm -D symbol parity ---"
    missing=$(comm -23 \
        <(nm -D --defined-only "$C_SO"        | awk '{print $NF}' | sort) \
        <(nm -D --defined-only "$LONG_RUST_SO" | awk '{print $NF}' | sort))
    if [ -n "$missing" ]; then
        echo "MISSING SYMBOLS: $missing"; fail=1
    else
        echo "0 missing symbols"
    fi
    csz=$(nm -SD --defined-only "$C_SO"        | awk '$NF=="array"{print $2}')
    rsz=$(nm -SD --defined-only "$LONG_RUST_SO" | awk '$NF=="array"{print $2}')
    if [ "$csz" != "$rsz" ]; then
        echo "array size mismatch: C=$csz Rust=$rsz"; fail=1
    else
        echo "array size matches ($csz)"
    fi

    echo "--- cargo test --release ---"
    timeout 900 cargo test --release "${args[@]}" -- --test-threads=1 2>&1 \
        | grep -E '^(test |running |test result|error|warning: unused|failures)' \
        | grep -vE '^test .* ok$' || fail=1

    echo
done

echo "==================================================================="
if [ "$fail" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
