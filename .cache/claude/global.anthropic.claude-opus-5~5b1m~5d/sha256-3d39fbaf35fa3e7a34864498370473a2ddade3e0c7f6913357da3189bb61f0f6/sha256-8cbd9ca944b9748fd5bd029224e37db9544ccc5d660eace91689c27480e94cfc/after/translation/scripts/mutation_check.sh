#!/usr/bin/env bash
# Harness self-check ("does the suite actually catch anything?").
#
# Each entry below deliberately breaks the Rust translation in a way the C code
# does NOT have.  Every mutant tagged KILL must make the test suite fail; a
# surviving KILL mutant is a blind spot in the differential suite.
#
# A few mutants are tagged EQUIV: they are *provably* semantically equivalent to
# the original for every input reachable through the public API, so no test can
# distinguish them.  The proof is recorded next to each one and is also asserted
# mechanically in tests/phase_c_errors.rs (e9_shift_count_always_in_range) and
# tests/phase_b_configs.rs (c22_choff_carry_across_granules).
set -u
cd "$(dirname "$0")/.."
SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

declare -a KIND=() NAMES=() FROM=() TO=()
add() { KIND+=("$1"); NAMES+=("$2"); FROM+=("$3"); TO+=("$4"); }

# ---------------------------------------------------------------- get_bits ----
add KILL "first-byte mask dropped" \
    '(255u32 >> s)' '(255u32)'
add KILL "first-byte mask uses 254" \
    '(255u32 >> s)' '(254u32 >> s)'
add KILL "limit check off by one (>= instead of >)" \
    'if (*bs).pos > (*bs).limit {' 'if (*bs).pos >= (*bs).limit {'
