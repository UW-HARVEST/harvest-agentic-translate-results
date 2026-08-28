#!/usr/bin/env bash
# Meta-verification: prove the differential suite actually has bug-detecting
# power. Each mutation injects a realistic translation bug into the Rust source;
# the suite MUST fail. A mutation that survives means the suite has a blind spot.
#
# Usage: ./mutation_check.sh
set -u

cd "$(dirname "$0")" || exit 1
BK="$(mktemp -d)"
cp src/lib.rs "$BK/lib.rs"
cp src/tables.rs "$BK/tables.rs"

restore() { cp "$BK/lib.rs" src/lib.rs; cp "$BK/tables.rs" src/tables.rs; }
trap 'restore; rm -rf "$BK"' EXIT

pass=0; survived=0

# apply <desc> <file> <python-replace-expr> [test-filter]
mutate() {
  local desc="$1" file="$2" old="$3" new="$4" filter="${5:-}"
  restore
  # Mutates CODE only: comment lines are stripped from consideration, so a
  # pattern that happens to also appear in a `//` comment cannot produce a
  # no-op "mutation" that then falsely looks like a surviving mutant.
  python3 - "$file" "$old" "$new" <<'PY' || { echo "  SKIP (pattern not in code): $desc"; return; }
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
lines = open(p).read().split("\n")

def is_comment(l):
    return l.lstrip().startswith("//")

# Rebuild the file with comment lines masked out, find the match there, then
# apply the edit at the same offset in the real text.
masked = "\n".join("\x00" * len(l) if is_comment(l) else l for l in lines)
idx = masked.find(old)
if idx < 0:
    sys.exit(1)
text = "\n".join(lines)
assert text[idx:idx + len(old)] == old
open(p, "w").write(text[:idx] + new + text[idx + len(old):])
PY

  if ! cargo build --release --offline >/dev/null 2>&1; then
    echo "  CAUGHT (compile error): $desc"; pass=$((pass+1)); return
  fi
  # shellcheck disable=SC2086
  if timeout 600 cargo test --offline --tests ${filter:+-- $filter} >/dev/null 2>&1; then
    echo "  *** SURVIVED ***: $desc"; survived=$((survived+1))
  else
    echo "  CAUGHT: $desc"; pass=$((pass+1))
  fi
}

echo "=== Mutation testing the differential suite ==="

mutate "wide lane/table mixup: T[5][c[2]] -> T[4][c[2]]" src/lib.rs \
  '^ T[5][c[2] as usize]' '^ T[4][c[2] as usize]'

mutate "wide lane swap: c[6]/c[7] tables exchanged" src/lib.rs \
  '^ T[1][c[6] as usize]
            ^ T[0][c[7] as usize]' '^ T[0][c[6] as usize]
            ^ T[1][c[7] as usize]'

mutate "seed fold byte order: d[0]<<8|d[1] -> d[1]<<8|d[0]" src/lib.rs \
  'crc ^= ((c[0] as u16) << 8) | (c[1] as u16);' \
  'crc ^= ((c[1] as u16) << 8) | (c[0] as u16);'

# NOTE: `while len >= 8` -> `while len > 8` is deliberately NOT tested here: it is
# a proven EQUIVALENT MUTANT. The slice-by-8 wide step computes exactly the same
# CRC as 8 consecutive byte-at-a-time steps (that identity is the whole point of
# slice-by-8, and it is verified against the C .so itself by
# cfg15_stream_split_at_every_offset). So every threshold >= 8 yields identical
# output and no test can distinguish them. The genuinely non-equivalent
# off-by-ones are the block STEP and a threshold below 8, tested below.

mutate "threshold below block size: while len >= 8 -> while len >= 7" src/lib.rs \
  'while len >= 8 {' 'while len >= 7 {'

mutate "block step: len -= 8 -> len -= 4" src/lib.rs \
  'len -= 8;' 'len -= 4;'

mutate "cursor step: pos += 8 -> pos += 4" src/lib.rs \
  'pos += 8;' 'pos += 4;'

mutate "seed fold drops c[1]" src/lib.rs \
  'crc ^= ((c[0] as u16) << 8) | (c[1] as u16);' \
  'crc ^= (c[0] as u16) << 8;'

mutate "low-byte mask: crc & 0xFF -> crc & 0xFE" src/lib.rs \
  '^ T[6][(crc & 0xFF) as usize]' '^ T[6][(crc & 0xFE) as usize]'

mutate "high-byte shift: crc >> 8 -> crc >> 9 (wide step)" src/lib.rs \
  'crc = T[7][(crc >> 8) as usize]' 'crc = T[7][(crc >> 9) as usize]'

mutate "seed index swap: T[7][crc>>8]/T[6][crc&0xFF] exchanged" src/lib.rs \
  'crc = T[7][(crc >> 8) as usize]
            ^ T[6][(crc & 0xFF) as usize]' \
  'crc = T[6][(crc >> 8) as usize]
            ^ T[7][(crc & 0xFF) as usize]'

mutate "tail table: T[0] -> T[1]" src/lib.rs \
  'crc = (crc << 8) ^ T[0][idx as usize];' 'crc = (crc << 8) ^ T[1][idx as usize];'

mutate "tail index: (crc>>8)^byte -> (crc&0xFF)^byte" src/lib.rs \
  'let idx = ((crc >> 8) as u8) ^ d[pos];' 'let idx = ((crc & 0xFF) as u8) ^ d[pos];'

mutate "tail shift: crc<<8 -> crc<<4" src/lib.rs \
  'crc = (crc << 8) ^ T[0][idx as usize];' 'crc = (crc << 4) ^ T[0][idx as usize];'

mutate "zero-length short-circuit moved to len==1" src/lib.rs \
  'if len == 0 {
        return crc16;
    }' 'if len == 1 {
        return crc16;
    }'

mutate "single table entry corrupted (T[3][200])" src/tables.rs \
  '0x0011,' '0x0012,'

mutate "len narrowing: len as usize -> len as u16 as usize" src/lib.rs \
  'core::slice::from_raw_parts(d, len as usize)' \
  'core::slice::from_raw_parts(d, len as u16 as usize)'

mutate "sign extension: len as usize -> len as i32 as usize" src/lib.rs \
  'core::slice::from_raw_parts(d, len as usize)' \
  'core::slice::from_raw_parts(d, len as i32 as usize)' \
  'err_len_with_sign_bit_set_no_sign_extension'

restore
cargo build --release --offline >/dev/null 2>&1

echo
echo "=== caught: $pass   survived: $survived ==="
[ "$survived" -eq 0 ] || { echo "SUITE HAS BLIND SPOTS"; exit 1; }
echo "All mutations detected: the suite has real bug-detecting power."
