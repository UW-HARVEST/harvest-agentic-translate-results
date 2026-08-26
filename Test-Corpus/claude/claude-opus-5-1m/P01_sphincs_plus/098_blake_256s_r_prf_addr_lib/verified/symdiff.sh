#!/bin/bash
# For each (backend,thash,secpar) combo, diff the C .so exported symbols against
# the Rust cdylib exported symbols.
set -u
R="$(cd "$(dirname "$0")" && pwd)"
export CARGO_NET_OFFLINE=true
LOG="${TMPDIR:-/var/tmp}/symdiff.log"
: > "$LOG"
COMBOS="${COMBOS:-}"
if [ -z "$COMBOS" ]; then
  COMBOS=""
  for bk in haraka sha2 shake blake; do for th in robust simple; do for sp in 128s 128f 192s 192f 256s 256f; do
    COMBOS="$COMBOS $bk:$th:$sp"; done; done; done
fi
total_missing=0
for c in $COMBOS; do
  bk=${c%%:*}; rest=${c#*:}; th=${rest%%:*}; sp=${rest##*:}
  d="$R/cbuild/$bk-$th-$sp"
  cs="${TMPDIR:-/var/tmp}/c.$bk.$th.$sp.syms"
  for so in $d/lib/$bk/lib$bk.so $d/app/libsphincs_core_det.so $d/app/libsphincs_core.so; do
    nm -D --defined-only "$so" 2>/dev/null | awk '{print $3}'
  done | sort -u > "$cs"
  ( cd "$R" && cargo build --release --no-default-features --features "$bk $th $sp" >>"$LOG" 2>&1 ) \
      || { echo "RUST BUILD FAIL $c"; continue; }
  rs="${TMPDIR:-/var/tmp}/r.$bk.$th.$sp.syms"
  nm -D --defined-only "$R/target/release/libsphincsplus.so" | awk '{print $3}' | sort -u > "$rs"
  miss=$(comm -23 "$cs" "$rs" | tr '\n' ' ')
  extra_undef=$(nm -D -u "$R/target/release/libsphincsplus.so" | awk '{print $2}' | grep -E '^(SPX_|crypto_sign|blake|sha2|sha256|sha512|shake|haraka|randombytes|seedexpander|AES256|DRBG|cst)' | tr '\n' ' ')
  n=$(comm -23 "$cs" "$rs" | wc -l)
  total_missing=$((total_missing+n))
  printf '%-22s missing(%d): %s%s\n' "$c" "$n" "$miss" "${extra_undef:+ | UNDEF: $extra_undef}"
done
echo "TOTAL MISSING=$total_missing"
