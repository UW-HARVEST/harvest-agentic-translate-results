#!/usr/bin/env bash
# Full verification run: Phase A artifacts are checked in, this script executes
# Phases B, C and D for EVERY feature combination.
#
# Feature combinations are derived from Cargo.toml's [features] section; the
# crate declares only `default = []`, so the enumeration is:
#     1) --no-default-features            (empty feature set)
#     2) <default>                        (identical to 1, run anyway)
# The loop is mechanical, so adding a feature to Cargo.toml automatically adds
# combinations here.
set -uo pipefail

cd "$(dirname "$0")"
export CARGO_NET_OFFLINE=true

# ---- enumerate feature combinations (powerset of the optional features) ------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { f=1; next }
    /^\[/           { f=0 }
    f && /^[A-Za-z_][A-Za-z0-9_.-]*[ \t]*=/ {
        split($0, a, "=");
        gsub(/[ \t]/, "", a[1]);
        if (a[1] != "default") print a[1];
    }' Cargo.toml
)
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
    [ -z "$f" ] && continue
    new=()
    for c in "${COMBOS[@]}"; do
        new+=("$c")
        if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    COMBOS=("${new[@]}")
done

echo "############################################################"
echo "# feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "#   --no-default-features --features '$c'"; done
echo "#   (plus the default build)"
echo "############################################################"

FAIL=0

run() {
    echo
    echo ">>> $*"
    if ! timeout 600 "$@"; then
        echo "!!! FAILED: $*"
        FAIL=1
    fi
}

# ---- Phase A/D: C reference build ------------------------------------------
mkdir -p c_build
run cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
run cmake --build c_src/build
run bash -c '"${CC:-cc}" -shared -fPIC -O2 -o c_build/libdriver_c.so c_src/src/main.c'

# ---- per-combination: check, build, symbol parity, Phases B & C -------------
for combo in "${COMBOS[@]}"; do
    echo
    echo "============================================================"
    echo "== FEATURES: '${combo:-<none>}' (--no-default-features)"
    echo "============================================================"
    run cargo check --no-default-features --features "$combo"
    run cargo build --no-default-features --features "$combo"
    run cargo build --release --no-default-features --features "$combo"
    run ./check_symbols.sh debug
    run ./check_symbols.sh release
    run cargo test --no-default-features --features "$combo"
    run cargo test --release --no-default-features --features "$combo"
done

echo
echo "============================================================"
echo "== FEATURES: default"
echo "============================================================"
run cargo check
run cargo build
run cargo build --release
run ./check_symbols.sh debug
run ./check_symbols.sh release
run cargo test
run cargo test --release

# extra end-to-end sanity: raw byte diff of the two executables' stdout
echo
echo ">>> byte-diff of executable stdout (C vs Rust)"
if diff <(c_src/build/driver) <(target/release/driver); then
    echo "identical"
else
    echo "!!! FAILED: executable stdout differs"
    FAIL=1
fi

echo
if [ $FAIL -eq 0 ]; then
    echo "########## ALL PHASES PASSED FOR ALL FEATURE COMBINATIONS ##########"
else
    echo "########## FAILURES DETECTED ##########"
fi
exit $FAIL
