#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
# Regenerates the diff used by SYMBOLS.md.  Exit status 0 == parity reached.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -n1)"
R_SO="$ROOT/translation/target/release/libarr_ins_lib.so"

if [[ -z "$C_SO" || ! -f "$C_SO" ]]; then
  echo "FAIL: C .so not found under $ROOT/c_src/build (build it first)" >&2
  exit 2
fi
if [[ ! -f "$R_SO" ]]; then
  echo "FAIL: Rust .so not found at $R_SO (cargo build --release)" >&2
  exit 2
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TtDdBbWwRr]$/ {print $3}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TtDdBbWwRr]$/ {print $3}' | sort -u > "$tmp/r.txt"

echo "C   .so : $C_SO  ($(wc -l < "$tmp/c.txt") defined dynamic symbols)"
echo "Rust.so : $R_SO  ($(wc -l < "$tmp/r.txt") defined dynamic symbols)"
echo
echo "--- symbols in C but MISSING from Rust ---"
comm -23 "$tmp/c.txt" "$tmp/r.txt" | tee "$tmp/missing.txt"
echo "--- symbols only in Rust (extra; allowed but reported) ---"
comm -13 "$tmp/c.txt" "$tmp/r.txt"
echo
echo "--- undefined (imported) symbols in the Rust .so ---"
nm -D --undefined-only "$R_SO" | awk '{print $2}' | sort -u | tee "$tmp/undef.txt"
echo
# every import must be resolvable: ldd reports no "not found"
echo "--- ldd ---"
ldd "$R_SO" | sed 's/^/    /'
if ldd "$R_SO" | grep -q "not found"; then
  echo "FAIL: unresolved shared-library dependency" >&2
  exit 1
fi

if [[ -s "$tmp/missing.txt" ]]; then
  echo "FAIL: $(wc -l < "$tmp/missing.txt") symbol(s) missing from the Rust .so" >&2
  exit 1
fi
echo "OK: symbol parity reached (0 missing)."
