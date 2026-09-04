#!/usr/bin/env bash
# Phase A / Phase D — exported-symbol parity between the C and Rust .so.
# Regenerates the diff behind SYMBOLS.md. Exit status is non-zero if any symbol
# exported by the C library is missing from the Rust library.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
c_so="$root/c_src/build/libdriver.so"
rust_so="${RUST_SO:-$here/target/release/libdriver.so}"

if [[ ! -f "$c_so" ]]; then
  echo "building the C shared library first..."
  (cd "$root/c_src" && mkdir -p build && cd build \
     && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
     && cmake --build . >/dev/null)
fi
[[ -f "$rust_so" ]] || (cd "$here" && cargo build --release >/dev/null)

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$c_so"    | awk '{print $3}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u > "$tmp/r.txt"

echo "C   exports: $(wc -l < "$tmp/c.txt")"
echo "Rust exports: $(wc -l < "$tmp/r.txt")"

missing="$(comm -23 "$tmp/c.txt" "$tmp/r.txt")"
extra="$(comm -13 "$tmp/c.txt" "$tmp/r.txt")"

echo
echo "--- exported by C, MISSING from Rust ---"
[[ -z "$missing" ]] && echo "(none)" || echo "$missing"
echo "--- exported by Rust, absent from C ---"
[[ -z "$extra" ]] && echo "(none)" || echo "$extra"

# Genuinely unresolvable imports, as reported by the dynamic loader itself
# (`ldd -r` resolves data + function relocations against the real dependency
# graph, which is the only authoritative answer).
echo
echo "--- unresolved imports reported by the dynamic loader ---"
c_unres="$(ldd -r "$c_so" 2>&1 | grep -i "undefined symbol" || true)"
r_unres="$(ldd -r "$rust_so" 2>&1 | grep -i "undefined symbol" || true)"
echo "C   : ${c_unres:-(none)}"
echo "Rust: ${r_unres:-(none)}"

if [[ -n "$missing" ]]; then
  echo
  echo "FAIL: Rust .so does not export the full C surface."
  exit 1
fi
if [[ -n "$r_unres" ]]; then
  echo
  echo "FAIL: Rust .so has imports the loader cannot satisfy."
  exit 1
fi
echo
echo "PASS: symbol parity is exact."
