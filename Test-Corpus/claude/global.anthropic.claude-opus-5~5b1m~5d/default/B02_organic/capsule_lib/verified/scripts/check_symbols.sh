#!/usr/bin/env bash
# Phase D: the symbol diff between the C .so and the Rust .so MUST be empty.
set -uo pipefail
cd "$(dirname "$0")/.."

C_SO=$(ls ../c_src/build/*.so | head -1)
cargo build --release >/dev/null 2>&1
R_SO=target/release/libcapsule_lib.so

echo "C   .so: $C_SO"
echo "Rust.so: $R_SO"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort > "$tmp/c.txt"
nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort > "$tmp/r.txt"

echo "C exports:    $(wc -l < "$tmp/c.txt")"
echo "Rust exports: $(wc -l < "$tmp/r.txt")"

echo
echo "--- symbols in C but MISSING from Rust ---"
comm -23 "$tmp/c.txt" "$tmp/r.txt" | tee "$tmp/missing.txt"
echo "--- symbols in Rust but not in C ---"
comm -13 "$tmp/c.txt" "$tmp/r.txt" | tee "$tmp/extra.txt"

echo
echo "--- undefined (imported) non-libc / non-unwinder symbols in Rust ---"
nm -D --undefined-only "$R_SO" \
  | awk '{print $NF}' \
  | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location)' \
  | grep -vE '@GLIBC|@GCC' \
  | tee "$tmp/undef.txt"

nmissing=$(wc -l < "$tmp/missing.txt")
nundef=$(wc -l < "$tmp/undef.txt")
echo
echo "missing=$nmissing  unresolved-non-libc=$nundef"
if [ "$nmissing" -eq 0 ] && [ "$nundef" -eq 0 ]; then
  echo "SYMBOL PARITY: OK (0 missing, 0 unresolved non-libc)"
else
  echo "SYMBOL PARITY: FAILED"; exit 1
fi
