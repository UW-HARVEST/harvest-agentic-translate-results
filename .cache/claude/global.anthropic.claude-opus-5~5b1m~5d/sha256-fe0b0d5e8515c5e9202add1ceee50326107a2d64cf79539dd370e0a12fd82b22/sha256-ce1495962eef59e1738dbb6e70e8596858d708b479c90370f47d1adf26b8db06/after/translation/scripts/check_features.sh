#!/usr/bin/env bash
# Enumerates every feature combination declared in Cargo.toml and runs the
# complete differential suite for each one, in both debug and release.
#
# `translation/Cargo.toml` declares no [features] table, so the only
# configurations that exist are the default one and --no-default-features;
# the loop below is derived from Cargo.toml, not hard-coded, so it keeps
# working if features are ever added.
set -uo pipefail
cd "$(dirname "$0")/.."

FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml | sort -u)

echo "features declared in Cargo.toml: [${FEATURES:-<none>}]"

# Build the cross product of the declared features (empty set included).
COMBOS=("")
for f in $FEATURES; do
    NEW=()
    for c in "${COMBOS[@]}"; do
        NEW+=("$c")
        if [ -z "$c" ]; then NEW+=("$f"); else NEW+=("$c,$f"); fi
    done
    COMBOS=("${NEW[@]}")
done

rc=0
for profile in "" "--release"; do
  # default features
  echo "=============================================================="
  echo "== profile='${profile:-debug}' features=<default>"
  echo "=============================================================="
  ./scripts/run_tests.sh $profile || rc=1

  for combo in "${COMBOS[@]}"; do
    echo "=============================================================="
    echo "== profile='${profile:-debug}' --no-default-features --features '${combo}'"
    echo "=============================================================="
    if [ -z "$combo" ]; then
      ./scripts/run_tests.sh $profile --no-default-features || rc=1
    else
      ./scripts/run_tests.sh $profile --no-default-features --features "$combo" || rc=1
    fi
  done
done

echo
if [ $rc -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES (see above)"; fi
exit $rc
