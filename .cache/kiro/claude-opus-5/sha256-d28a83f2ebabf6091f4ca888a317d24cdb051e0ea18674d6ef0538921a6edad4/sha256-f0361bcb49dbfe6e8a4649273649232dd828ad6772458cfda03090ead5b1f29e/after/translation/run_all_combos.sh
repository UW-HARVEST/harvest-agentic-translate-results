#!/usr/bin/env bash
# Phase D: run the whole differential suite under every cargo feature
# combination and both profiles. Feature combos are extracted from Cargo.toml
# rather than hard-coded.
set -uo pipefail
cd "$(dirname "$0")"

FEATURES=$(awk '
  /^\[features\]/      {inf=1; next}
  /^\[/                {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {sub(/[[:space:]]*=.*/,""); print}
' Cargo.toml | grep -v '^default$' | sort -u)

echo "=== features declared in Cargo.toml: [${FEATURES:-<none>}] ==="

# Build the powerset of the declared features (empty set == default build).
combos=("")
for f in $FEATURES; do
  new=()
  for c in "${combos[@]}"; do new+=("$c" "${c:+$c,}$f"); done
  combos=("${new[@]}")
done

fail=0
for profile in "" "--release"; do
  for combo in "${combos[@]}"; do
    if [ -z "$FEATURES" ]; then
      label="default ${profile:-debug}"
      args=()
    elif [ -z "$combo" ]; then
      label="no-default-features ${profile:-debug}"
      args=(--no-default-features)
    else
      label="features=$combo ${profile:-debug}"
      args=(--no-default-features --features "$combo")
    fi

    echo
    echo "===== cargo check: $label ====="
    if ! timeout 300 cargo check ${profile:+$profile} "${args[@]}" >/dev/null 2>&1; then
      echo "CHECK FAILED: $label"; fail=1; continue
    fi

    echo "===== cargo test: $label ====="
    # The cdylib must exist for the loader before the tests run.
    timeout 300 cargo build ${profile:+$profile} "${args[@]}" >/dev/null 2>&1
    if timeout 600 cargo test ${profile:+$profile} "${args[@]}" 2>&1 | tee /tmp/dt-$$.log | grep -E '^test result'; then
      grep -qE 'test result: FAILED|[1-9][0-9]* failed' /tmp/dt-$$.log && { echo "TESTS FAILED: $label"; fail=1; }
    else
      echo "TESTS FAILED (no result line): $label"; fail=1
    fi
    rm -f /tmp/dt-$$.log
  done
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit "$fail"
