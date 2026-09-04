#!/bin/bash
# Build the Rust cdylib for every feature combination into rsbuild/<tag>/ and
# diff its dynamic symbol table against the corresponding C .so pair.
#
#   no `urandom`  -> C = libsphincs_core_det.so (rng.c)   + lib<backend>.so
#   with urandom  -> C = libsphincs_core.so     (randombytes.c) + lib<backend>.so
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/rsbuild"
mkdir -p "$OUT"
mkdir -p /tmp/symlogs
fail=0
: > "$ROOT/symbol_parity.txt"

syms() { nm -D --defined-only "$1" 2>/dev/null | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"||$2=="W"{print $3}'; }

for b in blake haraka sha2 shake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      for r in "" ",urandom"; do
        combo="$b,$t,$s$r"
        tag="${b}_${t}_${s}${r//,/_}"
        cd "$ROOT/translation"
        if ! timeout 300 cargo build --release --offline --no-default-features \
              --features "$combo" > /tmp/symlogs/$tag.build.log 2>&1; then
          echo "BUILDFAIL $combo" >> "$ROOT/symbol_parity.txt"; fail=1; continue
        fi
        mkdir -p "$OUT/$tag"
        cp target/release/libsphincsplus.so "$OUT/$tag/"

        cdir="$ROOT/cbuild/${b}_${t}_${s}"
        if [ -z "$r" ]; then core="$cdir/app/libsphincs_core_det.so"; else core="$cdir/app/libsphincs_core.so"; fi
        back="$cdir/lib/$b/lib$b.so"
        { syms "$core"; syms "$back"; } | sort -u > /tmp/symlogs/$tag.c.txt
        syms "$OUT/$tag/libsphincsplus.so" | sort -u > /tmp/symlogs/$tag.rs.txt
        missing=$(comm -23 /tmp/symlogs/$tag.c.txt /tmp/symlogs/$tag.rs.txt | tr '\n' ' ')
        extra=$(comm -13 /tmp/symlogs/$tag.c.txt /tmp/symlogs/$tag.rs.txt | tr '\n' ' ')
        nc=$(wc -l < /tmp/symlogs/$tag.c.txt); nr=$(wc -l < /tmp/symlogs/$tag.rs.txt)
        if [ -z "$missing" ]; then
          echo "OK   $combo  C=$nc RS=$nr extra_in_rust=[$extra]" >> "$ROOT/symbol_parity.txt"
        else
          echo "MISS $combo  C=$nc RS=$nr missing=[$missing] extra=[$extra]" >> "$ROOT/symbol_parity.txt"
          fail=1
        fi
      done
    done
  done
done
echo "OK:   $(grep -c '^OK' "$ROOT/symbol_parity.txt")"
echo "BAD:  $(grep -cv '^OK' "$ROOT/symbol_parity.txt")"
grep -v '^OK' "$ROOT/symbol_parity.txt"
exit $fail
