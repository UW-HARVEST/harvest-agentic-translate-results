#!/usr/bin/env bash
# Phase D — enumerate every feature combination declared in Cargo.toml and run
# cargo check + the full differential test suite for each.
set -uo pipefail

cd "$(dirname "$0")"

# --- extract the [features] section (excluding "default") -------------------
FEATURES="$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
  }' Cargo.toml | sort -u)"

NFEAT=$(echo -n "$FEATURES" | grep -c . || true)
echo "features declared in Cargo.toml: ${NFEAT}"
if [ "$NFEAT" -gt 0 ]; then
    echo "$FEATURES" | sed 's/^/  - /'
fi
echo

# --- build the combination list --------------------------------------------
# With N features there are 2^N subsets; plus the "default" build.
COMBOS=()
COMBOS+=("__default__")
COMBOS+=("__none__")
if [ "$NFEAT" -gt 0 ]; then
    mapfile -t FARR <<< "$FEATURES"
    n=${#FARR[@]}
    total=$((1 << n))
    for ((mask = 1; mask < total; mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( (mask >> i) & 1 )); then
                combo="${combo:+$combo,}${FARR[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi

echo "combinations to verify: ${#COMBOS[@]}"
echo

rc=0
for combo in "${COMBOS[@]}"; do
    case "$combo" in
        __default__) ARGS=() ;              label="default" ;;
        __none__)    ARGS=(--no-default-features); label="--no-default-features" ;;
        *)           ARGS=(--no-default-features --features "$combo"); label="--features $combo" ;;
    esac

    echo "=== [$label] cargo check ==="
    if ! timeout 600 cargo check --release "${ARGS[@]}" 2>&1 | grep -E "^error" ; then
        echo "  check OK"
    else
        echo "  CHECK FAILED"
        rc=1
        continue
    fi

    echo "=== [$label] cargo build --release ==="
    if ! timeout 600 cargo build --release "${ARGS[@]}" > /tmp/fb.log 2>&1; then
        echo "  BUILD FAILED"; tail -20 /tmp/fb.log; rc=1; continue
    fi

    echo "=== [$label] symbol parity ==="
    if ! ./check_symbols.sh | tail -3; then
        rc=1; continue
    fi

    echo "=== [$label] cargo test --release ==="
    if timeout 600 cargo test --release "${ARGS[@]}" 2>&1 | tee /tmp/ft.log | grep -E "^test result:"; then
        if grep -qE "FAILED|panicked" /tmp/ft.log; then
            echo "  TESTS FAILED"; rc=1
        else
            echo "  tests OK"
        fi
    else
        echo "  TESTS DID NOT REPORT"; tail -30 /tmp/ft.log; rc=1
    fi
    echo
done

if [ $rc -eq 0 ]; then
    echo "ALL ${#COMBOS[@]} FEATURE COMBINATIONS PASS"
else
    echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $rc
