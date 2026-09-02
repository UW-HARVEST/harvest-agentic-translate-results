#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# this keeps working if features are added later. With no [features] table the
# cross-product is {default, --no-default-features}, and both are still run.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
R_SO="target/release/libgen_ray_lib.so"

if [ -z "$C_SO" ]; then
  echo "FAIL: C .so not built. Run:"
  echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi

# --- enumerate the optional features declared in Cargo.toml -----------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/        { in_f = 1; next }
    /^\[/                  { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

COMBOS=()
COMBOS+=("DEFAULT|")                        # default feature set
COMBOS+=("NO_DEFAULT|--no-default-features")

# Full power set of the optional features (skipped entirely when there are none).
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then sel+=("${FEATURES[i]}"); fi
    done
    joined=$(IFS=,; echo "${sel[*]:-}")
    if [ -z "$joined" ]; then
      COMBOS+=("no-default:<none>|--no-default-features")
    else
      COMBOS+=("no-default:$joined|--no-default-features --features $joined")
      COMBOS+=("default+:$joined|--features $joined")
    fi
  done
fi

echo "=== feature combinations to verify: ${#COMBOS[@]} ==="
printf '  %s\n' "${COMBOS[@]%%|*}"
echo

FAILED=()
for combo in "${COMBOS[@]}"; do
  label="${combo%%|*}"
  flags="${combo#*|}"
  echo "-------------------------------------------------------------"
  echo "### $label   (cargo flags: ${flags:-<none>})"

  # shellcheck disable=SC2086
  if ! timeout 300 cargo build --release $flags > /tmp/build.$$ 2>&1; then
    echo "  BUILD FAILED"; tail -20 /tmp/build.$$; FAILED+=("$label: build"); continue
  fi

  # Symbol parity must hold for THIS combination, not just the default one.
  nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="t"{print $3}' | sort -u > /tmp/csym.$$
  nm -D --defined-only "$R_SO" | awk '$2=="T"||$2=="t"{print $3}' | sort -u > /tmp/rsym.$$
  missing="$(comm -23 /tmp/csym.$$ /tmp/rsym.$$)"
  if [ -n "$missing" ]; then
    echo "  SYMBOL PARITY FAILED -- missing from the Rust .so:"
    echo "$missing" | sed 's/^/    /'
    FAILED+=("$label: symbols")
  else
    echo "  symbols: $(wc -l < /tmp/csym.$$)/$(wc -l < /tmp/csym.$$) present, 0 missing"
  fi

  # shellcheck disable=SC2086
  if ! timeout 600 cargo test --release $flags > /tmp/test.$$ 2>&1; then
    echo "  TESTS FAILED"
    grep -E "^test .* FAILED|panicked|diverged|test result" /tmp/test.$$ | head -30
    FAILED+=("$label: tests")
  else
    passed=$(grep -oE '[0-9]+ passed' /tmp/test.$$ | awk '{s+=$1} END {print s}')
    echo "  tests: $passed passed, 0 failed"
  fi
done

rm -f /tmp/build.$$ /tmp/test.$$ /tmp/csym.$$ /tmp/rsym.$$
echo "============================================================="
if [ ${#FAILED[@]} -eq 0 ]; then
  echo "ALL ${#COMBOS[@]} FEATURE COMBINATIONS PASSED"
  exit 0
else
  echo "FAILURES:"; printf '  %s\n' "${FAILED[@]}"
  exit 1
fi
