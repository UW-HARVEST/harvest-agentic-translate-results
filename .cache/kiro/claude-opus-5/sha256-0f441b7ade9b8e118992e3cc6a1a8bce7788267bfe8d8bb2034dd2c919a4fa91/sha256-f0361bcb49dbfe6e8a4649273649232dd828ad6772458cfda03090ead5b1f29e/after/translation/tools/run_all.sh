#!/usr/bin/env bash
# Run the whole differential suite under EVERY cargo feature combination.
#
# Feature combinations are extracted from Cargo.toml's [features] section rather
# than hard-coded, so a newly added feature is picked up automatically.
#
# Usage:  tools/run_all.sh            # fast layers only (~10 min)
#         tools/run_all.sh --slow     # also the #[ignore]d live 2000-iteration
#                                     # C-vs-Rust runs (~8 min per seed)
set -uo pipefail

crate="$(cd "$(dirname "$0")/.." && pwd)"
root="$(dirname "$crate")"
cd "$crate"

slow=0
[ "${1:-}" = "--slow" ] && slow=1

# --- build the C shared object (ground truth) ------------------------------
if [ ! -f "$root/c_src/build/liblong.so" ]; then
    ( cd "$root/c_src" && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null )
fi

# --- enumerate feature combinations ---------------------------------------
mapfile -t feats < <(
    awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{
        sub(/[[:space:]]*=.*/,""); print }' Cargo.toml
)

combos=("--no-default-features" "")
n=${#feats[@]}
if [ "$n" -gt 0 ]; then
    # every non-empty subset of the declared features, on top of the defaults
    for ((mask=1; mask<(1<<n); mask++)); do
        sel=""
        for ((i=0; i<n; i++)); do
            (( mask & (1<<i) )) && sel="${sel:+$sel,}${feats[$i]}"
        done
        combos+=("--features $sel")
        combos+=("--no-default-features --features $sel")
    done
fi

echo "feature combinations to verify: ${#combos[@]}"
for c in "${combos[@]}"; do echo "  cargo test --release ${c:-<default>}"; done

rc=0
for c in "${combos[@]}"; do
    echo
    echo "=================================================================="
    echo "== FEATURES: ${c:-<default>}"
    echo "=================================================================="
    # shellcheck disable=SC2086
    if ! cargo build --release $c; then rc=1; echo "BUILD FAILED: $c"; continue; fi
    # shellcheck disable=SC2086
    if ! cargo test  --release $c --no-run >/dev/null; then rc=1; echo "TESTBUILD FAILED: $c"; continue; fi

    # symbol parity must hold for every feature combination
    # shellcheck disable=SC2086
    cargo test --release $c --test symbols        -- --test-threads=1 || rc=1
    # Phase B (low-level entry points, may run in parallel: no stdout capture)
    # shellcheck disable=SC2086
    cargo test --release $c --test kernel_diff                        || rc=1
    # Phase C (captures fd 1 -> must be single threaded)
    # shellcheck disable=SC2086
    cargo test --release $c --test errors         -- --test-threads=1 || rc=1
    # Phase B full pipeline (captures fd 1)
    # shellcheck disable=SC2086
    cargo test --release $c --test long_exec_diff -- --test-threads=1 || rc=1
    # in-crate unit tests (fast accelerator vs naive iteration)
    # shellcheck disable=SC2086
    cargo test --release $c --lib                                     || rc=1

    if [ "$slow" = 1 ]; then
        # shellcheck disable=SC2086
        LONG_SEEDS="${LONG_SEEDS:-7}" cargo test --release $c --test long_exec_diff \
            -- --ignored --test-threads=1 || rc=1
    fi
done

echo
if [ "$rc" = 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES (rc=$rc)"; fi
exit "$rc"
