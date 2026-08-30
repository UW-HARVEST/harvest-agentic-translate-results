#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every valid
# build-time feature combination, in both the dev and release profiles.
#
# Usage: ./verify.sh          (run from the repository root or anywhere)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
TIMEOUT=600
fail=0

# ---------------------------------------------------------------- C ground truth
echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
[[ -f "$C_SO" ]] || { echo "missing $C_SO"; exit 1; }

# ------------------------------------------------- enumerate feature combinations
# Parse the [features] table from Cargo.toml. Every subset of the non-default
# feature names is a candidate combination; with an empty table the only
# configuration is the featureless one.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  COMBOS=("")   # no [features] table: a single configuration
else
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== feature combinations (${#COMBOS[@]}) =="
for c in "${COMBOS[@]}"; do echo "   - '${c:-<none>}'"; done

# ---------------------------------------------------------------------- the matrix
for profile in dev release; do
  flag=""; [[ $profile == release ]] && flag="--release"
  for combo in "${COMBOS[@]}"; do
    featargs=(--no-default-features)
    [[ -n "$combo" ]] && featargs+=(--features "$combo")

    label="profile=$profile features='${combo:-<none>}'"
    echo
    echo "===================================================================="
    echo "== $label"
    echo "===================================================================="

    if ! ( cd "$CRATE" && timeout $TIMEOUT cargo check $flag "${featargs[@]}" 2>&1 | tail -20 ); then
      echo "CHECK FAILED: $label"; fail=1; continue
    fi
    # Also check with the default feature set, which may differ from --no-default-features.
    if ! ( cd "$CRATE" && timeout $TIMEOUT cargo build $flag "${featargs[@]}" 2>&1 | tail -20 ); then
      echo "BUILD FAILED: $label"; fail=1; continue
    fi

    # nm -D parity between the C .so and the Rust .so for this configuration.
    outdir="$CRATE/target/$([[ $profile == release ]] && echo release || echo debug)"
    R_SO="$outdir/libdriver.so"
    if [[ ! -f "$R_SO" ]]; then
      echo "SYMBOL CHECK FAILED: no $R_SO"; fail=1; continue
    fi
    csyms=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TtDBRW]$/ && $3 !~ /^_/ {print $3}' | sort -u)
    rsyms=$(nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TtDBRW]$/ && $3 !~ /^_/ {print $3}' | sort -u)
    missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
    if [[ -n "$missing" ]]; then
      echo "SYMBOL PARITY FAILED for $label; Rust .so is missing:"; echo "$missing"; fail=1
    else
      echo "symbol parity OK ($(echo "$csyms" | wc -l) exported names)"
    fi

    if ! ( cd "$CRATE" && timeout $TIMEOUT cargo test $flag "${featargs[@]}" 2>&1 | tail -45 ); then
      echo "TESTS FAILED: $label"; fail=1
    fi
  done
done

echo
if (( fail )); then
  echo "RESULT: FAILURES (see above)"
else
  echo "RESULT: all configurations pass"
fi
exit $fail
