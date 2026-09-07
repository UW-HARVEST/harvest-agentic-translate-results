#!/usr/bin/env bash
# Mutation-tests the differential harness: each mutation is applied to the Rust
# source, the .so is rebuilt, and the test suite MUST fail. If a mutation
# survives, the harness has a blind spot.
set -u
cd "$(dirname "$0")/.." || exit 1
SRC=src/lib.rs
cp "$SRC" /tmp/lib.rs.orig
trap 'cp /tmp/lib.rs.orig "$SRC"; cargo build --release -q 2>/dev/null' EXIT

declare -a NAMES=() FROM=() TO=()
add() { NAMES+=("$1"); FROM+=("$2"); TO+=("$3"); }

add "g_scf_long row0 byte"      '6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0,' \
                                 '6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 55, 0,'
add "g_scf_short last row"      '26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0,
    ],
];

// The C initialisers' \
                                 '26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 13, 0,
    ],
];

// The C initialisers'
add "g_scf_mixed row1"          '12, 12, 12, 4, 4, 4, 8, 8, 8,' \
                                 '12, 12, 12, 4, 4, 5, 8, 8, 8,'
add "big_values threshold"      'if big_values > 288 {' 'if big_values >= 288 {'
add "block_type zero check"     'if block_type == 0 {' 'if block_type == 4 {'
add "scfsi mask"                'scfsi &= 0x0F0F;' 'scfsi &= 0x0F00;'
add "n_long_sfb mixed 6->7"     'if (hdr1 & 0x8) != 0 { 8 } else { 6 }' 'if (hdr1 & 0x8) != 0 { 8 } else { 7 }'
add "region_count[0] 8->7"      '(*gr).region_count[0] = 8;' '(*gr).region_count[0] = 7;'
add "preflag threshold 500"     'scalefac_compress >= 500' 'scalefac_compress > 500'
add "scalefac_compress width"   'if (hdr1 & 0x8) != 0 { 4 } else { 9 }' 'if (hdr1 & 0x8) != 0 { 4 } else { 8 }'
add "get_bits limit >= vs >"    'if (*bs).pos > (*bs).limit {' 'if (*bs).pos >= (*bs).limit {'
add "get_bits mask 255>>s"      '(255u32 >> s)' '(255u32 >> s) | 1'
add "get_bits loop step 8->7"   'shl = shl.wrapping_sub(8);' 'shl = shl.wrapping_sub(7);'
add "get_bits shl init n+s"     'let mut shl: c_int = n.wrapping_add(s as c_int);' 'let mut shl: c_int = n;'
add "get_bits final shr sign"   'next.wrapping_shr(shl.wrapping_neg() as u32)' 'next.wrapping_shr(shl as u32)'
add "get_bits cache or vs xor"  'cache |= next.wrapping_shl(shl as u32);' 'cache ^= next.wrapping_shl(shl as u32).wrapping_add(1);'
add "get_bits pos advance"      '(*bs).pos = (*bs).pos.wrapping_add(n);' '(*bs).pos = (*bs).pos.wrapping_add(n).wrapping_add(1);'
add "get_bits no pos advance"   '(*bs).pos = (*bs).pos.wrapping_add(n);' 'if (*bs).pos.wrapping_add(n) <= (*bs).limit { (*bs).pos = (*bs).pos.wrapping_add(n); } else { (*bs).pos = (*bs).limit + 1; }'
add "get_bits ptr base"         '.offset(((*bs).pos >> 3) as isize)' '.offset(((*bs).pos >> 3) as isize + 1)'
add "sfbtab long stride 23"     'offset(sr_idx as isize * 23)' 'offset(sr_idx as isize * 24)'
add "sfbtab short stride 40"    '(G_SCF_SHORT.as_ptr() as *const u8).offset(sr_idx as isize * 40)' '(G_SCF_SHORT.as_ptr() as *const u8).offset(sr_idx as isize * 41)'
add "sfbtab short vs mixed"     '(G_SCF_SHORT.as_ptr() as *const u8).offset(sr_idx as isize * 40)' '(G_SCF_MIXED.as_ptr() as *const u8).offset(sr_idx as isize * 40)'
add "n_long_sfb 22->21"         '(*gr).n_long_sfb = 22;' '(*gr).n_long_sfb = 21;'
add "mpeg1 flag 0x8 -> 0x10"    'if (hdr1 & 0x8) != 0 {
        gr_count = gr_count.wrapping_mul(2);' 'if (hdr1 & 0x10) != 0 {
        gr_count = gr_count.wrapping_mul(2);'
add "sr_idx hdr2 shift"         '((hdr2 >> 2) & 3)' '((hdr2 >> 1) & 3)'
add "sr_idx mult 3 -> 2"        '((hdr1 >> 4) & 1)) * 3' '((hdr1 >> 4) & 1)) * 2'
add "block_type==2 -> ==1"      'if block_type == 2 {' 'if block_type == 1 {'
add "mixed_block_flag polarity" 'if mixed_block_flag == 0 {' 'if mixed_block_flag != 0 {'
add "region_count[1] 255->0"    '(*gr).region_count[1] = 255;' '(*gr).region_count[1] = 0;'
add "scfsi field shift 12"      '((scfsi >> 12) & 15)' '((scfsi >> 8) & 15)'
add "granule advance +1 -> +2"  'gr.add(1)' 'gr.add(2)'
add "granule advance removed"    'gr = unsafe { gr.add(1) };' ''
add "region_count[0]=7 -> 6"    '(*gr).region_count[0] = 7;' '(*gr).region_count[0] = 6;'
add "main_data_begin shift"     '>> gr_count) as c_int' '>> (gr_count + 1)) as c_int'
add "scfsi read width"          'get_bits(bs, 7 + gr_count)' 'get_bits(bs, 6 + gr_count)'
add "final bounds > vs >="      'if part_23_sum.wrapping_add(pos) > limit' 'if part_23_sum.wrapping_add(pos) >= limit'
add "final bounds mul 8 vs 4"   'main_data_begin.wrapping_mul(8)' 'main_data_begin.wrapping_mul(4)'
add "final bounds drop check"   'if part_23_sum.wrapping_add(pos) > limit' 'if false && part_23_sum.wrapping_add(pos) > limit'
add "sr_idx decrement"          'sr_idx -= (sr_idx != 0) as c_int;' 'sr_idx -= 0;'
add "mono scfsi extra shift"    'if (hdr3 & 0xC0) == 0xC0 {
            scfsi = scfsi.wrapping_shl(4);' 'if (hdr3 & 0xC0) == 0xC0 {
            scfsi = scfsi.wrapping_shl(0);'
add "table_select[1] mask"      '((tables >> 5) & 31)' '((tables >> 5) & 15)'
add "tables shift 10 vs 15"     'tables = unsafe { get_bits(bs, 10) };' 'tables = unsafe { get_bits(bs, 11) };'
add "gr_count mono test"        'if (hdr3 & 0xC0) == 0xC0 { 1 } else { 2 }' 'if (hdr3 & 0xC0) == 0x80 { 1 } else { 2 }'
add "n_short_sfb 39->38"        '(*gr).n_short_sfb = 39;' '(*gr).n_short_sfb = 38;'
add "subblock_gain width"       '(*gr).subblock_gain[0] = get_bits(bs, 3) as u8;' '(*gr).subblock_gain[0] = get_bits(bs, 2) as u8;'
add "part_23_length width"      'get_bits(bs, 12) } as u16' 'get_bits(bs, 11) } as u16'
add "region_count[2] sentinel"  '(*gr).region_count[2] = 255;' '(*gr).region_count[2] = 254;'

survived=0; killed=0
for i in "${!NAMES[@]}"; do
  cp /tmp/lib.rs.orig "$SRC"
  python3 - "$SRC" "${FROM[$i]}" "${TO[$i]}" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if frm not in s:
    print("PATTERN-NOT-FOUND"); sys.exit(2)
open(path, "w").write(s.replace(frm, to, 1))
PY
  if [ $? -ne 0 ]; then
    printf '%-32s PATTERN NOT FOUND (mutation not applied)\n' "${NAMES[$i]}"
    survived=$((survived+1)); continue
  fi
  if ! cargo build --release -q 2>/dev/null; then
    printf '%-32s DID NOT COMPILE (skipped)\n' "${NAMES[$i]}"
    continue
  fi
  out=$(FUZZ_N=20000 HDR_STRIDE=64 timeout 600 cargo test --release -q -- --skip err_e11_null_pointers --skip err_e7_pos_overflow_faults 2>&1)
  if echo "$out" | grep -q 'FAILED\|panicked'; then
    n=$(echo "$out" | grep -oP '\d+(?= failed)' | paste -sd+ | bc 2>/dev/null || echo '?')
    printf '%-32s KILLED   (%s failing tests)\n' "${NAMES[$i]}" "$n"
    killed=$((killed+1))
  else
    printf '%-32s *** SURVIVED ***\n' "${NAMES[$i]}"
    survived=$((survived+1))
  fi
done
echo "-----------------------------------------"
echo "killed=$killed survived=$survived"
[ "$survived" -eq 0 ]
