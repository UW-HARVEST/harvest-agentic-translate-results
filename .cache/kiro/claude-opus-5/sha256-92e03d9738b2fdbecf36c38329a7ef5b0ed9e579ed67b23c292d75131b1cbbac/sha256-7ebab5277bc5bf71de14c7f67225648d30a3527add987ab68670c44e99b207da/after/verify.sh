#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every valid
# feature combination declared in translation/Cargo.toml.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
TIMEOUT=600
rc=0

echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
    && timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && timeout $TIMEOUT cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# --- enumerate feature combinations ------------------------------------------
# Read the [features] table from Cargo.toml; every non-default feature is
# treated as independently toggleable, so we test the full power set.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblk=1; next }
    /^\[/           { inblk=0 }
    inblk && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  COMBOS=("")   # no [features] table: the empty configuration is the only one
else
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== ${#COMBOS[@]} feature combination(s): ${COMBOS[*]:-<none>} =="

for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  echo
  echo "===================================================================="
  echo "== configuration: $label"
  echo "===================================================================="

  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  echo "-- cargo check [$label]"
  if ! ( cd "$CRATE" && timeout $TIMEOUT cargo check "${args[@]}" 2>&1 | tail -20 ); then
    echo "CHECK FAILED [$label]"; rc=1; continue
  fi

  # The tests dlopen target/<profile>/libdriver.so, so the cdylib must be built
  # with the same feature set before the test binaries run.
  echo "-- cargo build (cdylib) [$label]"
  if ! ( cd "$CRATE" && timeout $TIMEOUT cargo build "${args[@]}" 2>&1 | tail -20 ); then
    echo "BUILD FAILED [$label]"; rc=1; continue
  fi

  echo "-- nm -D symbol parity [$label]"
  c_syms=$(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort -u)
  rust_syms=$(nm -D --defined-only "$CRATE/target/debug/libdriver.so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))
  if [[ -n "$missing" ]]; then
    echo "MISSING EXPORTS [$label]:"; echo "$missing"; rc=1
  else
    echo "   all $(echo "$c_syms" | wc -l) C symbols present in the Rust cdylib"
  fi

  echo "-- cargo test [$label]"
  if ! ( cd "$CRATE" && timeout $TIMEOUT cargo test "${args[@]}" -- --test-threads=1 2>&1 | tail -30 ); then
    echo "TESTS FAILED [$label]"; rc=1
  fi
done

echo
if (( rc == 0 )); then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $rc
