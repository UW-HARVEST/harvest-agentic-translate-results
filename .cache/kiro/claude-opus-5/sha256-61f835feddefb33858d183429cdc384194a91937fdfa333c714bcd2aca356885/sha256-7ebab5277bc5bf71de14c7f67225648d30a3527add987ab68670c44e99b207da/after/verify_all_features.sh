#!/usr/bin/env bash
# Enumerate every feature combination declared in translation/Cargo.toml and run
# `cargo check` then `cargo test` for each.
#
# Usage: ./verify_all_features.sh [check|test]   (default: both)
set -uo pipefail

cd "$(dirname "$0")/translation" || exit 1

# Extract feature names from the [features] table, ignoring `default`.
features=$(awk '
  /^\[features\]/ { in_f = 1; next }
  /^\[/           { in_f = 0 }
  in_f && /=/     { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "default" && a[1] !~ /^#/) print a[1] }
' Cargo.toml)

combos=()
if [[ -z "$features" ]]; then
  echo "No [features] table in Cargo.toml -> exactly one configuration (no features)."
  combos=("")
else
  # Power set of the declared features.
  feat_arr=($features)
  n=${#feat_arr[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if ((mask & (1 << b))); then
        combo="${combo:+$combo,}${feat_arr[b]}"
      fi
    done
    combos+=("$combo")
  done
fi

mode="${1:-both}"
fail=0

for combo in "${combos[@]}"; do
  label="${combo:-<none>}"
  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  if [[ "$mode" == "check" || "$mode" == "both" ]]; then
    echo "=== cargo check --no-default-features --features '$label' ==="
    if ! timeout 600 cargo check "${args[@]}" --all-targets >"/tmp/check-${combo:-none}.log" 2>&1; then
      echo "  CHECK FAILED (see /tmp/check-${combo:-none}.log)"
      tail -30 "/tmp/check-${combo:-none}.log"
      fail=1
      continue
    fi
    echo "  ok"
  fi

  if [[ "$mode" == "test" || "$mode" == "both" ]]; then
    for profile in "" "--release"; do
      echo "=== cargo test --no-default-features --features '$label' ${profile:-(debug)} ==="
      log="/tmp/test-${combo:-none}${profile//-/}.log"
      if ! timeout 600 cargo test "${args[@]}" $profile >"$log" 2>&1; then
        echo "  TEST FAILED (see $log)"
        grep -E "^test .*FAILED|panicked|^error" "$log" | head -30
        fail=1
        continue
      fi
      grep -E "test result" "$log" | sed 's/^/  /'
    done
  fi
done

if ((fail)); then
  echo "RESULT: failures detected"
  exit 1
fi
echo "RESULT: all ${#combos[@]} feature combination(s) pass"
