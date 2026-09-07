#!/usr/bin/env bash
# Phase D verification: symbol parity + every feature combination.
# Run from the repository root or from translation/.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
CRATE="$ROOT/translation"
C_BUILD="$ROOT/c_src/build"

fail=0
note() { printf '\n=== %s ===\n' "$1"; }

# ---------------------------------------------------------------------------
note "Building the C shared library"
mkdir -p "$C_BUILD" || exit 1
( cd "$C_BUILD" && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$C_BUILD" -maxdepth 1 -name '*.so' | head -1)"
echo "C  .so: $C_SO"

note "Building the Rust cdylib"
timeout 600 cargo build --release >/dev/null 2>&1 || { echo "cargo build FAILED"; exit 1; }
R_SO="$CRATE/target/release/libmodeselect_lib.so"
echo "Rust .so: $R_SO"

# ---------------------------------------------------------------------------
note "Symbol parity (nm -D, defined symbols)"
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > /tmp/pd_c.syms
nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > /tmp/pd_r.syms
echo "C exports   : $(wc -l < /tmp/pd_c.syms)"
echo "Rust exports: $(wc -l < /tmp/pd_r.syms)"
MISSING="$(comm -23 /tmp/pd_c.syms /tmp/pd_r.syms)"
if [ -n "$MISSING" ]; then
  echo "MISSING FROM RUST:"; echo "$MISSING"; fail=1
else
  echo "OK: 0 symbols missing from the Rust .so"
fi

note "Unresolved (undefined non-libc) symbols"
for so in "$C_SO" "$R_SO"; do
  out="$(ldd -r "$so" 2>&1 | grep -i 'undefined symbol' || true)"
  if [ -n "$out" ]; then echo "UNRESOLVED in $so:"; echo "$out"; fail=1;
  else echo "OK: $(basename "$so") fully resolved"; fi
done

# ---------------------------------------------------------------------------
note "Binary/driver targets"
BINS="$(grep -cE '^\[\[bin\]\]' Cargo.toml || true)"
HAS_MAIN=0; [ -f src/main.rs ] && HAS_MAIN=1
EXES="$(grep -cE 'add_executable' "$ROOT/c_src/CMakeLists.txt" || true)"
echo "Cargo [[bin]] sections: $BINS ; src/main.rs: $HAS_MAIN ; CMake add_executable: $EXES"
if [ "$BINS" = "0" ] && [ "$HAS_MAIN" = "0" ] && [ "$EXES" = "0" ]; then
  echo "OK: neither project builds a driver binary; stdout is compared per FFI call instead"
else
  echo "A driver binary exists - its stdout must be compared too"; fail=1
fi

# ---------------------------------------------------------------------------
note "Enumerating feature combinations from Cargo.toml"
# Features declared under a [features] table (excluding "default").
FEATURES="$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml | sort -u)"

if [ -z "$FEATURES" ]; then
  echo "No [features] table -> the only configurations are default and --no-default-features"
  COMBOS=("" "--no-default-features")
else
  echo "Declared features: $FEATURES"
  # Full power set of declared features, with and without default features.
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  COMBOS=()
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${FARR[$i]}"; fi
    done
    if [ -z "$sel" ]; then
      COMBOS+=("" "--no-default-features")
    else
      COMBOS+=("--features $sel" "--no-default-features --features $sel")
    fi
  done
fi

# ---------------------------------------------------------------------------
note "cargo check + full test suite for every feature combination"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  printf -- '--- combo: %s ---\n' "$label"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check --release $combo >/dev/null 2>&1; then
    echo "  cargo check FAILED for: $label"; fail=1; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release $combo >/dev/null 2>&1; then
    echo "  cargo build FAILED for: $label"; fail=1; continue
  fi
  # Re-verify symbol parity for this combo's .so.
  nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > /tmp/pd_r.syms
  m="$(comm -23 /tmp/pd_c.syms /tmp/pd_r.syms)"
  if [ -n "$m" ]; then echo "  MISSING SYMBOLS for $label:"; echo "$m"; fail=1; fi
  # shellcheck disable=SC2086
  out="$(timeout 600 cargo test --release $combo 2>&1)"
  if [ $? -ne 0 ]; then
    echo "  cargo test FAILED for: $label"
    echo "$out" | grep -E 'FAILED|panicked|^test result' | head -20
    fail=1
  else
    echo "$out" | grep -E '^test result' | sed 's/^/  /'
  fi
done

# ---------------------------------------------------------------------------
note "RESULT"
if [ "$fail" = "0" ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
