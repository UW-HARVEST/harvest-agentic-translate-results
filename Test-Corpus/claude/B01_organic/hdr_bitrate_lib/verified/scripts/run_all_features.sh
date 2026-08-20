#!/usr/bin/env bash
# Phase D: run the FULL differential suite (Phases B and C) under every valid
# feature combination, in both debug and release profiles.
set -uo pipefail
cd "$(dirname "$0")/.."

feats=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)
n=$(printf '%s' "$feats" | grep -c . || true)

fail=0
run () { # $1 label, rest args
  local label="$1"; shift
  out=$(timeout 600 cargo test "$@" 2>&1)
  passed=$(echo "$out" | grep -oE '[0-9]+ passed' | awk '{s+=$1} END{print s+0}')
  nf=$(echo "$out" | grep -cE '\.\.\. FAILED')
  if echo "$out" | grep -qE '^error\[E|^error: could not compile'; then
     echo "  BUILD-FAIL  $label"; fail=1
  elif [ "$nf" -gt 0 ]; then
     echo "  FAIL        $label ($nf failing)"; fail=1
     echo "$out" | grep -E '\.\.\. FAILED' | sed 's/^/              /'
  else
     echo "  ok          $label ($passed tests passed)"
  fi
}

export HDR_NO_DEFAULT_FEATURES=1
if [ "${n:-0}" -eq 0 ]; then
  echo "no [features] => single combination (empty set)"
  unset HDR_FEATURES
  run "--no-default-features                 [debug]"   --no-default-features
  run "--no-default-features                 [release]" --no-default-features --release
  unset HDR_NO_DEFAULT_FEATURES
  run "default features                      [debug]"
  run "default features                      [release]" --release
else
  arr=($feats)
  total=$(( 1 << ${#arr[@]} ))
  for ((mask=0; mask<total; mask++)); do
    combo=()
    for ((b=0; b<${#arr[@]}; b++)); do (( mask & (1<<b) )) && combo+=("${arr[b]}"); done
    csv=$(IFS=,; echo "${combo[*]}")
    export HDR_FEATURES="$csv"
    run "--features '${csv:-<none>}' [debug]"   --no-default-features ${csv:+--features "$csv"}
    run "--features '${csv:-<none>}' [release]" --no-default-features ${csv:+--features "$csv"} --release
  done
fi

echo
[ "$fail" -eq 0 ] && echo "ALL COMBINATIONS PASS ALL DIFFERENTIAL TESTS" || echo "FAILURES PRESENT"
exit $fail
