#!/usr/bin/env bash
# Full differential verification run.
#
#   scripts/run_all.sh            # symbol parity + Phase B + Phase C, both profiles
#   scripts/run_all.sh e2e        # …plus the ~13 min end-to-end long_exec rows
#
# `Cargo.toml` has no `[features]` table, so `--no-default-features` is the one
# and only valid feature combination; it is used explicitly everywhere below.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)

echo "== building the C shared object =="
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
ls -l c_src/build/liblong.so

echo "== enumerating feature combinations =="
# Every combination in Cargo.toml's [features] table (there is none => just the
# empty combination).
COMBOS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml || true)
if [ -z "${COMBOS}" ]; then
    echo "no [features] table -> single combination: --no-default-features"
else
    echo "features found: ${COMBOS}"
fi

echo "== cargo check (every feature combination) =="
cargo check --offline --no-default-features --all-targets
for f in ${COMBOS}; do
    cargo check --offline --no-default-features --features "$f" --all-targets
done

echo "== building both Rust profiles (row C18 compares them) =="
cargo build --offline --no-default-features
cargo build --offline --no-default-features --release

for profile in debug release; do
    echo "== test suite, profile=${profile} =="
    if [ "$profile" = release ]; then
        REL=--release
    else
        REL=
    fi
    cargo test --offline --no-default-features $REL -- --test-threads=1
done

if [ "${1:-}" = e2e ]; then
    for seed in 42 0 4294967295; do
        echo "== end-to-end long_exec(${seed}) =="
        LONG_E2E_SEED=$seed cargo test --offline --no-default-features --release \
            --test phase_e2e -- --ignored --exact e2e_c --nocapture --test-threads=1
        LONG_E2E_SEED=$seed cargo test --offline --no-default-features --release \
            --test phase_e2e -- --ignored --exact e2e_rust --nocapture --test-threads=1
        LONG_E2E_SEED=$seed cargo test --offline --no-default-features --release \
            --test phase_e2e -- --ignored --exact e2e_compare --nocapture --test-threads=1
    done
fi

echo "ALL DONE (${ROOT})"
