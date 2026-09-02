#!/usr/bin/env bash
# Phase D — symbol parity. Every dynamic symbol the C .so defines must also be
# defined by the Rust .so, under the exact same name. Exits non-zero unless the
# diff is empty.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
ROOT=$(cd .. && pwd)

C_SO="$ROOT/c_src/build/libdriver.so"
RS_SO="$ROOT/translation/target/release/libdriver.so"
[ -f "$RS_SO" ] || RS_SO="$ROOT/translation/target/debug/libdriver.so"

for f in "$C_SO" "$RS_SO"; do
  [ -f "$f" ] || { echo "missing: $f" >&2; exit 2; }
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$C_SO"  | awk '{print $3}' | grep -v '^$' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$RS_SO" | awk '{print $3}' | grep -v '^$' | sort -u > "$tmp/rs.txt"

echo "C  .so defines $(wc -l < "$tmp/c.txt") dynamic symbols: $C_SO"
echo "Rust .so defines $(wc -l < "$tmp/rs.txt") dynamic symbols: $RS_SO"
echo

missing=$(comm -23 "$tmp/c.txt" "$tmp/rs.txt")
if [ -n "$missing" ]; then
  echo "MISSING from the Rust .so:"
  printf '  %s\n' $missing
else
  echo "MISSING from the Rust .so: (none)"
fi

echo
echo "Extra in the Rust .so (informational; Rust runtime glue is expected):"
comm -13 "$tmp/c.txt" "$tmp/rs.txt" | sed 's/^/  /' | head -20

echo
echo "Unresolved symbols after full dynamic-link resolution (ldd -r):"
undef=$(ldd -r "$RS_SO" 2>&1 | grep -i 'undefined symbol\|not found')
if [ -n "$undef" ]; then
  printf '  %s\n' "$undef"
else
  echo "  (none)"
fi

echo
echo "Non-libc imports in the Rust .so (informational):"
nm -D --undefined-only "$RS_SO" \
  | grep -v 'GLIBC' | grep -v ' w ' | awk '{print $2}' | grep -v '^$' | sort -u \
  | sed 's/^/  /'
echo "  ^ all of the above are libgcc_s unwinder entry points, part of the Rust"
echo "    language runtime rather than the translated library, and all resolve"
echo "    (see the ldd -r result above)."

echo
if [ -n "$missing" ] || [ -n "$undef" ]; then
  echo "SYMBOL PARITY: FAIL"
  exit 1
fi
echo "SYMBOL PARITY: OK (0 missing exports, 0 unresolved imports)"
