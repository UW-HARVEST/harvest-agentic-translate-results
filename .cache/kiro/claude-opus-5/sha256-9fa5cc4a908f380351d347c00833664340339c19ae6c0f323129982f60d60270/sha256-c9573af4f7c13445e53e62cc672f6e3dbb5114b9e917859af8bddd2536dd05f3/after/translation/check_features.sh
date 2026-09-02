#!/usr/bin/env bash
# Phase D — run the entire differential suite under EVERY feature combination
# and BOTH cargo profiles.
#
# Feature combinations are extracted mechanically from Cargo.toml rather than
# hard-coded, so a future `[features]` table is picked up automatically.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"

# ---- enumerate declared features ------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { infeat=1; next }
    /^\[/           { infeat=0 }
    infeat && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml
)

echo "declared features: ${FEATURES[*]:-<none>}"

# ---- build the combination list --------------------------------------------
# Always test: default, no-default-features, all-features.
COMBOS=("" "--no-default-features" "--all-features")
# Plus every non-empty subset of the declared features, on top of
# --no-default-features (so each feature's code path is isolated).
NF=${#FEATURES[@]}
if (( NF > 0 && NF <= 12 )); then
    for (( mask=1; mask < (1<<NF); mask++ )); do
        sel=()
        for (( b=0; b<NF; b++ )); do
            (( mask & (1<<b) )) && sel+=("${FEATURES[b]}")
        done
        COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    done
elif (( NF > 12 )); then
    echo "WARNING: $NF features -> $((1<<NF)) subsets; testing each feature alone" >&2
    for f in "${FEATURES[@]}"; do
        COMBOS+=("--no-default-features --features $f")
    done
fi

# De-duplicate.
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

echo "combinations to verify: ${#COMBOS[@]}"
printf '  [%s]\n' "${COMBOS[@]}"

# ---- 1. cargo check every combination (fast fail) ---------------------------
FAIL=0
for combo in "${COMBOS[@]}"; do
    printf '\n===== cargo check %s =====\n' "${combo:-<default>}"
    # shellcheck disable=SC2086
    if ! timeout 300 cargo check --all-targets $combo 2>&1 | tail -3; then
        echo "CHECK FAILED: ${combo:-<default>}"; FAIL=1
    fi
done

# ---- 2. full differential suite, every combination x both profiles ----------
LOG=$(mktemp -d)
for profile in release debug; do
    for combo in "${COMBOS[@]}"; do
        label="profile=$profile features=[${combo:-default}]"
        printf '\n===== TEST %s =====\n' "$label"
        out="$LOG/$(echo "$profile$combo" | tr -c 'A-Za-z0-9' '_').log"
        if PROFILE="$profile" FEATURES="$combo" timeout 600 ./run_tests.sh >"$out" 2>&1; then
            grep -E "^test result" "$out" | sed 's/^/  /'
            # Guard against a vacuous pass (no test binaries actually ran).
            n=$(grep -cE "^test result: ok" "$out" || true)
            if (( n < 6 )); then
                echo "  SUSPICIOUS: only $n test binaries reported ok for $label"
                FAIL=1
            fi
        else
            echo "TEST FAILED: $label"
            grep -E "^test result|panicked|FAILED|^error" "$out" | head -30
            FAIL=1
        fi
    done
done
rm -rf "$LOG"

echo
if (( FAIL )); then
    echo "RESULT: FAILURES PRESENT"
    exit 1
fi
echo "RESULT: all ${#COMBOS[@]} feature combination(s) x 2 profiles PASSED"
