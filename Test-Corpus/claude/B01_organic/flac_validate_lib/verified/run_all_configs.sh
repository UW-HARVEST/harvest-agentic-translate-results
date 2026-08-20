#!/usr/bin/env bash
# Phase D driver: enumerate every valid feature combination from Cargo.toml and
# run `cargo check` + the full differential suite (Phases B and C) for each.
set -uo pipefail
cd "$(dirname "$0")"

# ---- 1. Enumerate feature combinations straight out of Cargo.toml -----------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(?=^\[|\Z)', txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name and name != 'default':
                feats.append(name)
print('\n'.join(feats))
PY
)
# mapfile yields a single empty element when the generator printed nothing.
if [ "${#FEATURES[@]}" -eq 1 ] && [ -z "${FEATURES[0]}" ]; then
    FEATURES=()
fi

N=${#FEATURES[@]}
echo "Cargo.toml declares $N optional feature(s): ${FEATURES[*]:-<none>}"

COMBOS=()
if [ "$N" -eq 0 ]; then
    # No features at all: the two build configurations that exist are the
    # default one and the (identical) --no-default-features one.
    COMBOS+=("")
else
    # Full power set.
    for ((mask = 0; mask < (1 << N); mask++)); do
        combo=""
        for ((b = 0; b < N; b++)); do
            if (((mask >> b) & 1)); then
                combo="${combo:+$combo,}${FEATURES[$b]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi

echo "=> ${#COMBOS[@]} feature combination(s) to verify"
echo

ITERS="${HARVEST_ITERS:-}"
fail=0

run() { # label, extra cargo args...
    local label="$1"; shift
    echo "--- $label"
    if ! timeout 600 "$@" >"${TMPDIR:-/tmp}/cfg.log" 2>&1; then
        echo "    FAILED: $*"
        tail -n 40 "${TMPDIR:-/tmp}/cfg.log"
        fail=1
        return 1
    fi
    echo "    ok"
}

for combo in "${COMBOS[@]}"; do
    for defaults in "--no-default-features" ""; do
        if [ -n "$combo" ]; then
            featargs=(--features "$combo")
        else
            featargs=()
        fi
        label="features='${combo:-<none>}' ${defaults:-<with-defaults>}"

        # Phase A step 2: every combination must compile cleanly.
        # shellcheck disable=SC2086
        run "cargo check   [$label]" cargo check $defaults "${featargs[@]}" || continue

        # The cdylib artifacts the differential harness dlopen()s.
        # shellcheck disable=SC2086
        run "cargo build   [$label] (release)" cargo build --release $defaults "${featargs[@]}" || continue
        # shellcheck disable=SC2086
        run "cargo build   [$label] (debug)"   cargo build $defaults "${featargs[@]}" || continue

        # Phases B + C.
        # shellcheck disable=SC2086
        HARVEST_ITERS="$ITERS" run "cargo test    [$label]" cargo test $defaults "${featargs[@]}" || continue
    done
done

echo
if [ "$fail" -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS PASSED"
else
    echo "SOME FEATURE COMBINATIONS FAILED"
    exit 1
fi
