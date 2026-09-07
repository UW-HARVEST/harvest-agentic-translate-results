#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# Feature combinations are derived mechanically from Cargo.toml. This crate
# declares no [features] table, so the enumeration collapses to the two
# equivalent no-feature builds (default and --no-default-features); the loop is
# written generically so it keeps working if features are ever added.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
cd "$here"

# ---- build the C reference .so -------------------------------------------
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build failed" >&2; exit 2; }

# ---- enumerate the declared features ------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
COMBOS+=("__default__")
COMBOS+=("__none__")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  total=$(( (1 << n) - 1 ))
  for mask in $(seq 1 "$total"); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "feature combinations to verify: ${COMBOS[*]}"
status=0

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) args=() ; label="(default features)" ;;
    __none__)    args=(--no-default-features) ; label="--no-default-features" ;;
    *)           args=(--no-default-features --features "$combo") ; label="--features $combo" ;;
  esac

  echo
  echo "=============================================================="
  echo ">>> $label"
  echo "=============================================================="

  if ! cargo build --release --offline "${args[@]}"; then
    echo "BUILD FAILED for $label"; status=1; continue
  fi
  if ! ./check_symbols.sh > "$here/target/symcheck.log" 2>&1; then
    echo "SYMBOL PARITY FAILED for $label"; cat "$here/target/symcheck.log"; status=1
  else
    tail -1 "$here/target/symcheck.log"
  fi
  rm -f "$here/target/symcheck.log"
  if ! cargo test --release --offline "${args[@]}"; then
    echo "TESTS FAILED for $label"; status=1
  fi
done

echo
if [ "$status" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$status"
