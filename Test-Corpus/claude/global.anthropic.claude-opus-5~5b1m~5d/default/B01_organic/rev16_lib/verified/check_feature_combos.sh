#!/usr/bin/env bash
# Phase D: run the differential suite under every feature combination.
#
# Cargo.toml has no [features] section and no optional dependencies, so the
# powerset of features is the single empty set. This script derives that
# mechanically rather than assuming it, so it stays correct if features are
# added later.
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"

echo "== declared features =="
features="$(
  cargo "$CARGO_FLAGS" metadata --no-deps --format-version 1 2>/dev/null \
    | tr ',' '\n' \
    | sed -n 's/.*"features":{\(.*\)/\1/p' \
    || true
)"

# Robust extraction: list feature names from Cargo.toml's [features] table.
mapfile -t FEATS < <(
  awk '
    /^\[features\]/ { inblock = 1; next }
    /^\[/           { inblock = 0 }
    inblock && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "");
      print
    }
  ' Cargo.toml
)

if [ "${#FEATS[@]}" -eq 0 ]; then
  echo "(none declared)"
else
  printf '  %s\n' "${FEATS[@]}"
fi

status=0

run_case() {
  local label="$1"; shift
  echo
  echo "===================================================================="
  echo "CASE: $label"
  echo "  cargo test $CARGO_FLAGS $*"
  echo "===================================================================="
  if cargo test $CARGO_FLAGS "$@" 2>&1 | tail -n 8; then
    echo "  -> PASS"
  else
    echo "  -> FAIL"
    status=1
  fi
}

# Baseline: default features, and explicitly no default features.
run_case "default features"
run_case "--no-default-features" --no-default-features

# Powerset of any declared features.
n="${#FEATS[@]}"
if [ "$n" -gt 0 ]; then
  total=$(( 1 << n ))
  for (( mask = 0; mask < total; mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( (mask >> i) & 1 )); then
        combo="${combo:+$combo,}${FEATS[$i]}"
      fi
    done
    run_case "--no-default-features --features '${combo}'" \
      --no-default-features --features "$combo"
  done
fi

echo
if [ "$status" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$status"
