#!/usr/bin/env bash
# Phase A / D symbol parity: every dynamic symbol the C .so defines must also be
# defined by the Rust .so, under the exact same name. The diff must be empty.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=$(ls ../c_src/build/lib*.so 2>/dev/null | head -1)
R_SO=target/release/libinreftree_lib.so

if [ -z "${C_SO:-}" ]; then
  echo "C .so not built. Run:" >&2
  echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
  exit 1
fi
[ -f "$R_SO" ] || { echo "Rust .so not built. Run: cargo build --release" >&2; exit 1; }

defined() { nm -D --defined-only "$1" | awk '{print $3}' | sort -u; }

echo "C   : $C_SO      ($(defined "$C_SO" | wc -l) defined symbols)"
echo "Rust: $R_SO      ($(defined "$R_SO" | wc -l) defined symbols)"
echo

missing=$(comm -23 <(defined "$C_SO") <(defined "$R_SO"))
if [ -n "$missing" ]; then
  echo "MISSING from the Rust .so:"
  echo "$missing" | sed 's/^/  /'
  exit 1
fi
echo "symbol diff: EMPTY - the Rust .so exports every C symbol"

# The Rust .so must not *import* anything the C library defined itself.
bad=$(comm -12 <(defined "$C_SO") <(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sort -u))
if [ -n "$bad" ]; then
  echo "Rust .so has undefined references it should define: $bad"
  exit 1
fi
echo "undefined non-libc symbols in the Rust .so: 0"

echo
echo "shared surface:"
defined "$C_SO" | sed 's/^/  /'
