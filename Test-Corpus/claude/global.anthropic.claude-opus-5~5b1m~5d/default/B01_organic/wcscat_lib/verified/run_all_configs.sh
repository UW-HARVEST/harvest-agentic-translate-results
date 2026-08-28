#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination and
# under every build profile of the Rust cdylib.
#
# Feature combinations are extracted mechanically from Cargo.toml rather than
# hard-coded, so a newly added feature is picked up automatically.
set -u
cd "$(dirname "$0")"

FAILED=0
LOGF="$(pwd)/target/cfgrun.log"
mkdir -p target

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
COMBOS+=("--no-default-features")
COMBOS+=("")                      # default features
if [ ${#FEATURES[@]} -gt 0 ]; then
    COMBOS+=("--all-features")
    n=${#FEATURES[@]}
    total=$((1 << n))
    for ((mask = 1; mask < total; mask++)); do
        sel=()
        for ((i = 0; i < n; i++)); do
            if (((mask >> i) & 1)); then sel+=("${FEATURES[$i]}"); fi
        done
        joined=$(IFS=,; echo "${sel[*]}")
        COMBOS+=("--no-default-features --features $joined")
    done
fi

echo "features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]:-none})"
echo "feature combinations to test:    ${#COMBOS[@]}"
echo

# ---------------------------------------------------------------------------
# 2. cargo check every combination first (cheap fail-fast)
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    label=${combo:-'(default)'}
    if timeout 600 cargo check --all-targets $combo >/dev/null 2>&1; then
        echo "check  OK   $label"
    else
        echo "check  FAIL $label"
        timeout 600 cargo check --all-targets $combo 2>&1 | tail -20
        FAILED=$((FAILED + 1))
    fi
done
echo

# ---------------------------------------------------------------------------
# 3. Build the cdylib in both profiles and run the suite against each artifact.
#    WCSCAT_RUST_SO pins the exact .so the harness loads.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    for profile in debug release; do
        label="${combo:-'(default)'} / cdylib=$profile"
        outdir="target/cfgrun-$profile"
        if [ "$profile" = release ]; then
            timeout 600 cargo build --lib --release $combo \
                --target-dir "$outdir" >/dev/null 2>&1 || {
                echo "build  FAIL $label"; FAILED=$((FAILED + 1)); continue; }
            so="$outdir/release/libwcscat_lib.so"
        else
            timeout 600 cargo build --lib $combo \
                --target-dir "$outdir" >/dev/null 2>&1 || {
                echo "build  FAIL $label"; FAILED=$((FAILED + 1)); continue; }
            so="$outdir/debug/libwcscat_lib.so"
        fi
        if [ ! -f "$so" ]; then
            echo "build  FAIL $label (no $so)"; FAILED=$((FAILED + 1)); continue
        fi

        # Symbol parity for this artifact.
        c_so=$(ls ../c_src/build/lib*.so 2>/dev/null | head -1)
        missing=$(comm -23 \
            <(nm -D --defined-only "$c_so" | awk '{print $NF}' | sort -u) \
            <(nm -D --defined-only "$so"   | awk '{print $NF}' | sort -u))
        if [ -n "$missing" ]; then
            echo "symbol FAIL $label -> missing: $missing"
            FAILED=$((FAILED + 1))
        fi

        # Full suite (test harness itself built in release for speed).
        if WCSCAT_RUST_SO="$(realpath "$so")" \
           timeout 600 cargo test --release $combo >"$LOGF" 2>&1; then
            summary=$(grep -hE '^test result: ok' "$LOGF" \
                      | awk '{s+=$4} END {print s" tests passed"}')
            echo "tests  OK   $label  ($summary)"
        else
            echo "tests  FAIL $label"
            tail -40 "$LOGF"
            FAILED=$((FAILED + 1))
        fi
        :
    done
done

echo
if [ "$FAILED" -eq 0 ]; then
    echo "=== ALL CONFIGURATIONS PASSED ==="
else
    echo "=== $FAILED CONFIGURATION(S) FAILED ==="
fi
exit "$FAILED"
