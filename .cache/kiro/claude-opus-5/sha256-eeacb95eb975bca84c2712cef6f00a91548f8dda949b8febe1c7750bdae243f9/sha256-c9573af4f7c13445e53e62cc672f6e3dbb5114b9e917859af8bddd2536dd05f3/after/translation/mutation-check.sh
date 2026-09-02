#!/usr/bin/env bash
# mutation-check.sh — proves the differential suite is not vacuous.
#
# Applies a series of small deliberate defects to translation/src/lib.rs, one at
# a time, and asserts that the suite FAILS for each. A mutation that survives
# means the corresponding behaviour is untested.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
SRC="$CRATE/src/lib.rs"
BAK="$(mktemp)"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | tail -1)"
survivors=0

# name@@from@@to
MUTANTS=(
  'rice-autofill-boundary@@get!(bitdepth) <= 16@@get!(bitdepth) < 16'
  'rice-autofill-value@@set!(max_rice_value, 14)@@set!(max_rice_value, 15)'
  'rice-upper-bound@@get!(max_rice_value) > 30@@get!(max_rice_value) > 31'
  'blocksize-lower-bound@@get!(blocksize) < 16@@get!(blocksize) < 17'
  'blocksize-upper-bound@@get!(blocksize) > 65535@@get!(blocksize) > 65536'
  'samplerate-upper-bound@@get!(samplerate) > 655350@@get!(samplerate) > 655351'
  'channels-upper-bound@@get!(channels) > 8@@get!(channels) > 9'
  'bitdepth-upper-bound@@get!(bitdepth) > 32@@get!(bitdepth) > 33'
  'max-po-upper-bound@@get!(max_partition_order) > 15@@get!(max_partition_order) > 16'
  'min-max-po-order@@get!(min_partition_order) > get!(max_partition_order)@@get!(min_partition_order) >= get!(max_partition_order)'
  'stereo-mode-channels@@get!(channels) != 2@@get!(channels) != 1'
  'stereo-mode-bitdepth@@get!(bitdepth) == 32@@get!(bitdepth) == 31'
  'stereo-mode-and-or@@!= 2 || get!(bitdepth) == 32@@!= 2 && get!(bitdepth) == 32'
  'partition-loop-shift@@as u32 + 1))@@as u32 + 2))'
  'partition-loop-strict@@get!(partition_order) < get!(max_partition_order)@@get!(partition_order) <= get!(max_partition_order)'
  'cur-blocksize-source@@set!(cur_blocksize, get!(blocksize))@@set!(cur_blocksize, get!(blocksize) + 1)'
  'size-memory-const15@@15u32.wrapping_add(\n        5u32@@16u32.wrapping_add(\n        5u32'
  'size-memory-mask@@0xFFFF_FFF0u32@@0xFFFF_FFE0u32'
  'size-memory-mul@@wrapping_mul(4)@@wrapping_mul(5)'
)

for m in "${MUTANTS[@]}"; do
  name="${m%%@@*}"; rest="${m#*@@}"; from="${rest%%@@*}"; to="${rest##*@@}"
  restore
  python3 - "$SRC" "$from" "$to" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2].replace('\\n','\n'), sys.argv[3].replace('\\n','\n')
s = open(path).read()
if frm not in s:
    print("MUTANT-NOT-APPLIED", file=sys.stderr); sys.exit(3)
open(path,'w').write(s.replace(frm, to, 1))
PY
  rc=$?
  if [ "$rc" -eq 3 ]; then
    printf '   \033[33mSKIP\033[0m %-26s (pattern not found)\n' "$name"; survivors=$((survivors+1)); continue
  fi

  if ! timeout 300 cargo build --manifest-path "$CRATE/Cargo.toml" >/dev/null 2>&1; then
    printf '   \033[33mSKIP\033[0m %-26s (mutant does not compile)\n' "$name"; continue
  fi
  if C_SO="$C_SO" RUST_SO="$CRATE/target/debug/libflac_validate_lib.so" \
     timeout 300 cargo test --manifest-path "$CRATE/Cargo.toml" \
       -- --test-threads=4 >/tmp/mut.log 2>&1; then
    printf '   \033[31mSURVIVED\033[0m %-22s <-- behaviour is NOT covered\n' "$name"
    survivors=$((survivors+1))
  else
    printf '   \033[32mCAUGHT\033[0m %-24s by: %s\n' "$name" \
      "$(grep -oE '^ *[a-z0-9_]+::[a-z0-9_]+|^    [a-z0-9_]+$' /tmp/mut.log | head -3 | tr -d ' ' | tr '\n' ' ')"
  fi
done

restore
timeout 300 cargo build --manifest-path "$CRATE/Cargo.toml" >/dev/null 2>&1
timeout 300 cargo build --release --manifest-path "$CRATE/Cargo.toml" >/dev/null 2>&1

echo
if [ "$survivors" -eq 0 ]; then
  printf '\033[32mall %d mutants caught — the suite is not vacuous\033[0m\n' "${#MUTANTS[@]}"
else
  printf '\033[31m%d mutant(s) survived\033[0m\n' "$survivors"
fi
exit "$survivors"
