#!/usr/bin/env bash
# Full verification sweep: enumerate every Cargo feature combination, run
# `cargo check` and the whole differential test suite for each, and finish with
# the nm-based symbol diff.
set -uo pipefail
cd "$(dirname "$0")/.."

# ---------------------------------------------------------------------------
# 1. enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        k = line.split('=')[0].strip()
        if k and k != 'default':
            feats.append(k)
print('\n'.join(feats))
PY
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ] || [ -z "${FEATURES[0]:-}" ]; then
    COMBOS=("")   # the crate declares no features: exactly one configuration
else
    n=${#FEATURES[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        combo=()
        for ((i = 0; i < n; i++)); do
            (((mask >> i) & 1)) && combo+=("${FEATURES[i]}")
        done
        COMBOS+=("$(IFS=,; echo "${combo[*]}")")
    done
fi

echo "=== feature combinations to verify: ${#COMBOS[@]} ==="
for c in "${COMBOS[@]}"; do echo "  '--no-default-features --features ${c}'"; done

rc=0

# ---------------------------------------------------------------------------
# 2. cargo check + cargo test for every combination
# ---------------------------------------------------------------------------
for c in "${COMBOS[@]}"; do
    label="<none>"; [ -n "$c" ] && label="$c"
    echo
    echo "############ features: $label ############"

    echo "--- cargo check ---"
    if ! timeout 600 cargo check --no-default-features ${c:+--features "$c"} 2>&1 | tail -3; then
        echo "CHECK FAILED for [$label]"; rc=1
    fi

    echo "--- cargo test ---"
    out=$(timeout 600 cargo test --no-default-features ${c:+--features "$c"} 2>&1)
    echo "$out" | grep -E '^test result|^error' || true
    if echo "$out" | grep -qE 'FAILED|^error'; then
        echo "TESTS FAILED for [$label]"; rc=1
    fi
done

# ---------------------------------------------------------------------------
# 3. default + all-features + release profile
# ---------------------------------------------------------------------------
echo
echo "############ default features ############"
out=$(timeout 600 cargo test 2>&1); echo "$out" | grep -E '^test result|^error' || true
echo "$out" | grep -qE 'FAILED|^error' && { echo "TESTS FAILED (default)"; rc=1; }

echo
echo "############ --all-features ############"
out=$(timeout 600 cargo test --all-features 2>&1); echo "$out" | grep -E '^test result|^error' || true
echo "$out" | grep -qE 'FAILED|^error' && { echo "TESTS FAILED (all-features)"; rc=1; }

echo
echo "############ --release ############"
out=$(timeout 600 cargo test --release 2>&1); echo "$out" | grep -E '^test result|^error' || true
echo "$out" | grep -qE 'FAILED|^error' && { echo "TESTS FAILED (release)"; rc=1; }

# ---------------------------------------------------------------------------
# 4. symbol diff
# ---------------------------------------------------------------------------
echo
echo "############ symbol parity (nm -D) ############"
if ! ./scripts/symbol_diff.sh; then
    echo "SYMBOL DIFF NOT EMPTY"; rc=1
fi

echo
if [ "$rc" -eq 0 ]; then echo "ALL VERIFICATION PASSED"; else echo "VERIFICATION FAILED"; fi
exit $rc
