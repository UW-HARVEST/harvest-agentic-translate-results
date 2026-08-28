#!/usr/bin/env bash
# Runs the whole differential suite under EVERY feature combination.
#
# `Cargo.toml` declares no [features] table, so the complete combination set is
# the empty default plus the two explicit spellings of it. The feature list is
# extracted from Cargo.toml rather than hard-coded, so this keeps working if
# features are ever added.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 1

# --- enumerate declared features (excluding "default") ---------------------
mapfile -t features < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

combos=("" "--no-default-features" "--all-features")
# every subset of the declared features (there are none today, but be generic)
n=${#features[@]}
if (( n > 0 && n <= 8 )); then
    for ((mask = 1; mask < (1 << n); mask++)); do
        sel=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then sel="$sel,${features[$i]}"; fi
        done
        combos+=("--no-default-features --features ${sel#,}")
    done
fi

rc=0
for combo in "${combos[@]}"; do
    label="${combo:-<default>}"
    echo "=============================================================="
    echo "### feature combo: $label"
    echo "=============================================================="
    # shellcheck disable=SC2086
    if ! cargo build --offline --release $combo > "$TMPDIR/build_$$.log" 2>&1; then
        echo "FAIL: cargo build ($label)"; tail -n 30 "$TMPDIR/build_$$.log"; rc=1; continue
    fi
    ./check_symbols.sh || rc=1
    # shellcheck disable=SC2086
    if ! cargo test --offline --release $combo -- --test-threads=4 > "$TMPDIR/test_$$.log" 2>&1; then
        echo "FAIL: cargo test ($label)"; tail -n 40 "$TMPDIR/test_$$.log"; rc=1; continue
    fi
    grep -E '^test result:' "$TMPDIR/test_$$.log" | sed 's/^/  /'
    total_pass=$(grep -Eo '^test result: ok\. [0-9]+' "$TMPDIR/test_$$.log" | awk '{s+=$4} END {print s+0}')
    total_fail=$(grep -Eo '[0-9]+ failed' "$TMPDIR/test_$$.log" | awk '{s+=$1} END {print s+0}')
    echo "  => $total_pass passed, $total_fail failed  ($label)"
    (( total_fail == 0 )) || rc=1
done

echo "=============================================================="
if (( rc == 0 )); then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit $rc
