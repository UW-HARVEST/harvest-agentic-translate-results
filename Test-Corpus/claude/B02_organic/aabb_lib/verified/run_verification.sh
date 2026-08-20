#!/usr/bin/env bash
# Full verification driver (Phases A-D).
#
#  1. builds the C reference .so
#  2. enumerates EVERY valid cargo feature combination (the power set of the
#     [features] table) and runs `cargo check` + the whole differential test
#     suite for each one
#  3. re-checks exported-symbol parity for each one
set -uo pipefail
cd "$(dirname "$0")"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

note "1. building the C reference shared library"
mkdir -p c_src/build
(cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
ls -l c_src/build/libtranslated_rust.so

note "2. enumerating feature combinations"
# Every name in the [features] table (empty for this crate).
feats=$(awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
' Cargo.toml)
nfeat=$(printf '%s' "$feats" | grep -c . || true)
echo "features declared in Cargo.toml: ${nfeat:-0} [${feats//$'\n'/, }]"

# Build the power set of $feats as a list of comma-separated strings.
combos=("")
for f in $feats; do
    new=()
    for c in "${combos[@]}"; do
        new+=("$c")
        if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    combos=("${new[@]}")
done
echo "=> ${#combos[@]} combination(s) to verify"

run() { # run <label> <cargo-args...>
    local label=$1; shift
    note "cargo check  [$label]"
    if ! timeout 600 cargo check --offline --tests "$@" 2>&1 | tail -5; then
        echo "CHECK FAILED [$label]"; fail=1; return
    fi
    note "cargo build  [$label]"
    timeout 600 cargo build --offline "$@" >/dev/null 2>&1 || {
        echo "BUILD FAILED [$label]"; fail=1; return; }
    note "symbol parity [$label]"
    ./check_symbols.sh || fail=1
    note "cargo test   [$label]"
    local log
    log=$(mktemp)
    if timeout 600 cargo test --offline "$@" >"$log" 2>&1; then
        grep -E '^(     Running|test result)' "$log" | sed 's/^ *//'
    else
        echo "TESTS FAILED [$label]"; tail -40 "$log"; fail=1
    fi
    rm -f "$log"
}

for c in "${combos[@]}"; do
    if [ -z "$c" ]; then
        run "no-default-features" --no-default-features
    else
        run "no-default-features + $c" --no-default-features --features "$c"
    fi
done

# The default and all-features builds too (identical here, but checked anyway).
run "default"
run "all-features" --all-features

note "RESULT"
if [ "$fail" = 0 ]; then
    echo "ALL CONFIGURATIONS VERIFIED"
else
    echo "FAILURES DETECTED"
fi
exit $fail
