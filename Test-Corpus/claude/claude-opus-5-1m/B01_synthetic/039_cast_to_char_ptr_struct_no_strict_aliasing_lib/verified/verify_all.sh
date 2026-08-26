#!/usr/bin/env bash
# Full verification driver: builds the C .so, enumerates EVERY valid Cargo
# feature combination, then runs cargo check + the Phase B/C/D differential
# suites and the symbol-parity check for each one, in both debug and release.
set -u -o pipefail

cd "$(dirname "$0")" || exit 1
CARGO_FLAGS="--offline"
rc=0

step() { echo; echo "############ $* ############"; }

# ---------------------------------------------------------------------------
# 0. Build the C shared library
# ---------------------------------------------------------------------------
step "Building the C shared library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "!! C build failed"; exit 1; }
ls -l c_src/build/libdriver.so || exit 1

# ---------------------------------------------------------------------------
# 1. Enumerate every valid feature combination from Cargo.toml
# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
NFEAT=${#FEATURES[@]}
echo "declared non-default features: ${NFEAT}  ${FEATURES[*]:-(none)}"

# Build the list of combinations (powerset of declared features).
COMBOS=()
if (( NFEAT == 0 )); then
    # No [features] section => exactly one valid configuration: the empty set.
    COMBOS=("")
else
    for ((mask = 0; mask < (1 << NFEAT); mask++)); do
        combo=""
        for ((i = 0; i < NFEAT; i++)); do
            if (( mask & (1 << i) )); then
                combo="${combo:+$combo,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi
echo "=> ${#COMBOS[@]} feature combination(s) to verify:"
for c in "${COMBOS[@]}"; do echo "     - '${c:-<empty>}'"; done

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    step "cargo check --no-default-features --features '${combo:-<empty>}'"
    if ! timeout 600 cargo check $CARGO_FLAGS --all-targets \
            --no-default-features ${combo:+--features "$combo"} 2>&1 | tail -20; then
        echo "!! cargo check FAILED for '${combo:-<empty>}'"; rc=1
    fi
done
# The default configuration too (may differ from the empty set if defaults exist).
step "cargo check (default features)"
timeout 600 cargo check $CARGO_FLAGS --all-targets 2>&1 | tail -20 || rc=1

# ---------------------------------------------------------------------------
# 3. Phases B/C/D per combination, per profile
# ---------------------------------------------------------------------------
for profile in debug release; do
    relflag=""; [[ $profile == release ]] && relflag="--release"

    for combo in "${COMBOS[@]}"; do
        step "[$profile] cargo build+test --no-default-features --features '${combo:-<empty>}'"

        if ! timeout 600 cargo build $CARGO_FLAGS $relflag \
                --no-default-features ${combo:+--features "$combo"} 2>&1 | tail -5; then
            echo "!! build FAILED"; rc=1; continue
        fi

        out=$(timeout 600 cargo test $CARGO_FLAGS $relflag \
                --no-default-features ${combo:+--features "$combo"} 2>&1)
        echo "$out" | grep -E '^test result:|^error|FAILED'
        if echo "$out" | grep -qE 'FAILED|^error'; then
            echo "!! TESTS FAILED for '${combo:-<empty>}' [$profile]"
            echo "$out" | tail -40
            rc=1
        fi

        step "[$profile] symbol parity for '${combo:-<empty>}'"
        ./check_symbols.sh "$profile" | tail -12 || rc=1
    done

    step "[$profile] default-features test run"
    timeout 600 cargo build $CARGO_FLAGS $relflag 2>&1 | tail -3
    out=$(timeout 600 cargo test $CARGO_FLAGS $relflag 2>&1)
    echo "$out" | grep -E '^test result:|^error|FAILED'
    if echo "$out" | grep -qE 'FAILED|^error'; then
        echo "!! DEFAULT TESTS FAILED [$profile]"; echo "$out" | tail -40; rc=1
    fi
    ./check_symbols.sh "$profile" | tail -6 || rc=1
done

echo
if (( rc == 0 )); then
    echo "################ ALL CONFIGURATIONS VERIFIED ################"
else
    echo "################ FAILURES PRESENT (rc=$rc) ################"
fi
exit $rc
