#!/usr/bin/env bash
# Enumerate every valid feature combination declared in Cargo.toml and run
# `cargo check` and `cargo test` for each, in both the dev and release profile.
#
# Usage: ./check-all-features.sh [check|test|all]
set -uo pipefail
cd "$(dirname "$0")"

mode="${1:-all}"

# ---- enumerate features ---------------------------------------------------
# Feature names are the `name = [...]` entries in the [features] table.
mapfile -t features < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

combos=("")   # the empty combination: --no-default-features
n=${#features[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${features[$i]}"
      fi
    done
    combos+=("$combo")
  done
fi
# The declared default feature set is a valid configuration in its own right.
combos+=("<default>")

echo "features declared: ${n} (${features[*]:-none})"
echo "combinations to verify: ${#combos[@]}"
echo

status=0
for combo in "${combos[@]}"; do
  for profile in "" "--release"; do
    if [[ "$combo" == "<default>" ]]; then
      flags=()
      label="default features"
    else
      flags=(--no-default-features)
      [[ -n "$combo" ]] && flags+=(--features "$combo")
      label="--no-default-features${combo:+ --features $combo}"
    fi
    tag="$label ${profile:-(dev)}"

    if [[ "$mode" == "check" || "$mode" == "all" ]]; then
      if timeout 600 cargo check --all-targets $profile "${flags[@]}" >/tmp/dc-check.log 2>&1; then
        echo "PASS check : $tag"
      else
        echo "FAIL check : $tag"; tail -30 /tmp/dc-check.log; status=1
      fi
    fi

    if [[ "$mode" == "test" || "$mode" == "all" ]]; then
      if timeout 600 cargo test $profile "${flags[@]}" >/tmp/dc-test.log 2>&1; then
        echo "PASS test  : $tag ($(grep -m1 'test result' /tmp/dc-test.log | tail -c 60 | tr -d '\n'))"
      else
        echo "FAIL test  : $tag"; tail -40 /tmp/dc-test.log; status=1
      fi
    fi
  done
done

echo
[[ $status -eq 0 ]] && echo "ALL CONFIGURATIONS PASS" || echo "SOME CONFIGURATIONS FAILED"
exit $status
