#!/bin/sh
# Phase D: run the whole differential suite under EVERY feature combination
# declared by Cargo.toml.
#
# The crate declares no `[features]` section and has no optional dependencies,
# so the only configurations are the default one and `--no-default-features`
# (which is identical here).  The script derives the list mechanically so that
# it keeps working if features are ever added.
set -e
cd "$(dirname "$0")"

# --- mechanically extract the feature names from Cargo.toml ---------------
features=$(awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "" && a[1] !~ /^#/) print a[1] }
' Cargo.toml)

echo "declared features: ${features:-<none>}"

# --- build both libraries -------------------------------------------------
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )

run_combo() {
    label="$1"; shift
    echo
    echo "==================== configuration: $label ===================="
    cargo build --release --offline "$@" >/dev/null
    ./check_symbols.sh
    cargo test --offline "$@" -- --test-threads=4
}

# Always cover the default build and the empty feature set.
run_combo "<default>"
run_combo "--no-default-features" --no-default-features

# Then the powerset of the declared features (if any).
if [ -n "$features" ]; then
    set -- $features
    n=$#
    total=$((1 << n))
    i=1
    while [ "$i" -lt "$total" ]; do
        combo=""
        j=0
        for f in $features; do
            if [ $(( (i >> j) & 1 )) -eq 1 ]; then
                combo="$combo,$f"
            fi
            j=$((j + 1))
        done
        combo=${combo#,}
        run_combo "--no-default-features --features $combo" \
                  --no-default-features --features "$combo"
        i=$((i + 1))
    done
fi

# ------------------------------------------------------------------
# Extra configuration: the Rust cdylib built in DEBUG.
#
# The shipped artifact is the release `.so`, but the debug profile turns on
# Rust's integer-overflow checks and `debug_assert!`s, which is a different
# code path through the same source.  A translation that only happens to be
# correct because release mode wraps silently would fail here.
# ------------------------------------------------------------------
echo
echo "==================== configuration: debug cdylib ===================="
cargo build --offline >/dev/null
dbg="$(pwd)/target/debug/libpinflate_lib.so"
if [ -f "$dbg" ]; then
    nm -D --defined-only "$dbg" | awk '{print $3}' | sort -u > "${TMPDIR:-/tmp}/dbg_syms"
    nm -D --defined-only ../c_src/build/lib*.so | awk '{print $3}' | sort -u > "${TMPDIR:-/tmp}/c_syms2"
    if [ -n "$(comm -23 "${TMPDIR:-/tmp}/c_syms2" "${TMPDIR:-/tmp}/dbg_syms")" ]; then
        echo "MISSING from debug cdylib:"
        comm -23 "${TMPDIR:-/tmp}/c_syms2" "${TMPDIR:-/tmp}/dbg_syms"
        exit 1
    fi
    echo "symbol diff (C -> Rust debug cdylib): EMPTY  ✓"
    PINFLATE_RUST_SO="$dbg" PINFLATE_SKIP_NULL_OUT=1 cargo test --offline -- --test-threads=4
else
    echo "debug cdylib not produced at $dbg -- skipping"
    exit 1
fi

echo
echo "ALL CONFIGURATIONS PASSED"
