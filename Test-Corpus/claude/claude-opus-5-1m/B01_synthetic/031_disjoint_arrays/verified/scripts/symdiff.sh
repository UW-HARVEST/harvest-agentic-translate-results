#!/usr/bin/env bash
# Phase D: every symbol the C .so exports must also be exported by the Rust .so.
set -uo pipefail
cd "$(dirname "$0")/.."

RUST_SO=${RUST_SO:-target/debug/libfma_array.so}
if [ ! -f "$RUST_SO" ]; then
  echo "missing $RUST_SO -- run cargo build --all-targets" >&2
  exit 1
fi

api() { nm -D --defined-only "$1" | awk '{print $NF}' | grep -v '^_' | sort -u; }
all() { nm -D --defined-only "$1" | awk '{print $NF}' | sort -u; }

rc=0
rust_all=$(all "$RUST_SO")

for cso in c_src/build/libcref.so c_src/build/libcref_o2.so; do
  [ -f "$cso" ] || { echo "missing $cso -- run cargo build (build.rs makes it)" >&2; rc=1; continue; }
  echo "=== $cso ==="
  c_api=$(api "$cso")
  echo "$c_api" | sed 's/^/  export: /'
  miss=$(comm -23 <(echo "$c_api") <(echo "$rust_all"))
  if [ -n "$miss" ]; then
    echo "$miss" | sed 's/^/  MISSING FROM RUST: /'
    rc=1
  else
    echo "  missing from Rust: none"
  fi
done

echo "=== Rust .so unresolved non-libc/non-libgcc symbols ==="
extra=$(nm -D -u "$RUST_SO" | awk '{print $NF}' \
        | grep -v '@GLIBC' | grep -v '@GCC' \
        | grep -Ev '^(_ITM_|__gmon_start__|__cxa_)' || true)
if [ -n "$extra" ]; then
  echo "$extra" | sed 's/^/  UNRESOLVED: /'
  rc=1
else
  echo "  none"
fi

exit "$rc"