add KILL "limit check compares against pos before the update" \
    '(*bs).pos = (*bs).pos.wrapping_add(n);
    if (*bs).pos > (*bs).limit {' \
    'let __old = (*bs).pos; (*bs).pos = (*bs).pos.wrapping_add(n);
    if __old > (*bs).limit {'
add KILL "limit exceeded returns 1 instead of 0" \
    'if (*bs).pos > (*bs).limit {
        return 0;' \
    'if (*bs).pos > (*bs).limit {
        return 1;'
add KILL "pos not advanced when the limit is exceeded" \
    '(*bs).pos = (*bs).pos.wrapping_add(n);
    if (*bs).pos > (*bs).limit {
        return 0;
    }' \
    'if (*bs).pos.wrapping_add(n) > (*bs).limit { return 0; }
    (*bs).pos = (*bs).pos.wrapping_add(n);'
add KILL "cache shifted the wrong way" \
    'cache |= next.wrapping_shl(shl as u32);' \
    'cache |= next.wrapping_shr(shl as u32);'
add KILL "cache combined with xor" \
    'cache | next.wrapping_shr(shl.wrapping_neg() as u32)' \
    'cache ^ !next.wrapping_shr(shl.wrapping_neg() as u32)'
add KILL "final shift not negated" \
    'next.wrapping_shr(shl.wrapping_neg() as u32)' \
    'next.wrapping_shr(shl as u32)'
add KILL "byte pointer not advanced in the loop" \
    'next = *p as u32;
        p = p.wrapping_offset(1);' \
    'next = *p as u32;'
# PROOF: the loop can only reach shl == 0 exactly, and in that case the extra
# iteration contributes `next << 0` (already OR-ed in by the original's final
# `next >> -shl` with shl==0) and then returns `newnext >> 8`, which is 0 because
# `next` is always a single byte.  Output is bit-identical; the only difference is
# one extra, unobservable byte load.
add EQUIV "loop condition > 0 becomes >= 0" \
    'if shl <= 0 {
            break;
        }' \
    'if shl < 0 {
            break;
        }'
add KILL "shl decremented by 7" \
    'shl = shl.wrapping_sub(8);' 'shl = shl.wrapping_sub(7);'
add KILL "arithmetic >> 3 becomes logical" \
    '((*bs).pos >> 3) as isize' '(((*bs).pos as u32) >> 3) as isize'
add KILL "pos & 7 becomes pos & 15" \
    '((*bs).pos & 7) as u32' '(((*bs).pos as u32) & 15) as u32'
add KILL "pos & 7 uses rem_euclid" \
    '((*bs).pos & 7) as u32' '((*bs).pos.rem_euclid(8) + 1) as u32'

# ------------------------------------------------------ dequantize_granule ----
add KILL "half off by one" '.wrapping_sub(1);' '.wrapping_sub(2);'
add KILL "half uses ba instead of ba-1" \
    '1i32.wrapping_shl((ba - 1) as u32)' '1i32.wrapping_shl(ba as u32)'
add KILL "half not subtracted" \
    '(get_bits(bs, ba) as c_int).wrapping_sub(half)' \
    '(get_bits(bs, ba) as c_int)'
add KILL "get_bits result treated as unsigned before the float cast" \
    'let v = (get_bits(bs, ba) as c_int).wrapping_sub(half);
                        *dst.wrapping_offset(k as isize) = v as f32;' \
    'let v = (get_bits(bs, ba) as u32).wrapping_sub(half as u32);
                        *dst.wrapping_offset(k as isize) = v as f32;'
add KILL "code divided before use" \
    'let v = (code % m).wrapping_sub(m / 2) as c_int;' \
    'code /= m; let v = (code % m).wrapping_sub(m / 2) as c_int;'
add KILL "code divided twice" 'code /= m;' 'code /= m; code /= m;'
add KILL "modulo replaced by division" '(code % m)' '(code / m)'
add KILL "mod/2 becomes mod/2+1" \
    '.wrapping_sub(m / 2) as c_int' '.wrapping_sub(m / 2 + 1) as c_int'
# PROOF: two's-complement subtraction is sign-agnostic, so
# `(a as u32).wrapping_sub(b) as i32` and `(a as i32).wrapping_sub(b as i32)`
# produce identical bit patterns for all a, b.
add EQUIV "grouped subtraction done in signed space" \
    '(code % m).wrapping_sub(m / 2) as c_int' \
    '((code % m) as c_int).wrapping_sub((m / 2) as c_int)'
add KILL "mod is 2<<(ba-17) without the +1" \
    'as u32).wrapping_add(1);' 'as u32);'
add KILL "mod uses 1<<(ba-17)+1" \
    '(2i32.wrapping_shl((ba - 17) as u32) as u32)' \
    '(1i32.wrapping_shl((ba - 17) as u32) as u32)'
add KILL "field width forgets the -(mod>>3)" \
    'm.wrapping_add(2).wrapping_sub(m >> 3)' 'm.wrapping_add(2)'
add KILL "field width uses mod>>2" \
    'm.wrapping_add(2).wrapping_sub(m >> 3)' 'm.wrapping_add(2).wrapping_sub(m >> 2)'
add KILL "shift count not masked in 2 << (ba-17)" \
    '2i32.wrapping_shl((ba - 17) as u32)' \
    'if ba - 17 >= 32 { 0i32 } else { 2i32 << (ba - 17) }'
add KILL "grouped/linear threshold 17 -> 16" 'if ba < 17 {' 'if ba < 16 {'
add KILL "grouped/linear threshold 17 -> 18" 'if ba < 17 {' 'if ba < 18 {'
add KILL "zero-band guard inverted" 'if ba != 0 {' 'if ba >= 0 {'
add KILL "granule count 4 -> 3" 'while j < 4 {' 'while j < 3 {'
add KILL "granule count 4 -> 5" 'while j < 4 {' 'while j < 5 {'
add KILL "choff step 18 -> 19" \
    'choff = 18i32.wrapping_sub(choff);' 'choff = 19i32.wrapping_sub(choff);'
add KILL "choff step 18 -> 17" \
    'choff = 18i32.wrapping_sub(choff);' 'choff = 17i32.wrapping_sub(choff);'
add KILL "choff starts at 575" 'let mut choff: c_int = 576;' 'let mut choff: c_int = 575;'
add KILL "choff advanced before the band body" \
    'let ba: c_int = *bitalloc.wrapping_offset(i as isize) as c_int;' \
    'let ba: c_int = *bitalloc.wrapping_offset(i as isize) as c_int; dst = dst.wrapping_offset(choff as isize); choff = 18i32.wrapping_sub(choff);'
add KILL "dst base uses j instead of group_size*j" \
    'grbuf.wrapping_offset(group_size.wrapping_mul(j) as isize)' \
    'grbuf.wrapping_offset(j as isize)'
add KILL "dst write index off by one" \
    '*dst.wrapping_offset(k as isize) = v as f32;
                        k += 1;
                    }
                } else {' \
    '*dst.wrapping_offset(k as isize + 1) = v as f32;
                        k += 1;
                    }
                } else {'
add KILL "return value group_size*4 -> *2" \
    'group_size.wrapping_mul(4)' 'group_size.wrapping_mul(2)'
add KILL "return value saturates instead of wrapping" \
    'group_size.wrapping_mul(4)
}' 'group_size.saturating_mul(4)
}'
add KILL "total_bands not doubled" \
    'while i < 2i32.wrapping_mul((*sci).total_bands as c_int) {' \
    'while i < (*sci).total_bands as c_int {'
