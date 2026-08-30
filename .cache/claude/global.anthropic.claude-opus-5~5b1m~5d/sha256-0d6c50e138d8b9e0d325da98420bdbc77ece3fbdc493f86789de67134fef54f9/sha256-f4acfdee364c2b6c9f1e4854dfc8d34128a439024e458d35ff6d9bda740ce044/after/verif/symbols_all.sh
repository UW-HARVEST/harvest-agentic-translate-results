#!/usr/bin/env bash
# Build the Rust cdylib for every feature combination and diff its dynamic
# symbol table against the union of the two C shared libraries for the same
# configuration.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
OUT="$ROOT/verif/symbols"
mkdir -p "$OUT"

BACKENDS="${BACKENDS:-haraka sha2 shake blake}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"
THASHES="${THASHES:-robust simple}"

fail=0
for b in $BACKENDS; do
  for s in $SECPARS; do
    for t in $THASHES; do
      combo="$b,$t,$s"
      tag="$b-$s-$t"
      td="target/$tag"
      ( cd "$CRATE" && CARGO_TARGET_DIR="$td" timeout 600 cargo build --release \
          --no-default-features --features "$combo" >/dev/null 2>&1 ) || {
        echo "BUILDFAIL $combo"; fail=1; continue; }

      rso="$CRATE/$td/release/libsphincs_plus.so"
      cdir="$ROOT/c_src/build-$tag"
      nm -D --defined-only "$cdir/app/libsphincs_core.so" \
        "$cdir/app/libsphincs_core_det.so" "$cdir/lib/$b/lib$b.so" \
        | awk 'NF>=3 {print $3}' | sort -u > "$OUT/$tag.c.txt"
      nm -D --defined-only "$rso" | awk 'NF>=3 {print $3}' \
        | grep -v '^_' | sort -u > "$OUT/$tag.rust.txt"
      missing=$(comm -23 "$OUT/$tag.c.txt" "$OUT/$tag.rust.txt")
      if [ -z "$missing" ]; then
        echo "OK   $tag ($(wc -l < "$OUT/$tag.c.txt") C symbols all present)"
      else
        echo "FAIL $tag missing: $(printf '%s' "$missing" | tr '\n' ' ')"
        fail=1
      fi
    done
  done
done
exit $fail
