#!/usr/bin/env bash
# Enumerate EVERY feature combination declared in Cargo.toml (the power set of
# [features], plus the no-default-features and default builds) and run
# `cargo check`, relink the cdylib and run the whole differential suite for each.
#
# Usage: scripts/check_all_features.sh [check|test]      (default: test)
set -uo pipefail
cd "$(dirname "$0")/.."
MODE="${1:-test}"

# --- build the C reference library -------------------------------------------
cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null || exit 1
cmake --build c_src/build >/dev/null || exit 1

# --- enumerate the feature power set -----------------------------------------
mapfile -t FEATS < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                print(n)
PY
)
N=${#FEATS[@]}
echo "== declared features (${N}): ${FEATS[*]:-<none>}"

COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
        if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATS[$i]}"; fi
    done
    COMBOS+=("$combo")
done
# the plain (default-features) build is a configuration of its own
COMBOS+=("<default>")

FAIL=0
for combo in "${COMBOS[@]}"; do
    if [ "$combo" = "<default>" ]; then
        ARGS=()
        label="default features"
    else
        ARGS=(--no-default-features)
        [ -n "$combo" ] && ARGS+=(--features "$combo")
        label="--no-default-features${combo:+ --features $combo}"
    fi
    echo
    echo "======================================================================"
    echo "== $label"
    echo "======================================================================"
    cargo check --offline "${ARGS[@]}" 2>&1 | tail -3 || FAIL=1
    if [ "$MODE" = "test" ]; then
        # `cargo test` does not relink the cdylib, so build it explicitly first
        cargo build --offline "${ARGS[@]}" 2>&1 | tail -2 || FAIL=1
        cargo test --offline "${ARGS[@]}" -- --test-threads=1 2>&1 \
            | grep -E '^(test result|running|error|warning: unused|thread .* panicked)' || FAIL=1
        cargo test --offline "${ARGS[@]}" -- --test-threads=1 >/dev/null 2>&1 || FAIL=1
    fi
done

echo
if [ "$FAIL" = 0 ]; then echo "ALL FEATURE COMBINATIONS OK"; else echo "FAILURES (see above)"; fi
exit $FAIL