add KILL "total_bands read as i8" \
    '(*sci).total_bands as c_int' '((*sci).total_bands as i8) as c_int'
add KILL "band count saturates at 128" \
    'while i < 2i32.wrapping_mul((*sci).total_bands as c_int) {' \
    'while i < 2i32.wrapping_mul((*sci).total_bands as c_int).min(128) {'
add KILL "bitalloc read shifted by one" \
    '*bitalloc.wrapping_offset(i as isize) as c_int' \
    '*bitalloc.wrapping_offset(i as isize + 1) as c_int'
add KILL "bitalloc index clamped into the array (safe-Rust style bug)" \
    '*bitalloc.wrapping_offset(i as isize) as c_int' \
    '(*sci).bitalloc[(i as usize) & 63] as c_int'
add KILL "bitalloc index wraps at 64" \
    '*bitalloc.wrapping_offset(i as isize) as c_int' \
    '*bitalloc.wrapping_offset((i % 64) as isize) as c_int'

# ------------------------------------------------------- equivalent mutants ---
# PROOF: 2*total_bands is always even, so after a full granule the +576/-558
# toggle has been applied an even number of times and `choff` is necessarily
# back at 576.  Re-initialising it per granule therefore cannot be observed.
add EQUIV "choff re-initialised at the top of each granule" \
    'let mut dst: *mut f32 = grbuf.wrapping_offset' \
    'choff = 576; let mut dst: *mut f32 = grbuf.wrapping_offset'
# PROOF (asserted by e9_shift_count_always_in_range): every n reachable from
# dequantize_granule satisfies 1 <= n <= 0x70000003 and s <= 7, so n + s never
# overflows i32 and saturating_add == wrapping_add.
add EQUIV "get_bits n+s uses saturating_add" \
    'let mut shl: c_int = (n as u32).wrapping_add(s) as c_int;' \
    'let mut shl: c_int = (n as c_int).saturating_add(s as c_int);'
# PROOF: same premise -- shl never overflows, so checked/wrapping agree.
add EQUIV "shl decrement uses plain subtraction" \
    'shl = shl.wrapping_sub(8);' 'shl -= 8;'

kill_survived=0
equiv_died=0
skipped=0
total=${#NAMES[@]}
for idx in $(seq 0 $((total - 1))); do
  restore
  python3 - "$SRC" "${FROM[$idx]}" "${TO[$idx]}" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if frm not in s:
    sys.exit(7)
open(path, 'w').write(s.replace(frm, to, 1))
PY
  if [ $? -eq 7 ]; then
    echo "SKIP     (pattern not found)      : ${NAMES[$idx]}"
    skipped=$((skipped + 1)); continue
  fi
  if ! cargo build --release --offline >/dev/null 2>&1; then
    echo "SKIP     (mutant does not compile): ${NAMES[$idx]}"
    skipped=$((skipped + 1)); continue
  fi
  if timeout 300 cargo test --offline --release >/dev/null 2>&1; then
    if [ "${KIND[$idx]}" = "EQUIV" ]; then
      echo "survived (expected, EQUIV)       : ${NAMES[$idx]}"
    else
      echo "SURVIVED <-- BLIND SPOT          : ${NAMES[$idx]}"
      kill_survived=$((kill_survived + 1))
    fi
  else
    if [ "${KIND[$idx]}" = "EQUIV" ]; then
      echo "KILLED   <-- equivalence proof is WRONG: ${NAMES[$idx]}"
      equiv_died=$((equiv_died + 1))
    else
      echo "killed                           : ${NAMES[$idx]}"
    fi
  fi
done
restore
cargo build --release --offline >/dev/null 2>&1
echo "-----"
echo "mutants=$total skipped=$skipped  KILL-survivors=$kill_survived  EQUIV-killed=$equiv_died"
[ $kill_survived -eq 0 ] && [ $equiv_died -eq 0 ]
