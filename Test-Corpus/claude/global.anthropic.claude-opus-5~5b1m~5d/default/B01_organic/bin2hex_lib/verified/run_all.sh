#!/usr/bin/env bash
# Full verification driver: builds both libraries, diffs the dynamic symbol
# tables, and runs the differential test suites under every feature
# combination and both Rust optimisation profiles.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
fail=0
note() { printf '\n=== %s ===\n' "$*"; }
check() { if [ "$1" -eq 0 ]; then echo "PASS: $2"; else echo "FAIL: $2"; fail=1; fi; }

note "Build C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null )
check $? "C library build"
C_SO="$(find "$ROOT/c_src/build" -name '*.so' | sort | head -1)"
echo "C  .so: $C_SO"

# ---------------------------------------------------------------- features
# Enumerate feature combinations from Cargo.toml. This crate has no
# [features] section, so the only configuration is the default one; the loop
# below is written generically so it stays correct if features are ever added.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}' \
    "$CRATE/Cargo.toml" | grep -v '^default$'
)
echo "declared non-default features: ${#FEATURES[@]} (${FEATURES[*]-none})"

COMBOS=("default")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("none")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default) FLAGS=() ;;
    none)    FLAGS=(--no-default-features) ;;
    *)       FLAGS=(--no-default-features --features "$combo") ;;
  esac

  for profile in release debug; do
    PFLAGS=()
    [ "$profile" = release ] && PFLAGS=(--release)

    note "features=$combo profile=$profile"

    ( cd "$CRATE" && cargo build --offline "${FLAGS[@]}" "${PFLAGS[@]}" >/dev/null 2>&1 )
    check $? "cargo build (features=$combo, $profile)"

    RUST_SO="$CRATE/target/$profile/libbin2hex_lib.so"
    if [ ! -f "$RUST_SO" ]; then echo "FAIL: missing $RUST_SO"; fail=1; continue; fi

    # ---- Phase D: symbol parity -------------------------------------------
    diff <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort) > /dev/null
    if [ $? -eq 0 ]; then
      echo "PASS: exported symbol sets identical ($(nm -D --defined-only "$C_SO" | wc -l) symbol(s))"
    else
      echo "FAIL: exported symbol sets differ (features=$combo, $profile):"
      comm -23 <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort) \
               <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort) \
        | sed 's/^/  missing from Rust: /'
      fail=1
    fi

    # ---- Phases B & C ------------------------------------------------------
    ( cd "$CRATE" && BIN2HEX_RUST_SO="$RUST_SO" \
        cargo test --offline "${FLAGS[@]}" "${PFLAGS[@]}" --test differential 2>&1 | tail -3 )
    check ${PIPESTATUS[0]} "Phase B differential (features=$combo, $profile)"

    ( cd "$CRATE" && BIN2HEX_RUST_SO="$RUST_SO" \
        cargo test --offline "${FLAGS[@]}" "${PFLAGS[@]}" --test error_paths -- --test-threads=1 2>&1 | tail -3 )
    check ${PIPESTATUS[0]} "Phase C error paths (features=$combo, $profile)"
  done
done

note "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$fail"
