#!/usr/bin/env bash
# Phase D: enumerate every cargo feature combination and run the full
# verification (build the cdylib, check symbol parity, run all differential
# tests) under each one.
#
# The feature list is read out of the manifest rather than assumed, so if a
# feature is ever added this script picks it up automatically.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here"

# --- enumerate features from the manifest, not from assumption ---------------
mapfile -t FEATURES < <(
    cargo metadata --no-deps --format-version 1 2>/dev/null \
    | tr ',' '\n' \
    | sed -n 's/.*"features":{\(.*\)/\1/p' > /dev/null
    # Robust path: ask cargo directly.
    cargo metadata --no-deps --format-version 1 2>/dev/null \
    | python3 -c 'import json,sys; m=json.load(sys.stdin); print("\n".join(sorted(k for p in m["packages"] for k in p["features"])))'
)

# Drop empty entries.
FEATS=()
for f in "${FEATURES[@]:-}"; do [[ -n "$f" ]] && FEATS+=("$f"); done

echo "declared features: ${#FEATS[@]} ${FEATS[*]:-(none)}"

# --- build the combination list ---------------------------------------------
# With N features there are 2^N subsets; plus the default build.
COMBOS=()
COMBOS+=("--all-features")           # equals default when there are no features
COMBOS+=("--no-default-features")
n=${#FEATS[@]}
if (( n > 0 && n <= 12 )); then
    for (( mask=0; mask < (1<<n); mask++ )); do
        sel=()
        for (( b=0; b<n; b++ )); do
            (( mask & (1<<b) )) && sel+=("${FEATS[b]}")
        done
        if (( ${#sel[@]} == 0 )); then
            COMBOS+=("--no-default-features")
        else
            COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
        fi
    done
elif (( n > 12 )); then
    echo "WARNING: $n features => 2^$n subsets; testing singletons and all-features only" >&2
    for f in "${FEATS[@]}"; do
        COMBOS+=("--no-default-features --features $f")
    done
fi

# Deduplicate.
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

echo "combinations to verify: ${#COMBOS[@]}"
printf '  [%s]\n' "${COMBOS[@]}"
echo

# --- verify each combination ------------------------------------------------
fail=0
for combo in "${COMBOS[@]}"; do
    echo "############################################################"
    echo "# combination: cargo ... $combo"
    echo "############################################################"

    # shellcheck disable=SC2086
    if ! timeout 600 cargo build --release $combo > /tmp/fc_build.log 2>&1; then
        echo "BUILD FAILED for [$combo]"; tail -20 /tmp/fc_build.log; fail=1; continue
    fi
    echo "build: ok"

    if ! ./scripts/symbol_parity.sh | tail -3; then
        echo "SYMBOL PARITY FAILED for [$combo]"; fail=1; continue
    fi

    # The fast suite always runs. The heavy suite (~3 min) only when asked, via
    # RUN_HEAVY=1, so the sweep stays inside the time budget.
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test --release $combo --test differential > /tmp/fc_test.log 2>&1; then
        echo "DIFFERENTIAL TESTS FAILED for [$combo]"; tail -40 /tmp/fc_test.log; fail=1; continue
    fi
    echo "differential: $(grep -o 'test result: [^\n]*' /tmp/fc_test.log | tail -1)"

    if [[ "${RUN_HEAVY:-0}" == "1" ]]; then
        # shellcheck disable=SC2086
        if ! timeout 600 cargo test --release $combo --test heavy > /tmp/fc_heavy.log 2>&1; then
            echo "HEAVY TESTS FAILED for [$combo]"; tail -40 /tmp/fc_heavy.log; fail=1; continue
        fi
        echo "heavy: $(grep -o 'test result: [^\n]*' /tmp/fc_heavy.log | tail -1)"
    else
        # Still make sure the heavy suite at least compiles under this combo.
        # shellcheck disable=SC2086
        if ! timeout 600 cargo test --release $combo --test heavy --no-run > /tmp/fc_heavy.log 2>&1; then
            echo "HEAVY TESTS FAILED TO BUILD for [$combo]"; tail -20 /tmp/fc_heavy.log; fail=1; continue
        fi
        echo "heavy: compiled (set RUN_HEAVY=1 to execute; ~3 min)"
    fi
    echo
done

echo "############################################################"
if (( fail )); then
    echo "FEATURE SWEEP: FAIL"
else
    echo "FEATURE SWEEP: PASS (${#COMBOS[@]} combination(s))"
fi
exit $fail
