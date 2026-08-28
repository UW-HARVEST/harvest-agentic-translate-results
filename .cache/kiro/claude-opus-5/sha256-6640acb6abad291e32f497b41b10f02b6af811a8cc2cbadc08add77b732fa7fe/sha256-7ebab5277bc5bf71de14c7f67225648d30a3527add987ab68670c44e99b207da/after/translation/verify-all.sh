#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration declared in Cargo.toml.
set -uo pipefail
cd "$(dirname "$0")"

LOG=/tmp/verify-all.log
: > "$LOG"
fail=0

note() { printf '%s\n' "$*" | tee -a "$LOG"; }

# --- 1. enumerate feature combinations -------------------------------------
# Read [features] from Cargo.toml. Anything other than `default` is a real knob;
# we enumerate the powerset of the non-default features.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1] }
  ' Cargo.toml | grep -v '^default$'
)

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  note "Cargo.toml declares no [features]; single configuration only."
  COMBOS=("")
else
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
note "Feature combinations to verify: ${#COMBOS[@]}"

# --- 2. build the C shared library -----------------------------------------
note "== building C shared library =="
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build . ) >> "$LOG" 2>&1 || { note "C build FAILED"; exit 1; }
C_SO=$(find ../c_src/build -name '*.so' | sort | head -1)
note "C .so: $C_SO"

# --- 3..9. per-combination check / build / test / symbol diff --------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  note "===================================================================="
  note "== configuration: $label"

  for profile in "" "--release"; do
    pname=$([ -n "$profile" ] && echo release || echo debug)

    timeout 600 cargo check --no-default-features ${combo:+--features "$combo"} \
      $profile >> "$LOG" 2>&1 \
      || { note "  cargo check ($pname) FAILED"; fail=1; continue; }
    note "  cargo check ($pname): ok"

    timeout 600 cargo build --no-default-features ${combo:+--features "$combo"} \
      $profile >> "$LOG" 2>&1 \
      || { note "  cargo build ($pname) FAILED"; fail=1; continue; }

    RUST_SO="target/$pname/libhex2bin_lib.so"

    # --- symbol comparison: every dynamic symbol the C .so exports must
    #     also be exported by the Rust .so, with the identical name.
    missing=$(comm -23 \
      <(nm -D --defined-only --extern-only "$C_SO" | awk '{print $3}' | grep -v '^_' | sort -u) \
      <(nm -D --defined-only --extern-only "$RUST_SO" | awk '{print $3}' | sort -u))
    if [ -n "$missing" ]; then
      note "  MISSING EXPORTS ($pname): $(echo $missing)"
      fail=1
    else
      note "  symbols ($pname): all C exports present"
    fi

    timeout 600 cargo test --no-default-features ${combo:+--features "$combo"} \
      $profile >> "$LOG" 2>&1 \
      && note "  cargo test ($pname): ok" \
      || { note "  cargo test ($pname) FAILED"; fail=1; }
  done
done

note "===================================================================="
if (( fail )); then note "RESULT: FAILURES (see $LOG)"; else note "RESULT: all configurations match"; fi
exit $fail
