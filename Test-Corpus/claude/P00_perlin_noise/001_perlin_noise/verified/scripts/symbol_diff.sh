#!/usr/bin/env bash
# Compares the exported (defined, dynamic) symbols of the C shared object with
# those of the Rust cdylib.  Prints the diff and exits non-zero when the Rust
# side is missing anything the C side exports.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here"

profile="${1:-debug}"

[ -f target/cdiff/libc_driver.so ] || scripts/build_c_so.sh >/dev/null
if [ "$profile" = release ]; then
  timeout 600 cargo build --offline --release >/dev/null 2>&1
else
  timeout 600 cargo build --offline >/dev/null 2>&1
fi

c_so="target/cdiff/libc_driver.so"
rust_so="target/$profile/libstb_perlin_cli.so"

nm -D --defined-only "$c_so" | awk '{print $3}' | grep -v '^$' | sort -u >target/cdiff/c_syms.txt
nm -D --defined-only "$rust_so" | awk '{print $3}' | grep -v '^$' | sort -u >target/cdiff/rust_syms.txt

echo "--- C exports ($(wc -l <target/cdiff/c_syms.txt)) ---"
cat target/cdiff/c_syms.txt
echo "--- Rust exports ($(wc -l <target/cdiff/rust_syms.txt)) ---"
cat target/cdiff/rust_syms.txt

missing=$(comm -23 target/cdiff/c_syms.txt target/cdiff/rust_syms.txt)
echo "--- missing from Rust .so ---"
if [ -z "$missing" ]; then
  echo "(none)"
else
  echo "$missing"
fi

echo "--- undefined non-libc symbols in Rust .so ---"
nm -D --undefined-only "$rust_so" | awk '{print $NF}' |
  grep -v -E '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^_Unwind|^__cxa|^__tls_get_addr$|^statx$|^gettid$' |
  sort -u | tee target/cdiff/rust_undef.txt
[ -s target/cdiff/rust_undef.txt ] || echo "(none)"

[ -z "$missing" ] || exit 1
