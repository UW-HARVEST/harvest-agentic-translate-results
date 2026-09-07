#!/usr/bin/env bash
# Full verification driver: builds both libraries, diffs the exported symbol
# sets (Phase A / D), then runs the differential suite (Phases B + C) under
# every cargo feature combination.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
fail=0
note() { printf '\n=== %s ===\n' "$*"; }

note "Build C shared library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$root/c_src/build/libdriver.so"

note "Build Rust cdylib"
( cd "$here" && cargo build --release --offline >/dev/null ) \
  || { echo "Rust build FAILED"; exit 1; }
R_SO="$here/target/release/libdriver.so"

note "Symbol parity (nm -D)"
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "${TMPDIR:-/tmp}/c.syms"
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > "${TMPDIR:-/tmp}/r.syms"
missing="$(comm -23 "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms")"
echo "C exports:    $(tr '\n' ' ' < "${TMPDIR:-/tmp}/c.syms")"
echo "Rust exports: $(tr '\n' ' ' < "${TMPDIR:-/tmp}/r.syms")"
if [ -n "$missing" ]; then
  echo "MISSING FROM RUST .so:"; echo "$missing"; fail=1
else
  echo "symbol diff: EMPTY (OK)"
fi

note "Undefined non-libc symbols in Rust .so"
undef="$(nm -D --undefined-only "$R_SO" \
  | awk '{print $2}' \
  | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_|^$' || true)"
if [ -n "$undef" ]; then echo "UNRESOLVED: $undef"; fail=1; else echo "none (OK)"; fi

note "Feature combinations declared in Cargo.toml"
combos="$(cd "$here" && cargo metadata --no-deps --format-version 1 --offline \
  | tr ',' '\n' | grep -o '"features":{[^}]*}' || true)"
echo "features field: ${combos:-<none>}"

# Run the suite for the default build and, since this crate declares no
# [features], also with --no-default-features to prove nothing is hidden there.
for cfg in "" "--no-default-features"; do
  note "cargo test ${cfg:-<default features>}"
  ( cd "$here" && C_DRIVER_SO="$C_SO" RUST_DRIVER_SO="$R_SO" \
      timeout 600 cargo test --offline $cfg -- --test-threads=1 ) || fail=1
done

note "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
