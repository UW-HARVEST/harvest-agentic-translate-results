#!/usr/bin/env bash
# Full-scale differential run of `long_exec`.
#
# One `long_exec` call is ~470 s against the (unoptimised) C library and ~310 s
# against the Rust one, so the two sides are recorded by separate cargo
# invocations and compared afterwards. Each invocation stays well under a
# 600 s budget.
#
# Usage: ./run_long_exec_difftest.sh [seed ...]     (default seed: 42)
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

# Make sure both libraries exist before spending minutes on a run.
root="$(dirname "$here")"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null )
cargo build --release >/dev/null

seeds=("$@")
if [[ ${#seeds[@]} -eq 0 ]]; then
    seeds=(42)
fi

for seed in "${seeds[@]}"; do
    echo "=== seed $seed: recording C (~470 s) ==="
    DIFFTEST_SEED="$seed" timeout 600 cargo test --release --test long_exec \
        -- --ignored --exact --nocapture record_c

    echo "=== seed $seed: recording Rust (~310 s) ==="
    DIFFTEST_SEED="$seed" timeout 600 cargo test --release --test long_exec \
        -- --ignored --exact --nocapture record_rust

    echo "=== seed $seed: comparing ==="
    DIFFTEST_SEED="$seed" timeout 600 cargo test --release --test long_exec \
        -- --ignored --exact --nocapture compare_recordings
done

echo "long_exec differential run complete for seeds: ${seeds[*]}"
