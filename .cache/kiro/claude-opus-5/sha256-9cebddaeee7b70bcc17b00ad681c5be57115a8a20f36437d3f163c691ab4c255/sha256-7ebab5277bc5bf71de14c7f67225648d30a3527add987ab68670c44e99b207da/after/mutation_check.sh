#!/usr/bin/env bash
# Mutation-tests the harness: injects known bugs into translation/src/lib.rs and
# confirms the differential tests fail. Any mutation that still PASSES means the
# test suite has a blind spot.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$ROOT/translation/src/lib.rs"
BAK="$(mktemp)"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

BLIND=0

mutate() {  # mutate <name> <sed-expr>
  local name="$1"; shift
  restore
  for expr in "$@"; do
    perl -0777 -pi -e "$expr" "$SRC"
  done
  if ! diff -q "$BAK" "$SRC" >/dev/null; then
    if timeout 600 sh -c "cd '$ROOT/translation' && cargo test --release" >/tmp/mut.log 2>&1; then
      echo "BLIND SPOT: mutation '$name' was NOT detected"
      BLIND=1
    else
      echo "detected: $name"
    fi
  else
    echo "SKIP (no textual change): $name"
    BLIND=1
  fi
}

mutate "sample_rate: bit-cast instead of numeric conversion" \
  's/let converted = f64_to_u64\(raw\);/let converted = raw.to_bits();/'

mutate "sample_rate: saturating cast instead of cvttsd2si" \
  's/fn f64_to_u64\(v: f64\) -> ima_u64_t \{/fn f64_to_u64(v: f64) -> ima_u64_t { return v as ima_u64_t; #[allow(unreachable_code)] {/' \
  's/(\n\s*)\/\/ -+\n\/\/ ima_parse/}}\1\/\/ ima_parse/'

mutate "sample_rate: skip the final byte swap" \
  's/let swapped = ima_btoh64\(converted\);/let swapped = converted;/'

mutate "bswap32: drop one lane" \
  's/\| \(v >> 0x18 & 0x000000ffu32\)/| 0/'

mutate "bswap64: swap two lanes" \
  's/\(v >> 0x28 & 0x000000000000ff00u64\)/(v >> 0x28 \& 0x00000000000000ffu64)/'

mutate "bswap16: identity" \
  's/ima_bswap16\(v\)\n\}/v\n}/'

mutate "chunk stride: 12 bytes instead of 16" \
  's/chunk = \(chunk\.wrapping_add\(1\) as \*const ima_u8_t\)/chunk = ((chunk as *const ima_u8_t).wrapping_add(12) as *const ima_u8_t)/'

mutate "blocks: forget the caf_data offset" \
  's/blocks = \(chunk\.wrapping_add\(1\) as \*const caf_data\)\.wrapping_add\(1\)/blocks = (chunk.wrapping_add(1) as *const caf_data).wrapping_add(0)/'

mutate "return codes: -1 and -2 swapped" \
  's/return -1;/return -2;/' \
  's/return -2;\n        \}\n\n        loop/return -1;\n        }\n\n        loop/'

mutate "format_id check inverted fourcc" \
  "s/fourcc\(b'4', b'a', b'm', b'i'\)/fourcc(b'i', b'm', b'a', b'4')/"

mutate "file type fourcc wrong" \
  "s/fourcc\(b'f', b'f', b'a', b'c'\)/fourcc(b'c', b'a', b'f', b'f')/"

mutate "desc/pakt chunk tags swapped" \
  "s/const CAF_CHUNK_DESC: ima_u32_t = fourcc\(b'c', b's', b'e', b'd'\);/const CAF_CHUNK_DESC: ima_u32_t = fourcc(b't', b'k', b'a', b'p');/" \
  "s/const CAF_CHUNK_PAKT: ima_u32_t = fourcc\(b't', b'k', b'a', b'p'\);/const CAF_CHUNK_PAKT: ima_u32_t = fourcc(b'c', b's', b'e', b'd');/"

mutate "frame_count read from packet_count" \
  's/addr_of!\(\(\*pakt\)\.frame_count\)/addr_of!((*pakt).packet_count)/'

mutate "channel_count read from bits_per_channel" \
  's/addr_of!\(\(\*desc\)\.channels_per_frame\)/addr_of!((*desc).bits_per_channel)/'

mutate "info->size uses 0 instead of chunk_size" \
  's/ptr::write_unaligned\(addr_of_mut!\(\(\*info\)\.size\), chunk_size as ima_u64_t\);/ptr::write_unaligned(addr_of_mut!((*info).size), 0u64);/'

mutate "chunk size read from offset 4 instead of 8" \
  's/ptr::read_unaligned\(addr_of!\(\(\*chunk\)\.size\)\)/(ptr::read_unaligned((chunk as *const ima_u8_t).wrapping_add(4) as *const u32) as i64)/'

mutate "version check accepts anything" \
  's/if ima_btoh16\(ptr::read_unaligned\(addr_of!\(\(\*header\)\.version\)\)\) != 1 \{/if false {/'

restore
if (( BLIND )); then
  echo "RESULT: blind spots found"
  exit 1
fi
echo "RESULT: all mutations detected"
