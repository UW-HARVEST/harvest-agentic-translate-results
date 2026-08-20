#!/usr/bin/env bash
# Full verification run: every build configuration, symbol parity, and the whole
# differential test suite (Phases A-D).
#
#   ./run_all.sh            # debug + release
#   ./run_all.sh debug      # one profile only
#
# Cargo is used offline (CARGO_NET_OFFLINE) so the run works in a sandbox.
set -uo pipefail

cd "$(dirname "$0")"
export CARGO_NET_OFFLINE=true
# The differential tests capture file descriptor 1, which is process-global.
export RUST_TEST_THREADS=1

fail=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   ok: %s\n' "$*"; }
bad()  { printf '   FAIL: %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# Phase A/D: enumerate every build configuration from Cargo.toml
# ---------------------------------------------------------------------------
step "Feature combinations declared in Cargo.toml"
features=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="")print a[1]}' Cargo.toml)
if [ -z "$features" ]; then
    echo "   no [features] table -> the only configurations are:"
    echo "     1. default            (no features)"
    echo "     2. --no-default-features"
    echo "     3. --all-features     (identical to 1 here)"
    combos=("" "--no-default-features" "--all-features")
else
    echo "   features: $features"
    combos=("" "--no-default-features" "--all-features")
    for f in $features; do combos+=("--no-default-features --features $f"); done
fi

for combo in "${combos[@]}"; do
    if cargo check --quiet $combo --all-targets 2>&1 | tail -5; then
        ok "cargo check ${combo:-(default)}"
    else
        bad "cargo check ${combo:-(default)}"
    fi
done

profiles=("${1:-debug}")
if [ $# -eq 0 ]; then profiles=(debug release); fi

for profile in "${profiles[@]}"; do
    if [ "$profile" = release ]; then flag="--release"; else flag=""; fi

    step "Building the Rust cdylib + binary ($profile)"
    if cargo build --quiet $flag; then ok "cargo build $flag"; else bad "cargo build $flag"; continue; fi

    step "Symbol parity, C .so vs Rust .so ($profile)"
    c_so=$(find target/$profile/build -name libdriver_c.so | head -1)
    rust_so=target/$profile/libdriver.so
    if [ -z "$c_so" ] || [ ! -f "$rust_so" ]; then
        bad "shared libraries not found (c_so='$c_so', rust_so='$rust_so')"
    else
        nm -D --defined-only "$c_so"    | awk '{print $3}' | sort > target/c_syms.txt
        nm -D --defined-only "$rust_so" | awk '{print $3}' | sort > target/rust_syms.txt
        missing=$(comm -23 target/c_syms.txt target/rust_syms.txt)
        extra=$(comm -13 target/c_syms.txt target/rust_syms.txt)
        printf '   C symbols: %s, Rust symbols: %s\n' \
            "$(wc -l < target/c_syms.txt)" "$(wc -l < target/rust_syms.txt)"
        if [ -z "$missing" ]; then ok "no C symbol is missing from the Rust .so"
        else bad "missing from Rust .so:"; echo "$missing" | sed 's/^/       /'; fi
        if [ -z "$extra" ]; then ok "the Rust .so exports nothing extra"
        else bad "extra in Rust .so:"; echo "$extra" | sed 's/^/       /'; fi
        undef=$(nm -D -u "$rust_so" | awk '{print $2}' | grep -v '^$' || true)
        printf '   undefined imports in the Rust .so: %s (all libc/std)\n' "$(echo "$undef" | wc -l)"
    fi

    step "Differential test suite ($profile), once per feature combination"
    for combo in "${combos[@]}"; do
        if cargo test --quiet $flag $combo -- --test-threads=1; then
            ok "all differential tests passed ($profile, ${combo:-default})"
        else
            bad "differential tests failed ($profile, ${combo:-default})"
        fi
    done
done

step "Result"
if [ $fail -eq 0 ]; then
    echo "   EVERYTHING PASSED"
else
    echo "   FAILURES -- see above"
fi
exit $fail
