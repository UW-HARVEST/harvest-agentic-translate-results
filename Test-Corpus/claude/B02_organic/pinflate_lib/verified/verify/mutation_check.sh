#!/usr/bin/env bash
# Mutation check: the differential suite is only meaningful if it FAILS when the
# Rust translation is wrong. Each mutation below injects one small divergence
# into src/lib.rs, runs the suite, and requires it to fail.
set -u
cd "$(dirname "$0")/.." || exit 1
ulimit -c 0

SRC=src/lib.rs
BAK=$(mktemp "${TMPDIR:-/tmp}/lib.rs.bakXXXXXX")
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

pass=0
fail=0

mutate() {  # name  from  to
    local name="$1" from="$2" to="$3"
    restore
    python3 - "$SRC" "$from" "$to" <<'PY'
import sys
p, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
if a not in s:
    sys.stderr.write("PATTERN NOT FOUND: %r\n" % a)
    sys.exit(2)
open(p, "w").write(s.replace(a, b, 1))
PY
    if [ $? -ne 0 ]; then
        echo "SKIP  $name (pattern not found)"
        return
    fi
    rm -rf target/cdylib_build
    out=$(timeout 900 cargo test --offline 2>&1)
    if echo "$out" | grep -qE "test result: FAILED|error\[E|error: could not compile"; then
        echo "GOOD  $name -> suite failed as required"
        echo "$out" | grep -m2 -E "^test .* FAILED|diverged|^error" | sed 's/^/        /'
        pass=$((pass + 1))
    else
        echo "BAD   $name -> suite still PASSED (blind spot!)"
        fail=$((fail + 1))
    fi
}

echo "=== mutation check: each row must make the suite fail ==="

mutate "cp_rev16 wrong mask" \
  'a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);' \
  'a = ((a & 0xAAAA) >> 1) | ((a & 0x5551) << 1);'

mutate "E2 comparison flipped (<= becomes <)" \
  'if !((*s).bits_left / 8 <= len as c_int) {' \
  'if !((*s).bits_left / 8 < len as c_int) {'

mutate "E1 LEN/NLEN check dropped" \
  'if !(len == !nlen) {' \
  'if false && !(len == !nlen) {'

mutate "cp_ptr assert removed" \
  '        ((*s).bits_left & 7) == 0,' \
  '        true || ((*s).bits_left & 7) == 0,'

mutate "cp_decode prefix assert removed" \
  '        (search >> (len & 31)) == (key >> (len & 31)),' \
  '        true || (search >> (len & 31)) == (key >> (len & 31)),'

mutate "cp_build len<16 assert removed" \
  'cp_assert!(len < 16, "len < 16", 154, "cp_build");' \
  'cp_assert!(len < 256, "len < 16", 154, "cp_build");'

mutate "cp_read_bits bits_left assert weakened to >= 0" \
  'cp_assert!((*s).bits_left > 0, "s->bits_left > 0", 125, "cp_read_bits");' \
  'cp_assert!((*s).bits_left >= 0, "s->bits_left > 0", 125, "cp_read_bits");'

mutate "cp_read_bits bits_left assert removed" \
  'cp_assert!((*s).bits_left > 0, "s->bits_left > 0", 125, "cp_read_bits");' \
  'cp_assert!(true, "s->bits_left > 0", 125, "cp_read_bits");'

mutate "cp_dynamic frame: lens offset shifted" \
  'const FR_LENS: usize = FR - 0x180;' \
  'const FR_LENS: usize = FR - 0x184;'

mutate "cp_dynamic frame: n modelled as a plain local" \
  'const FR_N: usize = FR - 0x8;' \
  'const FR_N: usize = 0x300;'

mutate "cp_dynamic: lens[-1] reads 1 instead of the s pointer MSB" \
  'fn fr_lens_get(f: &[u8; FRAME_CAP], k: c_int) -> u8 {
    let off = (FR_LENS as isize).wrapping_add(k as isize);
    if off >= 0 && (off as usize) < FRAME_CAP {
        f[off as usize]
    } else {
        0
    }
}' \
  'fn fr_lens_get(f: &[u8; FRAME_CAP], k: c_int) -> u8 {
    let off = (FR_LENS as isize).wrapping_add(k as isize);
    if k < 0 { return 1; }
    if off >= 0 && (off as usize) < FRAME_CAP {
        f[off as usize]
    } else {
        0
    }
}'

# NB: for `backwards_distance == 1` the `memset` arm and the byte-copy loop are
# semantically identical (src == dst - 1, so the loop propagates the same byte),
# so *disabling* the memset arm is not observable and is not a useful mutation.
# Taking the memset arm for the wrong distance, however, is.
mutate "cp_block memset arm taken for distance 2" \
  'if backwards_distance == 1 {' \
  'if backwards_distance == 1 || backwards_distance == 2 {'

mutate "cp_stored copies from the wrong offset" \
  'ptr::copy_nonoverlapping(p, (*s).out, len as usize);' \
  'ptr::copy_nonoverlapping(p.wrapping_offset(1), (*s).out, len as usize);'

mutate "cp_len_base entry changed" \
  '3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,' \
  '3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 257, 0, 0,'

mutate "pinflate first_bytes alignment mask changed" \
  'let first_bytes: c_int = ((in_addr.wrapping_add(3) & !3usize).wrapping_sub(in_addr)) as c_int;' \
  'let first_bytes: c_int = ((in_addr.wrapping_add(7) & !7usize).wrapping_sub(in_addr)) as c_int;'

mutate "cp_error_reason cleared on entry" \
  '    let layout = Layout::new::<cp_state_t>();' \
  '    cp_error_reason = ptr::null();
    let layout = Layout::new::<cp_state_t>();'

mutate "pinflate final-word branch dropped" \
  '} else if (*s).final_word_available != 0 {' \
  '} else if false && (*s).final_word_available != 0 {'

restore
rm -rf target/cdylib_build
echo
echo "=== mutations detected: $pass   blind spots: $fail ==="
[ "$fail" -eq 0 ]
