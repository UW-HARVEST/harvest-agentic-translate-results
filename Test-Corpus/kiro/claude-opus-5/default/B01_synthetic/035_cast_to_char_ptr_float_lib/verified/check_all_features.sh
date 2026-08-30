#!/usr/bin/env bash
# Enumerates every feature combination declared in Cargo.toml (excluding
# "default") and runs `cargo check` plus `cargo test` for each one.
#
# The crate currently declares no [features], so the only valid configuration
# is the empty one; the script still handles the general case so that it keeps
# working if features are added later.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "Declared features (${n}): ${FEATURES[*]:-<none>}"

combos=()
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=()
  for ((i = 0; i < n; i++)); do
    if ((mask & (1 << i))); then combo+=("${FEATURES[i]}"); fi
  done
  combos+=("$(IFS=,; echo "${combo[*]}")")
done

status=0
for combo in "${combos[@]}"; do
  label="${combo:-<empty>}"
  echo "=============================================================="
  echo "combination: ${label}"
  echo "=============================================================="

  args=(--no-default-features)
  if [[ -n "$combo" ]]; then args+=(--features "$combo"); fi

  if ! timeout 600 cargo check --all-targets "${args[@]}"; then
    echo "CHECK FAILED: ${label}"
    status=1
    continue
  fi
  if ! timeout 600 cargo test "${args[@]}"; then
    echo "TEST FAILED: ${label}"
    status=1
  fi
done

# The default feature set as consumers get it by default.
echo "=============================================================="
echo "combination: <default features>"
echo "=============================================================="
timeout 600 cargo check --all-targets || status=1
timeout 600 cargo test || status=1

if ((status == 0)); then
  echo "ALL COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$status"
