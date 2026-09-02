#!/usr/bin/env bash
# Phase D driver: enumerate every Cargo feature combination, and for each one
# build the cdylib, diff its exported symbols against the C .so, and run the
# full differential test suite (Phases B + C).
#
# Usage: translation/scripts/verify_all.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WS_ROOT="$(cd "$CRATE_DIR/.." && pwd)"
cd "$CRATE_DIR"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# --- 1. Locate / build the C .so -------------------------------------------
note "C shared library"
if ! ls "$WS_ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
  ( cd "$WS_ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
C_SO="$(ls "$WS_ROOT"/c_src/build/lib*.so | head -n1)"
echo "C  .so: $C_SO"

# --- 2. Enumerate feature combinations from Cargo.toml ---------------------
# Read the [features] table keys (excluding "default"), then emit the powerset.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "="); gsub(/[[:space:]]/, "", kv[1]);
      if (kv[1] != "default") print kv[1]
    }
  ' Cargo.toml
)

note "feature enumeration"
echo "declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of cargo flag-sets to test.
COMBOS=()
COMBOS+=("")                          # default features
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("--no-default-features")   # nothing enabled
  n=${#FEATURES[@]}
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

# --- 3. Per-combination: check, build, symbol-diff, test -------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  note "COMBO: $label"

  # shellcheck disable=SC2086
  if ! timeout 600 cargo check $combo >/dev/null 2>&1; then
    echo "cargo check FAILED for: $label"; fail=1; continue
  fi

  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release $combo >/dev/null 2>&1; then
    echo "cargo build FAILED for: $label"; fail=1; continue
  fi
  RUST_SO="$CRATE_DIR/target/release/libmd5_digest_lib.so"

  # Symbol parity: every symbol the C .so defines must be defined by the Rust
  # .so under the exact same name.
  nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u > /tmp/c_syms.$$
  nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u > /tmp/r_syms.$$
  missing="$(comm -23 /tmp/c_syms.$$ /tmp/r_syms.$$)"
  echo "C defines $(wc -l < /tmp/c_syms.$$) symbol(s); Rust defines $(wc -l < /tmp/r_syms.$$)"
  if [ -n "$missing" ]; then
    echo "MISSING FROM RUST .so:"; echo "$missing"; fail=1
  else
    echo "symbol diff: EMPTY (parity OK)"
  fi

  # Undefined non-libc symbols in the Rust .so would indicate untranslated code.
  undef="$(nm -D -u "$RUST_SO" | awk '{print $NF}' \
            | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_|^statx$|^gettid$' || true)"
  if [ -n "$undef" ]; then
    echo "UNDEFINED NON-LIBC SYMBOLS IN RUST .so:"; echo "$undef"; fail=1
  else
    echo "undefined non-libc symbols: none"
  fi
  rm -f /tmp/c_syms.$$ /tmp/r_syms.$$

  # Differential tests (Phase B + Phase C) against this build.
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test --release $combo 2>&1 | tail -n 25; then
    echo "cargo test FAILED for: $label"; fail=1
  fi
done

note "RESULT"
if [ "$fail" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED (symbol parity + Phase B + Phase C)"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
