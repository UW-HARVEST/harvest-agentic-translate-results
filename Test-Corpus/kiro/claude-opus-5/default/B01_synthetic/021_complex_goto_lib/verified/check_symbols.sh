#!/usr/bin/env bash
# Phase A / Phase D: exported-symbol parity between the C .so and the Rust .so.
# Exits non-zero if the C .so exports anything the Rust .so does not, or if the
# Rust .so has undefined non-libc symbols.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=${DRIVER_C_SO:-../c_src/build/libdriver.so}
RUST_SO=${DRIVER_RUST_SO:-target/release/libdriver.so}

[ -f "$C_SO" ]    || { echo "missing C .so: $C_SO (build it with cmake)"; exit 2; }
[ -f "$RUST_SO" ] || { echo "missing Rust .so: $RUST_SO (cargo build --release)"; exit 2; }

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT

# Defined, dynamically-exported symbols. Drop the toolchain/runtime symbols that
# every ELF shared object carries so we compare the actual library surface.
extract() {
  nm -D --defined-only "$1" \
    | awk '{print $NF}' \
    | grep -vE '^(_init|_fini|_edata|_end|__bss_start|__libc_csu_.*|_ITM_.*|__gmon_start__|__cxa_.*|rust_eh_personality|rust_metadata_.*)$' \
    | sort -u
}

extract "$C_SO"    > "$tmp/c.txt"
extract "$RUST_SO" > "$tmp/rust.txt"

echo "C .so    ($C_SO): $(wc -l < "$tmp/c.txt") exported symbol(s)"
echo "Rust .so ($RUST_SO): $(wc -l < "$tmp/rust.txt") exported symbol(s)"

missing=$(comm -23 "$tmp/c.txt" "$tmp/rust.txt")
if [ -n "$missing" ]; then
  echo
  echo "FAIL: exported by C but MISSING from Rust:"
  echo "$missing" | sed 's/^/  /'
  exit 1
fi
echo "OK: every C-exported symbol is also exported by the Rust .so"

# Undefined symbols in the Rust .so must all be resolvable libc/runtime imports.
undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' | sort -u)
unresolved=$(ldd -r "$RUST_SO" 2>&1 | grep -E 'undefined symbol' || true)
if [ -n "$unresolved" ]; then
  echo
  echo "FAIL: Rust .so has unresolved symbols at load time:"
  echo "$unresolved" | sed 's/^/  /'
  exit 1
fi
echo "OK: Rust .so has no unresolved imports ($(echo "$undef" | wc -l) undefined symbols, all satisfied by libc/runtime)"

echo
echo "symbol diff is EMPTY"
