#!/usr/bin/env bash
# Enumerates every valid cargo feature combination of this crate and runs
# `cargo check` (and optionally `cargo test`) for each one.
#
#   scripts/check_features.sh          # cargo check for every combination
#   scripts/check_features.sh test     # cargo test  for every combination
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here"

mode="${1:-check}"

# --- enumerate the [features] table of Cargo.toml -------------------------
mapfile -t features < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#features[@]}
echo "### optional features found: $n ${features[*]:-(none)}"

combos=()
if [ "$n" -eq 0 ]; then
  # No [features] table at all: the only build-time configuration of this crate
  # is the default (empty) one.  All three spellings must work.
  combos=("" "")
  runs=("--no-default-features" "--all-features" "")
else
  total=$((1 << n))
  runs=()
  for ((mask = 0; mask < total; mask++)); do
    set=()
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then set+=("${features[$i]}"); fi
    done
    joined=$(
      IFS=,
      echo "${set[*]}"
    )
    runs+=("--no-default-features --features $joined")
  done
  runs+=("--all-features" "")
fi

mkdir -p logs
status=0
i=0
for args in "${runs[@]}"; do
  label="${args:-<default>}"
  i=$((i + 1))
  log="logs/${mode}_combo_${i}.log"
  printf '\n=== cargo %s %s (log: %s) ===\n' "$mode" "$label" "$log"
  if [ "$mode" = test ]; then
    # shellcheck disable=SC2086
    timeout 600 cargo test --offline $args >"$log" 2>&1
  else
    # shellcheck disable=SC2086
    timeout 600 cargo check --offline --all-targets $args >"$log" 2>&1
  fi
  rc=$?
  grep -E '^(test result|error)' "$log" | sed 's/^/    /'
  passed=$(awk '/^test result: ok\./ { gsub(/[^0-9 ]/, "", $0); n += $1 } END { print n + 0 }' "$log")
  echo "    total tests passed: ${passed:-0}"
  if [ "$rc" -ne 0 ]; then
    echo "FAILED: cargo $mode $label (rc=$rc)"
    status=1
  fi
done

echo
if [ "$status" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS OK"; else echo "SOME COMBINATIONS FAILED"; fi
exit "$status"
