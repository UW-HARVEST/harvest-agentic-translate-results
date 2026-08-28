#!/usr/bin/env bash
# Negative control: deliberately break the Rust translation in ways that mimic
# realistic mis-translations, and confirm the differential suite catches each
# one.  A surviving mutant means the test suite has a blind spot.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"
ORIG="$(mktemp)"
cp src/lib.rs "$ORIG"
trap 'cp "$ORIG" src/lib.rs; rm -f "$ORIG" ./mut.log ./mut-build.log' EXIT
case "${1:-release}" in
  dev|debug) PROFILE="" ;;
  *)         PROFILE="--release" ;;
esac
run_mutant() {
  name="$1"; shift
  cp "$ORIG" src/lib.rs
  "$@"
  if diff -q "$ORIG" src/lib.rs >/dev/null; then echo "MUTANT $name: SED DID NOT APPLY"; return 1; fi
  if ! cargo build $PROFILE >./mut-build.log 2>&1; then echo "MUTANT $name: build failed"; return 1; fi
  if timeout 300 cargo test $PROFILE >./mut.log 2>&1; then
    echo "MUTANT $name: *** SURVIVED (blind spot!) ***"; return 1
  fi
  why=$(grep -oE 'test result: FAILED[^;]*; [0-9]+ failed|signal: [0-9]+' ./mut.log | head -2 | tr '\n' ' ')
  echo "MUTANT $name: killed  [$why]"
}
# --- struct layout / pointer arithmetic ---
m_chunk_stride()  { sed -i 's/^const SIZEOF_CAF_CHUNK: usize = 16;/const SIZEOF_CAF_CHUNK: usize = 12;/' src/lib.rs; }
m_header_size()   { sed -i 's/^const SIZEOF_CAF_HEADER: usize = 8;/const SIZEOF_CAF_HEADER: usize = 12;/' src/lib.rs; }
m_cafdata_size()  { sed -i 's/^const SIZEOF_CAF_DATA: usize = 4;/const SIZEOF_CAF_DATA: usize = 8;/' src/lib.rs; }
m_ver_off()       { sed -i 's/^const CAF_HEADER_VERSION: usize = 4;/const CAF_HEADER_VERSION: usize = 6;/' src/lib.rs; }
m_type_off()      { sed -i 's/^const CAF_HEADER_TYPE: usize = 0;/const CAF_HEADER_TYPE: usize = 2;/' src/lib.rs; }
m_fmt_off()       { sed -i 's/^const CAF_DESC_FORMAT_ID: usize = 8;/const CAF_DESC_FORMAT_ID: usize = 12;/' src/lib.rs; }
m_chan_off()      { sed -i 's/^const CAF_DESC_CHANNELS_PER_FRAME: usize = 24;/const CAF_DESC_CHANNELS_PER_FRAME: usize = 28;/' src/lib.rs; }
m_sr_off()        { sed -i 's/^const CAF_DESC_SAMPLE_RATE: usize = 0;/const CAF_DESC_SAMPLE_RATE: usize = 8;/' src/lib.rs; }
m_fc_off()        { sed -i 's/^const CAF_PAKT_FRAME_COUNT: usize = 8;/const CAF_PAKT_FRAME_COUNT: usize = 0;/' src/lib.rs; }
m_blocks_base()   { sed -i 's/blocks = chunk/blocks = header/' src/lib.rs; }
# --- fourcc constants / byte order ---
m_desc_fourcc()   { sed -i "s/^const CAF_TYPE_DESC: ima_u32_t = fourcc(b'c', b's', b'e', b'd');/const CAF_TYPE_DESC: ima_u32_t = fourcc(b'd', b'e', b's', b'c');/" src/lib.rs; }
m_btoh32_id()     { sed -i "/^fn ima_btoh32(v: ima_u32_t) -> ima_u32_t {/{n;s/    ima_bswap32(v)/    v/;}" src/lib.rs; }
m_bswap16()       { sed -i 's/(v >> 0x08 \& 0x00ffu16)/(v >> 0x08 \& 0x00feu16)/' src/lib.rs; }
m_bswap64()       { sed -i 's/(v << 0x28 \& 0x00ff000000000000u64)/(v << 0x28 \& 0x00fe000000000000u64)/' src/lib.rs; }
m_no_size_swap()  { sed -i 's/chunk_size = ima_btoh64(load_u64(chunk, CAF_CHUNK_SIZE)) as ima_s64_t;/chunk_size = load_u64(chunk, CAF_CHUNK_SIZE) as ima_s64_t;/' src/lib.rs; }
m_no_final_swap() { sed -i 's/^    conv64_u = ima_btoh64(conv64_u);/    \/\/ swap removed/' src/lib.rs; }
# --- error codes / predicates ---
m_err_1_to_2()    { sed -i 's/^        return -1;/        return -2;/' src/lib.rs; }
m_err_2_to_3()    { sed -i '0,/        return -2;/s/        return -2;/        return -3;/' src/lib.rs; }
m_ver_pred()      { sed -i 's/if ima_btoh16(load_u16(header, CAF_HEADER_VERSION)) != 1 {/if ima_btoh16(load_u16(header, CAF_HEADER_VERSION)) != 0 {/' src/lib.rs; }
m_first_desc()    { sed -i 's/if chunk_type == CAF_TYPE_DESC {/if chunk_type == CAF_TYPE_DESC \&\& desc.is_null() {/' src/lib.rs; }
m_first_pakt()    { sed -i 's/} else if chunk_type == CAF_TYPE_PAKT {/} else if chunk_type == CAF_TYPE_PAKT \&\& pakt.is_null() {/' src/lib.rs; }
m_size_zero()     { sed -i 's/(\*info).size = chunk_size as ima_u64_t;/(*info).size = 0;/' src/lib.rs; }
m_fc_from_desc()  { sed -i 's/load_u64(pakt, CAF_PAKT_FRAME_COUNT)/load_u64(desc, CAF_PAKT_FRAME_COUNT)/' src/lib.rs; }
# --- the double -> u64 value conversion (lib.c:127) ---
m_sat_cast()      { sed -i 's/^    conv64_u = double_to_u64(load_f64(desc, CAF_DESC_SAMPLE_RATE));/    conv64_u = load_f64(desc, CAF_DESC_SAMPLE_RATE) as u64;/' src/lib.rs; }
m_bitcast()       { sed -i 's/^    conv64_u = double_to_u64(load_f64(desc, CAF_DESC_SAMPLE_RATE));/    conv64_u = load_f64(desc, CAF_DESC_SAMPLE_RATE).to_bits();/' src/lib.rs; }
m_from_bits()     { sed -i 's/(\*info).sample_rate = ima_f64_t::from_bits(conv64_u);/(*info).sample_rate = conv64_u as ima_f64_t;/' src/lib.rs; }
m_round()         { sed -i 's/let t = x.trunc();/let t = x.round();/' src/lib.rs; }
m_range_le()      { sed -i 's/t < 9223372036854775808.0 {/t <= 9223372036854775808.0 {/' src/lib.rs; }
m_indefinite_0()  { sed -i 's/return ima_s64_t::MIN;/return 0;/' src/lib.rs; }
m_bias_or()       { sed -i 's/(cvttsd2si64(x - TWO_POW_63) as ima_u64_t) ^ (1u64 << 63)/(cvttsd2si64(x - TWO_POW_63) as ima_u64_t) | (1u64 << 63)/' src/lib.rs; }
m_no_bias()       { sed -i 's/^    if x >= TWO_POW_63 {/    if false {/' src/lib.rs; }

MUTANTS=(m_chunk_stride m_header_size m_cafdata_size m_ver_off m_type_off m_fmt_off
         m_chan_off m_sr_off m_fc_off m_blocks_base m_desc_fourcc m_btoh32_id
         m_bswap16 m_bswap64 m_no_size_swap m_no_final_swap m_err_1_to_2
         m_err_2_to_3 m_ver_pred m_first_desc m_first_pakt m_size_zero
         m_fc_from_desc m_sat_cast m_bitcast m_from_bits m_round m_range_le
         m_indefinite_0 m_bias_or m_no_bias)

killed=0; survived=0
for m in "${MUTANTS[@]}"; do
  if run_mutant "$m" "$m"; then killed=$((killed+1)); else survived=$((survived+1)); fi
done
cp "$ORIG" src/lib.rs
rm -f ./mut.log ./mut-build.log
cargo build $PROFILE >/dev/null 2>&1
echo
echo "profile ${PROFILE:---dev}: $killed killed / $survived survived (out of ${#MUTANTS[@]})"
[[ $survived -eq 0 ]]
