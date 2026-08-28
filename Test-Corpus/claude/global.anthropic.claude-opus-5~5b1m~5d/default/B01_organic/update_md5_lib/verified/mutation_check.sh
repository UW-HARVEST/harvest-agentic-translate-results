#!/usr/bin/env bash
# Sensitivity check for the differential test-suite.
#
# Injects one deliberate bug at a time into src/lib.rs, rebuilds the cdylib and
# runs the full suite. EVERY mutation must make the suite FAIL — otherwise the
# tests are not actually observing that behaviour. src/lib.rs is always
# restored, including on interrupt/error.
#
# Mutations are applied only to real CODE lines (lines whose first non-space
# characters are `//` are skipped) and the pattern must match exactly one such
# line, so a mutation can never silently land in a doc comment.
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; cargo build -q >/dev/null 2>&1; rm -f "$BAK"; }
trap restore EXIT INT TERM

SEP='@@@'
MUTATIONS=(
  "loop runs 4 times instead of 5@@@while i <= 4 {@@@while i < 4 {"
  "sample stride 8 elements instead of 32@@@samples.wrapping_add(8 * core::mem::size_of::<tflac_s32>())@@@samples.wrapping_add(8)"
  "pos2 masked with 32 instead of 64@@@let pos2 = ld_u32(p_pos) % 64;@@@let pos2 = ld_u32(p_pos) % 32;"
  "bytes = bits / 4 instead of bits / 8@@@bytes = bits / 8;@@@bytes = bits / 4;"
  "spill test uses > 64 instead of >= 64@@@if pos >= 64 {@@@if pos > 64 {"
  "pos wrapped modulo 63 instead of 64@@@pos %= 64;@@@pos %= 63;"
  "spill source offset 65 instead of 64@@@64usize.wrapping_add(bytes as usize)@@@65usize.wrapping_add(bytes as usize)"
  "pack byte 7 shifted by 55 instead of 56@@@(n >> 56) as tflac_u8@@@(n >> 55) as tflac_u8"
  "pack byte 0 shifted by 8@@@st_u8(d.wrapping_add(0), n as tflac_u8);@@@st_u8(d.wrapping_add(0), (n >> 8) as tflac_u8);"
  "b decremented by 4 instead of step@@@b = b.wrapping_sub(step);@@@b = b.wrapping_sub(4);"
  "total accumulates bytes instead of bits@@@wrapping_add(bits as tflac_u64)@@@wrapping_add((bits / 8) as tflac_u64)"
  "pos re-read replaced by the stale pos2@@@let mut pos = ld_u32(p_pos).wrapping_add(bytes);@@@let mut pos = pos2.wrapping_add(bytes);"
  "sample mask 0xFFFF instead of 0xFF@@@as i64 as tflac_uint) & 0xFF@@@as i64 as tflac_uint) & 0xFFFF"
  "blocksize+channels instead of blocksize*channels@@@cur_blocksize.wrapping_mul(channels)@@@cur_blocksize.wrapping_add(channels)"
  "cur_blocksize and channels field offsets swapped@@@ld_u32(core::ptr::addr_of!((*t).cur_blocksize) as *const tflac_u8)@@@ld_u32(core::ptr::addr_of!((*t).channels) as *const tflac_u8)"
)

fail=0
i=0
for m in "${MUTATIONS[@]}"; do
  i=$((i + 1))
  desc=${m%%${SEP}*}
  rest=${m#*${SEP}}
  old=${rest%%${SEP}*}
  new=${rest#*${SEP}}

  cp "$BAK" "$SRC"
  if ! python3 - "$SRC" "$old" "$new" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
lines = open(path).read().split("\n")
hits = [n for n, l in enumerate(lines)
        if old in l and not l.strip().startswith("//")]
if len(hits) != 1:
    sys.stderr.write("pattern %r matched %d code lines\n" % (old, len(hits)))
    sys.exit(3)
lines[hits[0]] = lines[hits[0]].replace(old, new)
open(path, "w").write("\n".join(lines))
PY
  then
    echo "[$i] SKIP (pattern not unique in code): $desc"
    fail=1
    continue
  fi

  if ! cargo build -q >/dev/null 2>&1; then
    echo "[$i] SKIP (mutant does not compile): $desc"
    fail=1
    continue
  fi
  if cargo test -q >/dev/null 2>&1; then
    echo "[$i] NOT DETECTED (BAD): $desc"
    fail=1
  else
    echo "[$i] detected: $desc"
  fi
done

cp "$BAK" "$SRC"
cargo build -q >/dev/null 2>&1
if [ "$fail" -eq 0 ]; then
  echo "ALL ${#MUTATIONS[@]} MUTATIONS DETECTED"
else
  echo "SOME MUTATIONS WERE NOT DETECTED"
fi
exit "$fail"
