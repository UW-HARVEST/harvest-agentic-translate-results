#!/usr/bin/env bash
# Build the C shared object and the Rust cdylib, then diff their exported
# symbols. Fails if the Rust .so is missing any symbol the C .so exports.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/.." && pwd)"
root="$(cd "$crate/.." && pwd)"

echo "== building C shared library =="
mkdir -p "$root/c_src/build"
cmake -S "$root/c_src" -B "$root/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build "$root/c_src/build" >/dev/null
c_so="$(find "$root/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | tail -1)"
echo "   $c_so"

echo "== building Rust cdylib (release) =="
( cd "$crate" && cargo build --release >/dev/null 2>&1 )
rust_so="$crate/target/release/libhsv_to_rgb_lib.so"
echo "   $rust_so"

echo "== exported symbol diff (C -> Rust) =="
c_syms="$(mktemp)"; rust_syms="$(mktemp)"
trap 'rm -f "$c_syms" "$rust_syms"' EXIT
nm -D --defined-only "$c_so"    | awk '{print $3}' | sort -u > "$c_syms"
nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u > "$rust_syms"

missing="$(comm -23 "$c_syms" "$rust_syms" || true)"
if [[ -n "$missing" ]]; then
  echo "MISSING from Rust .so:"
  echo "$missing" | sed 's/^/  /'
  exit 1
fi
echo "C exports   : $(wc -l < "$c_syms")"
echo "Rust exports: $(wc -l < "$rust_syms")"
echo "missing     : 0  ✅"

echo "== undefined non-libc symbols in Rust .so =="
nm -D --undefined-only "$rust_so" | awk '{print $2}' | sort -u | sed 's/^/  /'
