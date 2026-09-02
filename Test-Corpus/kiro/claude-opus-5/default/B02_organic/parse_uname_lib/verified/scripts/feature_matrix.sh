#!/usr/bin/env bash
# Phase D — run the whole verification for EVERY feature combination.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so a
# future [features] table is picked up automatically. With no features declared,
# the matrix is the single default configuration.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

# Names in the [features] table (keys before '='), excluding "default".
features=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                    if (a[1] != "" && a[1] != "default" && a[1] !~ /^#/) print a[1] }
' Cargo.toml | sort -u)

n=$(printf '%s\n' "$features" | grep -c . )
echo "declared features: ${features:-<none>}  (count: $n)"

# Build the list of combinations to test.
combos=()
combos+=("DEFAULT")
if [ "$n" -gt 0 ]; then
  combos+=("NONE")
  # every non-empty subset (2^n - 1); guard against combinatorial explosion
  if [ "$n" -le 8 ]; then
    names=($features)
    total=$(( (1 << n) - 1 ))
    for ((mask=1; mask<=total; mask++)); do
      sel=""
      for ((i=0; i<n; i++)); do
        if (( mask & (1 << i) )); then sel="$sel,${names[i]}"; fi
      done
      combos+=("${sel#,}")
    done
  else
    echo "more than 8 features: testing all-on and each-one-on only" >&2
    combos+=("$(printf '%s\n' "$features" | paste -sd, -)")
    for f in $features; do combos+=("$f"); done
  fi
fi

fail=0
for combo in "${combos[@]}"; do
  case "$combo" in
    DEFAULT) args=() ;;
    NONE)    args=(--no-default-features) ;;
    *)       args=(--no-default-features --features "$combo") ;;
  esac
  echo
  echo "=============================================================="
  echo "combination: $combo   (cargo ${args[*]:-<default>})"
  echo "=============================================================="

  if ! timeout 600 cargo build --release --quiet "${args[@]}"; then
    echo "  build (release) FAILED"; fail=1; continue
  fi
  if ! timeout 600 cargo build --quiet "${args[@]}"; then
    echo "  build (debug) FAILED"; fail=1; continue
  fi
  if ! bash scripts/symbol_diff.sh | tail -1; then
    echo "  symbol parity FAILED"; fail=1
  fi
  if ! timeout 600 cargo test --quiet "${args[@]}" 2>&1 | tail -20; then
    echo "  tests FAILED"; fail=1
  fi
done

echo
if [ "$fail" -ne 0 ]; then echo "FEATURE MATRIX: FAIL"; exit 1; fi
echo "FEATURE MATRIX: OK for ${#combos[@]} combination(s)"
