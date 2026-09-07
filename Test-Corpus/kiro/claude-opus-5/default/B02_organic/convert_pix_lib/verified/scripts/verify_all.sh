#!/usr/bin/env bash
# Phase D driver: build both libraries, diff their dynamic symbol tables, and run
# every test suite under every Cargo feature combination.
#
# Usage: translation/scripts/verify_all.sh [--fast]
#   --fast  skip the subprocess fuzz sweep (phase_c_aborts::fuzz_*), which costs
#           ~2 min because each live C assert kills a child process.
set -uo pipefail

FAST=0
[[ "${1:-}" == "--fast" ]] && FAST=1

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(dirname "$HERE")"
ROOT="$(dirname "$CRATE")"
rc=0
step() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
step "1. Build the C shared library"
# --------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "C  .so: $C_SO"

# --------------------------------------------------------------------------
step "2. Enumerate Cargo feature combinations"
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  cd "$CRATE" && cargo metadata --no-deps --format-version 1 2>/dev/null \
    | python3 -c '
import json,sys
m = json.load(sys.stdin)
feats = sorted(f for p in m["packages"] for f in p["features"] if f != "default")
print("\n".join(feats))
' | grep -v '^$'
)
if [[ ${#FEATURES[@]} -eq 0 ]]; then
  echo "no [features] declared -> the only configuration is the default one"
  COMBOS=("default")
else
  echo "features: ${FEATURES[*]}"
  # full power set, plus the default and no-default baselines
  COMBOS=("default" "none")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && combo+="${FEATURES[b]},"
    done
    COMBOS+=("${combo%,}")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

# --------------------------------------------------------------------------
step "3. Per-combination: build, symbol diff, full differential suite"
# --------------------------------------------------------------------------
SUITES=(smoke phase_b_valid phase_c_errors phase_c_aborts)
if [[ $FAST -eq 1 ]]; then SUITES=(smoke phase_b_valid phase_c_errors); fi

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default) FLAGS=() ;;
    none)    FLAGS=(--no-default-features) ;;
    *)       FLAGS=(--no-default-features --features "$combo") ;;
  esac
  printf '\n--- features: %s ---\n' "$combo"

  ( cd "$CRATE" && cargo build --release "${FLAGS[@]}" -q ) \
    || { echo "  Rust build FAILED"; rc=1; continue; }
  RS_SO="$CRATE/target/release/libconvert_pix_lib.so"

  # ---- symbol parity (Phase D gate) ----
  nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u > /tmp/.cp_sym_c.$$
  nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u > /tmp/.cp_sym_r.$$
  missing="$(comm -23 /tmp/.cp_sym_c.$$ /tmp/.cp_sym_r.$$)"
  extra="$(comm -13 /tmp/.cp_sym_c.$$ /tmp/.cp_sym_r.$$)"
  nc=$(wc -l < /tmp/.cp_sym_c.$$); nr=$(wc -l < /tmp/.cp_sym_r.$$)
  echo "  symbols: C=$nc Rust=$nr"
  if [[ -n "$missing" ]]; then
    echo "  MISSING from Rust .so:"; sed 's/^/    /' <<<"$missing"; rc=1
  else
    echo "  missing from Rust .so: none"
  fi
  if [[ -n "$extra" ]]; then
    echo "  EXTRA in Rust .so (C keeps these private):"; sed 's/^/    /' <<<"$extra"; rc=1
  else
    echo "  extra in Rust .so: none"
  fi
  # undefined non-libc references
  undef="$(nm -D --undefined-only "$RS_SO" | awk '{print $NF}' \
           | grep -vE '@GLIBC|^_ITM_|^__cxa_|^__gmon_start__$|^_Unwind_|^__tls_get_addr' || true)"
  if [[ -n "$undef" ]]; then
    echo "  UNDEFINED non-libc symbols in Rust .so:"; sed 's/^/    /' <<<"$undef"; rc=1
  else
    echo "  undefined non-libc symbols: none"
  fi
  rm -f /tmp/.cp_sym_c.$$ /tmp/.cp_sym_r.$$

  # ---- differential suites ----
  for t in "${SUITES[@]}"; do
    out=$( cd "$CRATE" && timeout 600 cargo test --release "${FLAGS[@]}" \
             --test "$t" -- --test-threads=1 2>&1 )
    line=$(grep -E '^test result' <<<"$out" | tail -1)
    if grep -qE 'FAILED|error:' <<<"$out"; then
      echo "  $t: FAILED  ${line:-<no result line>}"
      grep -E 'panicked at|assertion' <<<"$out" | head -5 | sed 's/^/      /'
      rc=1
    else
      echo "  $t: ${line:-ok}"
    fi
  done
done

# --------------------------------------------------------------------------
step "4. Binary / driver comparison"
# --------------------------------------------------------------------------
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  echo "c_src declares an executable -- stdout comparison REQUIRED but not implemented"; rc=1
else
  echo "c_src/CMakeLists.txt declares no add_executable, and translation/Cargo.toml"
  echo "declares no [[bin]] -- there is no driver binary. This gate is N/A."
fi

step "RESULT"
[[ $rc -eq 0 ]] && echo "ALL PHASE D CHECKS PASSED" || echo "FAILURES PRESENT (see above)"
exit $rc
