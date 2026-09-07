#!/usr/bin/env bash
# Phase D — symbol parity + every feature combination, fully automated.
#
# Run from the crate root:  ./scripts/verify.sh
set -uo pipefail

cd "$(dirname "$0")/.."
CRATE_ROOT="$PWD"
C_SO="$CRATE_ROOT/../c_src/build/libdriver.so"
RUST_SO="$CRATE_ROOT/target/release/libdriver.so"
FAIL=0

hdr() { printf '\n=== %s ===\n' "$1"; }
ok()  { printf '  [ok]   %s\n' "$1"; }
bad() { printf '  [FAIL] %s\n' "$1"; FAIL=1; }

# ---------------------------------------------------------------------------
hdr "Build C shared library"
( cd "$CRATE_ROOT/../c_src" \
  && mkdir -p build \
  && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "libdriver.so (C)" || bad "C build"

# ---------------------------------------------------------------------------
hdr "Enumerate feature combinations from Cargo.toml"
# Mechanically derive the set of build configurations rather than hardcoding it.
FEATURE_LINES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {print}' Cargo.toml)
FEATURES=$(printf '%s\n' "$FEATURE_LINES" | sed -n 's/^\([A-Za-z0-9_-]*\)[[:space:]]*=.*/\1/p' | grep -v '^default$')

# Combination list: always the default build and the no-default build; then the
# powerset of any declared non-default features.
COMBOS=("default" "none")
if [ -n "$FEATURES" ]; then
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
  ok "declared features: $(printf '%s ' "${FARR[@]}")"
else
  ok "no [features] section -> only the default/no-default configurations exist"
fi
printf '  combinations to verify: %s\n' "${COMBOS[*]}"

# ---------------------------------------------------------------------------
run_combo() {
  local combo="$1" args=()
  case "$combo" in
    default) args=() ;;
    none)    args=(--no-default-features) ;;
    *)       args=(--no-default-features --features "$combo") ;;
  esac

  hdr "Configuration: $combo"

  timeout 600 cargo build --release "${args[@]}" >/dev/null 2>&1 \
    && ok "cargo build --release ${args[*]}" \
    || { bad "cargo build ${args[*]}"; return; }

  # --- symbol parity -------------------------------------------------------
  local c_syms rust_syms missing
  c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u)
  rust_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u)
  missing=$(comm -23 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$rust_syms"))
  if [ -z "$missing" ]; then
    ok "symbol parity: 0 C symbols missing from Rust ($(printf '%s\n' "$c_syms" | wc -l) checked)"
  else
    bad "symbols missing from Rust .so:"; printf '        %s\n' $missing
  fi

  # --- undefined (imported) symbol audit ----------------------------------
  # Everything the Rust .so imports must be libc / libgcc / Rust-runtime.
  local rogue
  rogue=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
    | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_' || true)
  if [ -z "$rogue" ]; then
    ok "undefined symbols: 0 non-libc/non-runtime imports"
  else
    bad "unresolved non-libc imports:"; printf '        %s\n' $rogue
  fi

  # --- differential tests (Phases B and C) --------------------------------
  timeout 600 cargo test --release "${args[@]}" >/tmp/driver_test_$$.log 2>&1 \
    && ok "Phase B + Phase C differential rows: $(grep -c 'row .* \.\.\. ok' /tmp/driver_test_$$.log) passed" \
    || { bad "differential tests"; tail -40 /tmp/driver_test_$$.log; }
  rm -f /tmp/driver_test_$$.log

  # --- negative control ----------------------------------------------------
  # Prove the suite can actually SEE a divergence: point it at a deliberately
  # mutated "Rust" library and require the run to fail.
  local mut; mut=$(mktemp -d)
  cat >"$mut/mutant.c" <<'EOF'
#include <stdio.h>
/* Deliberately wrong: uppercase hex, and driver adds 2 instead of 1. */
void printHexCharLine(char c) { printf("%02X\n", c); }
void driver(char d) { char r = d + 2; printHexCharLine(r); }
EOF
  if cc -shared -fPIC -o "$mut/libdriver.so" "$mut/mutant.c" 2>/dev/null; then
    if DRIVER_RUST_SO="$mut/libdriver.so" \
       timeout 600 cargo test --release "${args[@]}" >/dev/null 2>&1; then
      bad "negative control: suite PASSED against a mutated library (tests are vacuous)"
    else
      ok "negative control: suite correctly rejects a mutated library"
    fi
  else
    printf '  [skip] negative control (no C compiler)\n'
  fi
  rm -rf "$mut"
}

for combo in "${COMBOS[@]}"; do
  run_combo "$combo"
done

# ---------------------------------------------------------------------------
hdr "Binary / driver executable check"
if grep -q 'add_executable' "$CRATE_ROOT/../c_src/CMakeLists.txt"; then
  bad "c_src declares add_executable but no stdout comparison is implemented"
else
  ok "c_src builds a SHARED library only (no add_executable); no binary stdout comparison applies"
fi
if [ -d "$CRATE_ROOT/src/bin" ] || grep -q '^\[\[bin\]\]' "$CRATE_ROOT/Cargo.toml"; then
  bad "crate declares a binary but no stdout comparison is implemented"
else
  ok "Rust crate declares no [[bin]] target"
fi

# ---------------------------------------------------------------------------
hdr "Result"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL PHASE D CHECKS PASSED"
else
  echo "PHASE D FAILURES PRESENT"
fi
exit "$FAIL"
