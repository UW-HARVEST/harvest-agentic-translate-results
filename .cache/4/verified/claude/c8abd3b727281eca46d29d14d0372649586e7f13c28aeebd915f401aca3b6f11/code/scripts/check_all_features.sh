#!/usr/bin/env bash
# Phase A/D: enumerate EVERY valid feature combination from Cargo.toml and run
# `cargo check` for each. Fails if any combination does not compile.
set -uo pipefail
cd "$(dirname "$0")/.."

# Extract feature names from the [features] table (excluding "default").
feats=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

n=$(printf '%s' "$feats" | grep -c . || true)
echo "features declared in Cargo.toml: ${n:-0}${feats:+ -> $(echo $feats | tr '\n' ' ')}"

fail=0
check () { # $1 = human label, rest = extra cargo args
  local label="$1"; shift
  if timeout 600 cargo check --quiet "$@" 2>&1 | grep -qE '^error'; then
     echo "  FAIL  $label"; fail=1
  else
     echo "  ok    $label"
  fi
}

# Always check the two canonical no-feature builds.
check "default features"        
check "--no-default-features"   --no-default-features

if [ "${n:-0}" -eq 0 ]; then
  echo
  echo "No [features] declared => exactly ONE valid feature combination (the empty set)."
  echo "The 'every feature combination' gate is satisfied by this single combination."
else
  # Full power set of the declared features.
  arr=($feats)
  total=$(( 1 << ${#arr[@]} ))
  for ((mask=0; mask<total; mask++)); do
    combo=()
    for ((b=0; b<${#arr[@]}; b++)); do
      (( mask & (1<<b) )) && combo+=("${arr[b]}")
    done
    csv=$(IFS=,; echo "${combo[*]}")
    check "--no-default-features --features '${csv}'" --no-default-features ${csv:+--features "$csv"}
  done
fi

echo
[ "$fail" -eq 0 ] && echo "ALL FEATURE COMBINATIONS CHECK CLEANLY" || echo "SOME COMBINATIONS FAILED"
exit $fail
