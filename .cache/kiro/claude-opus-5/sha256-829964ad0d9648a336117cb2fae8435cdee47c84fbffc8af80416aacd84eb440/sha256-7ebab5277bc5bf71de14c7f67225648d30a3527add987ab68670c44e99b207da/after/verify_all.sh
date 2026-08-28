#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination declared by translation/Cargo.toml
# and runs `cargo check` + `cargo test` for each, in both dev and release profiles.
#
# Usage: ./verify_all.sh            (from the working directory root)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOG=/tmp/verify_all.log
: > "$LOG"

# --- 1. build the C reference shared library -------------------------------
echo "== building C reference ==" | tee -a "$LOG"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) >> "$LOG" 2>&1 || { echo "C build FAILED (see $LOG)"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -n1)
echo "C library: $C_SO"

# --- 2. enumerate feature combinations ------------------------------------
# Read the [features] table; ignore "default" and any feature whose name starts
# with '_' (conventionally private). If there are no features, the only valid
# configuration is the empty one.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z_][A-Za-z0-9_-]*[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' "$CRATE/Cargo.toml"
)

N=${#FEATURES[@]}
echo "== declared features (${N}): ${FEATURES[*]:-<none>} =="

COMBOS=()
if [ "$N" -eq 0 ]; then
  COMBOS+=("")            # the single valid configuration
else
  for ((mask=0; mask<(1<<N); mask++)); do
    combo=""
    for ((i=0; i<N; i++)); do
      if (( mask & (1<<i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "== ${#COMBOS[@]} feature combination(s) to verify =="

# --- 3. cargo check for every combination ---------------------------------
FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  for prof in "" "--release"; do
    printf '  check %-24s %-10s ... ' "$label" "${prof:-dev}"
    if timeout 600 cargo check --manifest-path "$CRATE/Cargo.toml" \
         --all-targets --no-default-features \
         ${combo:+--features "$combo"} $prof >> "$LOG" 2>&1; then
      echo ok
    else
      echo FAILED; FAIL=1
    fi
  done
done

# --- 4. cargo test for every combination ----------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  for prof in "" "--release"; do
    printf '  test  %-24s %-10s ... ' "$label" "${prof:-dev}"
    # Build the cdylib for this exact configuration first so the harness loads
    # the artifact matching the features under test.
    timeout 600 cargo build --manifest-path "$CRATE/Cargo.toml" --lib \
      --no-default-features ${combo:+--features "$combo"} $prof >> "$LOG" 2>&1
    if timeout 600 cargo test --manifest-path "$CRATE/Cargo.toml" \
         --no-default-features ${combo:+--features "$combo"} $prof \
         -- --test-threads=4 >> "$LOG" 2>&1; then
      echo ok
    else
      echo FAILED; FAIL=1
    fi
  done
done

# --- 5. symbol parity -----------------------------------------------------
echo "== symbol parity =="
for prof in debug release; do
  RS_SO="$CRATE/target/$prof/libgjk_cache_lib.so"
  [ -f "$RS_SO" ] || continue
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort -u))
  if [ -n "$missing" ]; then
    echo "  $prof: MISSING from Rust .so:"; echo "$missing" | sed 's/^/    /'
    FAIL=1
  else
    echo "  $prof: all $(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u | wc -l) C symbols present"
  fi
done

if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "FAILURES PRESENT (details in $LOG)"
fi
exit "$FAIL"
