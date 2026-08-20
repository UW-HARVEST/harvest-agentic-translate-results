#!/usr/bin/env bash
# Phase A/D — enumerate every valid feature combination mechanically from
# Cargo.toml and `cargo check` each one.
#
# Features are read out of the [features] table rather than hard-coded, so a
# newly added feature is picked up automatically.
set -uo pipefail
cd "$(dirname "$0")"

mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z_][A-Za-z0-9_-]*[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml | sort -u
)

n=${#FEATURES[@]}
echo "features declared in Cargo.toml: ${n} (${FEATURES[*]:-none})"
echo "feature combinations to check:   $((1 << n))"
echo

fail=0
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=()
  for ((i = 0; i < n; i++)); do
    if (( (mask >> i) & 1 )); then combo+=("${FEATURES[$i]}"); fi
  done
  spec=$(IFS=,; echo "${combo[*]:-}")
  label=${spec:-<none>}

  for profile in dev release; do
    flags=(--no-default-features)
    [[ -n "$spec" ]] && flags+=(--features "$spec")
    [[ "$profile" == release ]] && flags+=(--release)

    printf 'cargo check --all-targets %-28s profile=%-7s ... ' "$label" "$profile"
    if out=$(timeout 300 cargo check --all-targets "${flags[@]}" 2>&1); then
      echo "OK"
    else
      echo "FAILED"
      echo "$out" | grep -E '^(error|warning: unused)' | head -20
      fail=1
    fi
  done
done

echo
if (( fail )); then
  echo "RESULT: at least one feature combination failed to check"
  exit 1
fi
echo "RESULT: all feature combinations check cleanly"
