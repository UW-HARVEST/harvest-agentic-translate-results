#!/usr/bin/env bash
# Phase D driver: enumerate every build-time configuration and run the full
# differential suite under each one.
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so this
# keeps working if features are ever added.
set -u
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"

# ---------------------------------------------------------------- features ---
# Everything between the [features] header and the next [section] header.
FEATURES=$(awk '
  /^\[features\]/ { inside=1; next }
  /^\[/           { inside=0 }
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
  }
' Cargo.toml | grep -v '^default$' | sort -u)

echo "=== features declared in Cargo.toml: [${FEATURES:-<none>}]"

# Build the power set of the feature list.
COMBOS=()
if [ -z "$FEATURES" ]; then
    COMBOS=("")
else
    mapfile -t FARR <<< "$FEATURES"
    n=${#FARR[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then
                combo="${combo:+$combo,}${FARR[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi
# If a `default` feature exists it is an extra configuration of its own.
if grep -qE '^default[[:space:]]*=' Cargo.toml; then
    COMBOS+=("__default__")
fi

echo "=== ${#COMBOS[@]} configuration(s) to verify"

fail=0

for combo in "${COMBOS[@]}"; do
    if [ "$combo" = "__default__" ]; then
        SEL=()
        label="(default features)"
    elif [ -z "$combo" ]; then
        SEL=(--no-default-features)
        label="--no-default-features"
    else
        SEL=(--no-default-features --features "$combo")
        label="--no-default-features --features $combo"
    fi

    echo
    echo "############################################################"
    echo "### CONFIG: $label"
    echo "############################################################"

    echo "--- cargo check --all-targets"
    if ! timeout 300 cargo check $CARGO_FLAGS --all-targets "${SEL[@]}" 2>&1 | tail -5; then
        echo "!!! cargo check FAILED for $label"
        fail=1
        continue
    fi

    # The differential tests load target/<profile>/libdriver.so; make sure it is
    # the cdylib built for THIS configuration (and not a stale one).
    rm -f target/debug/libdriver.so target/debug/libdriver_ffi.so
    echo "--- cargo build --lib (cdylib for this config)"
    timeout 300 cargo build $CARGO_FLAGS --lib "${SEL[@]}" >/dev/null 2>&1
    if [ ! -f target/debug/libdriver.so ]; then
        echo "!!! cargo build --lib did not produce target/debug/libdriver.so"
        fail=1
        continue
    fi

    echo "--- cargo test"
    if ! timeout 600 cargo test $CARGO_FLAGS "${SEL[@]}" 2>&1 \
            | grep -E 'Running|test result|FAILED|^error'; then
        echo "!!! cargo test produced no recognisable output for $label"
        fail=1
        continue
    fi
    if timeout 600 cargo test $CARGO_FLAGS "${SEL[@]}" 2>&1 | grep -qE 'FAILED|^error'; then
        echo "!!! cargo test FAILED for $label"
        fail=1
    fi
done

echo
if [ $fail -eq 0 ]; then
    echo "=== ALL CONFIGURATIONS PASSED"
else
    echo "=== SOME CONFIGURATIONS FAILED"
fi
exit $fail
