#!/usr/bin/env bash
# Phase D: derive the feature list from Cargo.toml and run the whole suite under
# every feature combination (plus --no-default-features and --all-features).
# If the crate declares no features, that is reported explicitly rather than
# assumed.
set -uo pipefail
cd "$(dirname "$0")"

# Features are the keys of the [features] table, minus `default`.
features=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); if (a[1] != "default") print a[1]
  }' Cargo.toml | sort -u)

n=$(printf '%s' "$features" | grep -c . || true)
echo "features declared in Cargo.toml: $n"
if [ "$n" -gt 0 ]; then echo "$features" | sed 's/^/  - /'; fi

run() {
  local label="$1"; shift
  echo
  echo "=== $label ==="
  cargo build --quiet "$@" 2>&1 | tail -5
  cargo build --quiet --release "$@" 2>&1 | tail -5
  if ! timeout 600 cargo test --quiet "$@" -- --test-threads=1 2>&1 | tail -15; then
    echo "FAIL: $label"
    return 1
  fi
  ./check_symbols.sh > /dev/null || { echo "FAIL (symbols): $label"; return 1; }
  echo "PASS: $label"
}

rc=0
run "default features" || rc=1
run "--no-default-features" --no-default-features || rc=1
run "--all-features" --all-features || rc=1

if [ "$n" -gt 0 ]; then
  # Full power set of the declared features.
  readarray -t arr <<< "$features"
  total=$((1 << ${#arr[@]}))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < ${#arr[@]}; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${arr[$i]}"; fi
    done
    run "--no-default-features --features '${combo}'" --no-default-features ${combo:+--features "$combo"} || rc=1
  done
else
  echo
  echo "No [features] table: the default configuration is the ONLY configuration."
  echo "(--no-default-features and --all-features are therefore identical to it,"
  echo " and were still run above to prove it.)"
fi

echo
[ "$rc" -eq 0 ] && echo "ALL FEATURE COMBINATIONS PASS" || echo "SOME FEATURE COMBINATIONS FAILED"
exit "$rc"
