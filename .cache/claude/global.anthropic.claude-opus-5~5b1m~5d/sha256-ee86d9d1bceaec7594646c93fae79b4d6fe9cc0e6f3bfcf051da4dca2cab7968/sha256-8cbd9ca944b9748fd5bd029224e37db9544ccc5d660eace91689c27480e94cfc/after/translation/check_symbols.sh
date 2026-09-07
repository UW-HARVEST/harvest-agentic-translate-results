#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
#
# Every symbol the C shared object exports must also be exported by the Rust
# shared object under the exact same name. The diff must be EMPTY.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
C_SO="${DIFFTEST_C_SO:-$root/c_src/build/libdriver.so}"
R_SO="${DIFFTEST_RUST_SO:-$here/target/release/libdriver.so}"

for f in "$C_SO" "$R_SO"; do
  if [ ! -f "$f" ]; then echo "missing $f" >&2; exit 2; fi
done

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# defined, dynamically-exported symbols
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$tmp/c.txt"
# the Rust cdylib additionally exports Rust/std internals with no C counterpart
nm -D --defined-only "$R_SO" | awk '{print $3}' \
  | grep -Ev '^(_ZN|_R|rust_|__rust|_rust)' | sort -u > "$tmp/r.txt"

echo "== C exports ($(wc -l < "$tmp/c.txt") symbols) =="
cat "$tmp/c.txt"
echo
echo "== Rust C-ABI exports ($(wc -l < "$tmp/r.txt") symbols) =="
cat "$tmp/r.txt"
echo

missing="$(comm -23 "$tmp/c.txt" "$tmp/r.txt")"
extra="$(comm -13 "$tmp/c.txt" "$tmp/r.txt")"

status=0
if [ -n "$missing" ]; then
  echo "MISSING from the Rust .so:"; echo "$missing"; status=1
else
  echo "MISSING from the Rust .so: (none)"
fi
if [ -n "$extra" ]; then
  echo "EXTRA C-ABI symbols in the Rust .so:"; echo "$extra"
else
  echo "EXTRA C-ABI symbols in the Rust .so: (none)"
fi

# undefined (imported) symbols in the Rust .so that are not resolvable by libc
echo
echo "== Rust .so undefined symbols =="
nm -D -u "$R_SO" | awk '{print $2}' | sort -u

if [ "$status" -eq 0 ]; then
  echo
  echo "SYMBOL PARITY: OK (0 missing)"
fi
exit "$status"
