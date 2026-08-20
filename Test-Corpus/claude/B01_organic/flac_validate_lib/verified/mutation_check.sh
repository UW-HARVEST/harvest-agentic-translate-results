#!/usr/bin/env bash
# Negative control for the differential harness.
#
# Injects a series of deliberate bugs into src/lib.rs, one at a time, and
# asserts that the test suite FAILS for each one. If a mutant survives, the
# harness has a blind spot for that behaviour.
#
# Usage: ./mutation_check.sh
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK="${TMPDIR:-/tmp}/lib.rs.orig"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

ITERS="${HARVEST_ITERS:-400}"

declare -a NAMES=() FROM=() TO=()
add() { NAMES+=("$1"); FROM+=("$2"); TO+=("$3"); }

add "blocksize lower bound off-by-one"   't.blocksize < 16'                     't.blocksize < 15'
add "blocksize upper bound off-by-one"   't.blocksize > 65535'                  't.blocksize > 65536'
add "samplerate zero check dropped"      'if t.samplerate == 0 {'              'if t.samplerate == u32::MAX {'
add "samplerate upper bound off-by-one"  't.samplerate > 655350'                't.samplerate > 655351'
add "channels upper bound off-by-one"    't.channels > 8'                       't.channels > 9'
add "bitdepth upper bound off-by-one"    't.bitdepth > 32'                      't.bitdepth > 33'
add "channel_mode compare inverted"      't.channel_mode != TFLAC_CHANNEL_INDEPENDENT &&' 't.channel_mode == TFLAC_CHANNEL_INDEPENDENT &&'
add "channels!=2 becomes channels<2"     't.channels != 2 || t.bitdepth == 32'  't.channels < 2 || t.bitdepth == 32'
add "bitdepth==32 arm dropped"           't.channels != 2 || t.bitdepth == 32'  't.channels != 2'
add "forced mode value wrong"            't.channel_mode = TFLAC_CHANNEL_INDEPENDENT;' 't.channel_mode = 1;'
add "rice autofill boundary off-by-one"  'if t.bitdepth <= 16 {'                'if t.bitdepth < 16 {'
add "rice autofill low value wrong"      't.max_rice_value = 14;'               't.max_rice_value = 15;'
add "rice autofill high value wrong"     't.max_rice_value = 30;'               't.max_rice_value = 29;'
add "rice upper bound off-by-one"        't.max_rice_value > 30'                't.max_rice_value > 31'
add "max_partition_order bound off"      't.max_partition_order > 15'           't.max_partition_order > 16'
add "min>max becomes min>=max"           't.min_partition_order > t.max_partition_order' 't.min_partition_order >= t.max_partition_order'
add "partition_order seeded from max"    't.partition_order = t.min_partition_order;' 't.partition_order = t.max_partition_order;'
add "loop shift amount off-by-one"       'u32::from(t.partition_order).wrapping_add(1)' 'u32::from(t.partition_order)'
add "loop cap becomes inclusive"         't.partition_order < t.max_partition_order' 't.partition_order <= t.max_partition_order'
add "loop condition operands swapped"    't.blocksize % divisor == 0'           't.blocksize % divisor != 0'
add "cur_blocksize not written"          't.cur_blocksize = t.blocksize;'       't.cur_blocksize = t.blocksize.wrapping_add(1);'
add "size_memory mask wrong"             '0xFFFF_FFF0u32'                       '0xFFFF_FFFFu32'
add "size_memory constant wrong"         '15u32.wrapping_add('                  '16u32.wrapping_add('
add "size_memory multiplier wrong"       '5u32.wrapping_mul('                   '6u32.wrapping_mul('
add "size_memory scale wrong"            'blocksize.wrapping_mul(4)'            'blocksize.wrapping_mul(5)'
add "size_memory saturates not wraps"    'blocksize.wrapping_mul(4)'            'blocksize.saturating_mul(4)'
add "flac_validate export removed"       '#[unsafe(no_mangle)]
pub unsafe extern "C" fn flac_validate' 'pub unsafe extern "C" fn flac_validate'
add "size_memory export removed"         '#[unsafe(no_mangle)]
pub extern "C" fn tflac_size_memory'     'pub extern "C" fn tflac_size_memory'

survived=0
total=${#NAMES[@]}
printf '%s\n' "Running $total mutants with HARVEST_ITERS=$ITERS"
for i in "${!NAMES[@]}"; do
    restore
    if ! python3 - "$SRC" "${FROM[$i]}" "${TO[$i]}" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if frm not in s:
    sys.exit(1)
open(path, 'w').write(s.replace(frm, to, 1))
PY
    then
        printf '  !! %-40s COULD NOT APPLY (pattern not found)\n' "${NAMES[$i]}"
        survived=$((survived + 1))
        continue
    fi

    if ! HARVEST_ITERS="$ITERS" cargo build --release >/dev/null 2>&1 \
       || ! HARVEST_ITERS="$ITERS" cargo build >/dev/null 2>&1; then
        printf '  ok %-40s (rejected at compile time)\n' "${NAMES[$i]}"
        continue
    fi
    if HARVEST_ITERS="$ITERS" timeout 600 cargo test -q >/dev/null 2>&1; then
        printf '  !! %-40s SURVIVED — harness blind spot\n' "${NAMES[$i]}"
        survived=$((survived + 1))
    else
        printf '  ok %-40s detected\n' "${NAMES[$i]}"
    fi
done

restore
cargo build --release >/dev/null 2>&1
cargo build >/dev/null 2>&1
echo
if [ "$survived" -eq 0 ]; then
    echo "ALL $total MUTANTS DETECTED"
else
    echo "$survived / $total MUTANTS SURVIVED"
    exit 1
fi
