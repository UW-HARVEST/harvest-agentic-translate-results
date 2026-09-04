#!/usr/bin/env bash
# Full differential verification: builds the C .so and every Rust build
# profile / feature combination, then runs the whole test suite against each.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CARGO="cargo --offline"
FAIL=0

echo "== building C shared library =="
( mkdir -p "$ROOT/c_src/build" \
  && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
echo "   $C_SO"

cd "$CRATE" || exit 1

# --- feature combinations -----------------------------------------------------
# Enumerated mechanically from Cargo.toml.  This crate declares no [features]
# table, so the only combinations are the implicit default (empty) one and
# --no-default-features (also empty).  Both are run anyway.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml | tr -d ' ')
echo "== feature table: [${FEATURES:-<none>}] =="

run_suite() { # $1 = label, $2 = rust .so, $3.. = extra cargo flags
  local label="$1"; shift
  local so="$1"; shift
  echo "-- $label  (rust .so: $so)"
  if [ ! -f "$so" ]; then echo "   MISSING $so"; FAIL=1; return; fi
  local out rc
  out=$(DRIVER_RUST_SO="$so" timeout 600 $CARGO test "$@" -- --test-threads=1 2>&1)
  rc=$?
  echo "$out" | grep -E '^(test |test result|running|failures:|  ---)' | tail -n 25
  if [ "$rc" -ne 0 ]; then
    echo "   *** FAILED ($label) ***"
    echo "$out" | tail -n 40
    FAIL=1
  fi
}

# --- symbol parity ------------------------------------------------------------
echo "== symbol parity =="
$CARGO build            >/dev/null 2>&1
$CARGO build --release  >/dev/null 2>&1
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > $TMPDIR/c.syms.$$
for so in target/debug/libdriver.so target/release/libdriver.so; do
  nm -D --defined-only "$so" | awk '{print $3}' | sort -u > $TMPDIR/r.syms.$$
  missing=$(comm -23 $TMPDIR/c.syms.$$ $TMPDIR/r.syms.$$)
  if [ -n "$missing" ]; then
    echo "   MISSING from $so:"; echo "$missing"; FAIL=1
  else
    echo "   $so: 0 missing symbols"
  fi
done
# Undefined symbols that are part of the platform runtime (libc / libgcc_s /
# libpthread) are expected; anything else would mean a missing translation.
undef=$(nm -D --undefined-only target/release/libdriver.so \
        | awk '{print $2}' \
        | grep -v '@GLIBC' | grep -v '@GCC_' | grep -v '@GLIBC_PRIVATE' \
        | grep -v '^__cxa' | grep -v '^_ITM_' | grep -v '^__gmon' \
        | grep -v '^_Unwind_' | grep -v '^$')
if [ -n "$undef" ]; then echo "   non-libc undefined: $undef"; FAIL=1
else echo "   0 non-libc undefined symbols"; fi
rm -f $TMPDIR/c.syms.$$ $TMPDIR/r.syms.$$

# --- the matrix ---------------------------------------------------------------
for combo in "default" "no-default-features"; do
  case "$combo" in
    default)             FLAGS=() ;;
    no-default-features) FLAGS=(--no-default-features) ;;
  esac
  $CARGO build "${FLAGS[@]}"           >/dev/null 2>&1
  $CARGO build "${FLAGS[@]}" --release >/dev/null 2>&1
  run_suite "features=$combo profile=debug   (unoptimised rust .so)" \
            "$CRATE/target/debug/libdriver.so"   "${FLAGS[@]}"
  run_suite "features=$combo profile=release (optimised rust .so)" \
            "$CRATE/target/release/libdriver.so" "${FLAGS[@]}"
done

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL GREEN"; else echo "THERE WERE FAILURES"; fi
exit "$FAIL"
