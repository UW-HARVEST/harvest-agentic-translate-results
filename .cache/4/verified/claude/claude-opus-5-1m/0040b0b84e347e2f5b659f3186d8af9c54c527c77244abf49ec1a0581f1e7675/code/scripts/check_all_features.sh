#!/usr/bin/env bash
# Enumerates EVERY valid Cargo feature combination (the power set of the
# `[features]` table, plus the empty set) and runs `cargo check`, `cargo build`
# and the full differential suite for each, in both the dev and release
# profiles.
#
# usage: scripts/check_all_features.sh [--no-release]
set -uo pipefail
cd "$(dirname "$0")/.."

# --- enumerate features from Cargo.toml -------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re
text = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        names.append(line.split('=')[0].strip().strip('"'))
for n in names:
    if n != "default":
        print(n)
PY
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- power set ---------------------------------------------------------------
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
    [ -z "$f" ] && continue
    new=()
    for c in "${COMBOS[@]}"; do
        if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    COMBOS+=("${new[@]}")
done

echo "feature combinations to verify: ${#COMBOS[@]}"

PROFILES=(dev release)
[ "${1:-}" = "--no-release" ] && PROFILES=(dev)

fail=0
for combo in "${COMBOS[@]}"; do
    label="${combo:-<empty/default>}"
    for profile in "${PROFILES[@]}"; do
        relflag=""
        [ "$profile" = "release" ] && relflag="--release"
        featflag=(--no-default-features)
        [ -n "$combo" ] && featflag=(--no-default-features --features "$combo")

        echo "=============================================================="
        echo ">>> features: $label   profile: $profile"
        for step in check build; do
            if ! timeout 590 cargo "$step" --offline $relflag "${featflag[@]}" \
                    --all-targets 2>&1 | tail -3; then
                echo "!!! cargo $step FAILED (features=$label profile=$profile)"
                fail=1
            fi
        done
        if ! timeout 590 cargo test --offline $relflag "${featflag[@]}" 2>&1 \
                | grep -E "test result|in-process differential|error\[|FAILED"; then
            echo "!!! cargo test produced no result line (features=$label profile=$profile)"
            fail=1
        fi
        if timeout 590 cargo test --offline $relflag "${featflag[@]}" 2>&1 \
                | grep -qE "FAILED|error\["; then
            echo "!!! cargo test FAILED (features=$label profile=$profile)"
            fail=1
        fi
    done
done

# also the plain default invocation (identical to the empty set here, but check
# it explicitly) and --all-features
echo "=============================================================="
echo ">>> default features"
timeout 590 cargo test --offline 2>&1 | grep -E "test result|in-process differential" || fail=1
echo ">>> --all-features"
timeout 590 cargo test --offline --all-features 2>&1 | grep -E "test result|in-process differential" || fail=1

if [ "$fail" -ne 0 ]; then
    echo "FEATURE MATRIX: FAILURES PRESENT"
    exit 1
fi
echo "FEATURE MATRIX: all combinations pass"
