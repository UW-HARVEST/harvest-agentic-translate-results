#!/usr/bin/env bash
# Compare the exported symbol sets of the C and Rust shared objects. Every
# symbol the C .so exports must also be exported by the Rust .so, with the same
# name and (for data objects) the same size.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

c_so="${PCRE2_C_SO:-$root/c_src/build/libpcre2.so}"
r_so="${PCRE2_RUST_SO:-$here/target/release/libpcre2.so}"

test -f "$c_so" || { echo "missing $c_so" >&2; exit 1; }
test -f "$r_so" || { echo "missing $r_so" >&2; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u > "$tmp/c.names"
nm -D --defined-only "$r_so" | awk '{print $3}' | sort -u > "$tmp/r.names"

missing="$(comm -23 "$tmp/c.names" "$tmp/r.names")"
extra="$(comm -13 "$tmp/c.names" "$tmp/r.names")"

echo "C exports:    $(wc -l < "$tmp/c.names")"
echo "Rust exports: $(wc -l < "$tmp/r.names")"

status=0
if [ -n "$missing" ]; then
  echo "MISSING from the Rust .so:"; echo "$missing"; status=1
else
  echo "No missing symbols."
fi
if [ -n "$extra" ]; then
  echo "Only in the Rust .so (informational):"; echo "$extra"
fi

# Data object sizes must agree too.
for so in "$c_so" "$r_so"; do
  nm -D -S --defined-only "$so" \
    | awk 'NF==4 && $3 ~ /^[RrDdBb]$/ {print $4, $2}' | sort > "$tmp/$(basename "$(dirname "$so")").sizes"
done
if diff -u "$tmp/build.sizes" "$tmp/release.sizes" >/dev/null 2>&1; then
  echo "Data object sizes identical ($(wc -l < "$tmp/build.sizes") objects)."
else
  echo "Data object size differences:"
  diff -u "$tmp/build.sizes" "$tmp/release.sizes" || true
  status=1
fi

exit "$status"
