#!/usr/bin/env bash
# Runs the differential suite across EVERY feature combination x profile.
# Feature combos are extracted from Cargo.toml rather than hardcoded.
set -uo pipefail
cd "$(dirname "$0")"

# --- enumerate feature combinations -----------------------------------------
# Collect declared features (excluding "default"); build the powerset.
mapfile -t FEATS < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/ {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATS[@]}" -eq 0 ]; then
  echo "No [features] declared in Cargo.toml -> single configuration."
  COMBOS+=("--no-default-features")   # identical to default when no features exist
  COMBOS+=("")                        # plain default build
else
  n=${#FEATS[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATS[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${combo}")
  done
  COMBOS+=("")            # default features
  COMBOS+=("--all-features")
fi

# --- rebuild the C ground truth ---------------------------------------------
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
nm -D --defined-only ../c_src/build/libSimpleList.so | awk '{print $NF}' | sort -u > /tmp/c_syms.txt

FAIL=0
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo test ${profile:-<dev>} ${combo:-<default-features>}"
    echo "=================================================================="
    echo "### $label"

    # shellcheck disable=SC2086
    timeout 600 cargo check --quiet $profile $combo 2>&1 | tail -5
    # shellcheck disable=SC2086
    timeout 600 cargo build --quiet --lib $profile $combo 2>&1 | tail -5

    # symbol parity for this configuration
    pdir=$([ -n "$profile" ] && echo release || echo debug)
    so="target/${pdir}/libSimpleList.so"
    if [ -f "$so" ]; then
      nm -D --defined-only "$so" | awk '{print $NF}' | sort -u > /tmp/r_syms.txt
      missing=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
      if [ -n "$missing" ]; then
        echo "SYMBOL PARITY FAILURE, missing from Rust .so:"; echo "$missing"; FAIL=1
      else
        echo "symbol parity: OK (0 missing)"
      fi
    else
      echo "MISSING cdylib $so"; FAIL=1
    fi

    # shellcheck disable=SC2086
    if timeout 600 cargo test --quiet $profile $combo 2>&1 | tail -12; then
      echo "tests: OK"
    else
      echo "TESTS FAILED for: $label"; FAIL=1
    fi
  done
done

echo "=================================================================="
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$FAIL"
