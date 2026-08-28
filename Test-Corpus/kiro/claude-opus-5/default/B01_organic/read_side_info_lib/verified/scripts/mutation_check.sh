#!/usr/bin/env bash
# Sanity check: the differential suite must FAIL for each deliberate mutation of
# the Rust source. Restores the original file afterwards.
set -uo pipefail
crate="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src="$crate/src/lib.rs"
cp "$src" /tmp/lib.rs.orig
trap 'cp /tmp/lib.rs.orig "$src"' EXIT

mutate() { # <description> <sed expression>
  cp /tmp/lib.rs.orig "$src"
  perl -0pi -e "$2" "$src"
  if cmp -s /tmp/lib.rs.orig "$src"; then echo "NO-OP MUTATION: $1"; return 1; fi
  cd "$crate"
  cargo build --release >/dev/null 2>&1 || { echo "BUILD FAIL: $1"; return 1; }
  if timeout 600 cargo test --release >/tmp/mut.log 2>&1; then
    echo "SURVIVED (bad): $1"; return 1
  else
    echo "caught: $1"; return 0
  fi
}

fails=0
mutate "get_bits: check limit before advancing pos" \
  's/\(\*bs\)\.pos \+= n;\n        if \(\*bs\)\.pos > \(\*bs\)\.limit \{/if (*bs).pos + n > (*bs).limit { (*bs).pos += n;/' || fails=1
mutate "get_bits: mask 255 -> 127" 's/255u32 >> s/127u32 >> s/' || fails=1
mutate "preflag threshold 500 -> 501" 's/>= 500/>= 501/' || fails=1
mutate "scfsi mask 0x0F0F -> 0x0F0E" 's/0x0F0F/0x0F0E/' || fails=1
mutate "big_values bound 288 -> 289" 's/> 288/> 289/' || fails=1
mutate "n_short_sfb 39 -> 38" 's/n_short_sfb = 39/n_short_sfb = 38/' || fails=1
mutate "mixed n_long_sfb 8/6 swapped" 's/\{ 8 \} else \{ 6 \}/{ 6 } else { 8 }/' || fails=1
mutate "table_select shift 10 -> 9" 's/tables >> 10/tables >> 9/' || fails=1
mutate "region_count\[1\] widths 4\/3 swapped" \
  's/region_count\[0\] = get_bits\(bs, 4\)/region_count[0] = get_bits(bs, 3)/' || fails=1
mutate "final overflow check > -> >=" \
  's/if part_23_sum \+ \(\*bs\)\.pos > /if part_23_sum + (*bs).pos >= /' || fails=1
mutate "short table used for mixed blocks" 's/= scf_mixed_row/= scf_short_row/' || fails=1
mutate "g_scf_long row 5 value 158 -> 156" 's/76, 158, 0/76, 156, 0/' || fails=1
mutate "g_scf_mixed row 1 first value 12 -> 6" 's/        12, 12, 12, 4, 4, 4, 8/        6, 12, 12, 4, 4, 4, 8/' || fails=1
mutate "sr_idx decrement dropped" 's/sr_idx -= \(sr_idx != 0\) as c_int;//' || fails=1
mutate "scfsi field shift 12 -> 8" 's/\(scfsi >> 12\)/(scfsi >> 8)/' || fails=1
mutate "gr_count doubling dropped" 's/gr_count \*= 2;//' || fails=1
mutate "scalefac_compress width 4\/9 swapped" 's/\{ 4 \} else \{ 9 \}/{ 9 } else { 4 }/' || fails=1

cp /tmp/lib.rs.orig "$src"
cd "$crate" && cargo build --release >/dev/null 2>&1
if (( fails )); then echo "MUTATION SANITY CHECK FAILED"; exit 1; fi
echo "all mutations detected"
